// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::fmt::Write as _;
#[cfg(test)]
mod tests;

pub(crate) const COVERAGE: &str = "mmaps/available-navigation-v52.tsv";
const COVERAGE_MAX: usize = 128 * 1024 * 1024;
const MAPS_MAX: usize = 4096;
const TILES_MAX: usize = 4096 * 4096;

struct Source {
    summary: Summary,
    artifacts: Vec<Artifact>,
    geometry: collision_geometry_v22::Coverage,
    records: AvailableRecords<terrain_sources::NavigationMapFields>,
    liquids: models::liquid::Table,
    coverage_sha: [u8; 32],
}

fn source(root: &Path, plan: &PlanGuard, product: &Plan) -> Result<Source, Error> {
    if !plan.plan.is_v51() || plan.plan.locale != product.locale {
        return Err("product_navigation_input_plan".into());
    }
    let summary = crate::verify::verify_links(&plan.plan, root, None)?;
    if summary.context.plan_sha256 != plan.sha256 {
        return Err("product_navigation_input_context".into());
    }
    let mut artifacts = Vec::new();
    let mut tables = BTreeMap::new();
    let geometry_path = collision_geometry_v22::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH;
    let map_path = format!("dbc/{}/Map.db2", product.locale.name());
    let liquid_path = format!("dbc/{}/LiquidType.db2", product.locale.name());
    let limits = source_limits(&plan.plan)?;
    let checked = package::verify_with_preflight(
        root,
        Some(summary.context),
        limits,
        |entries| {
            if entries.len() > MAX_INPUT_FILES {
                return Err(package::Error::Limit);
            }
            artifacts.extend_from_slice(entries);
            Ok(())
        },
        |artifact, file| {
            if [geometry_path, map_path.as_str(), liquid_path.as_str()]
                .contains(&artifact.path.as_str())
            {
                let max = if artifact.path == geometry_path {
                    MAX_COVERAGE_VMAP
                } else {
                    MAP_LIMIT as usize
                };
                if artifact.bytes > max as u64 {
                    return Err(package::Error::Limit);
                }
                let mut bytes = vec![0; artifact.bytes as usize];
                file.read_exact(&mut bytes)
                    .map_err(|_| package::Error::Io)?;
                tables.insert(artifact.path.clone(), bytes);
            }
            Ok(())
        },
    )?;
    if checked != summary {
        return Err("product_input_changed".into());
    }
    let geometry_bytes = tables
        .remove(geometry_path)
        .ok_or("product_navigation_input_geometry")?;
    let geometry = collision_geometry_v22::Coverage::parse_v51(&geometry_bytes)?;
    let map = tables
        .remove(&map_path)
        .ok_or("product_navigation_input_map")?;
    let records = availability::read_navigation_maps_70450(&driver::CompleteDb2Source(&map))
        .map_err(|e| Error::from(e.code()))?;
    let liquid = tables
        .remove(&liquid_path)
        .ok_or("product_navigation_input_liquid_type")?;
    let liquid =
        terrain_sources::read_liquid_types_70450(&liquid).map_err(|e| Error::from(e.code()))?;
    let liquids = models::liquid::Table::from_source(&liquid, Default::default())
        .map_err(|e| Error::from(e.code()))?;
    if geometry.maps.len() > MAPS_MAX {
        return Err("product_navigation_limit".into());
    }
    Ok(Source {
        summary,
        artifacts,
        geometry,
        records,
        liquids,
        coverage_sha: integrity::digest(&geometry_bytes),
    })
}

fn source_limits(plan: &Plan) -> Result<package::Limits, Error> {
    let crate::plan::CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
        return Err("product_navigation_input_storage".into());
    };
    Ok(terrain_v20::package_limits_v51(budget))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MapFact {
    id: u32,
    root: u32,
    state: &'static str,
    tiles: Vec<Coordinate>,
}

fn owner(
    root: &Path,
    source: &Source,
    id: u32,
) -> Result<navigation_input::available::AvailableSelectedPackage, Error> {
    navigation_input::available::AvailableSelectedPackage::open_70450(
        &root.join("maps"),
        &root.join("vmaps"),
        &source.records,
        id,
        navigation_limits(),
    )
    .map_err(|e| Error::from(e.code()))
}

fn inventory(root: &Path, source: &Source) -> Result<Vec<MapFact>, Error> {
    let mut result = Vec::new();
    let mut tiles = 0usize;
    for map in &source.geometry.maps {
        if map.root_absent {
            result.push(MapFact {
                id: map.id,
                root: map.id,
                state: "root-absent",
                tiles: Vec::new(),
            });
            continue;
        }
        let mut package = owner(root, source, map.id)?;
        let entry = package
            .plan()
            .map(map.id)
            .ok_or("product_navigation_inventory")?;
        tiles = tiles
            .checked_add(entry.tile_count())
            .filter(|n| *n <= TILES_MAX)
            .ok_or("product_navigation_limit")?;
        result.push(MapFact {
            id: map.id,
            root: entry.root_map_id(),
            state: if entry.is_output() {
                "planned"
            } else {
                "empty"
            },
            tiles: entry.tiles().collect(),
        });
        package
            .verify_unchanged()
            .map_err(|e| Error::from(e.code()))?;
    }
    if result.windows(2).any(|w| w[0].id >= w[1].id) {
        return Err("product_navigation_inventory".into());
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Output {
    map: u32,
    global: Option<TileOutput>,
    tiles: Vec<TileOutput>,
}

struct Coverage52 {
    context: Context,
    source: Summary,
    maps: Vec<MapFact>,
    outputs: Vec<Output>,
    builder: u64,
    save: u64,
}

fn identity(source: Summary) -> [u8; 32] {
    integrity::digest(
        format!("rustycore-navigation-v52-sealed-v51\n{}\n", source.json()).as_bytes(),
    )
}

impl Coverage52 {
    fn render(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut text = format!(
            "navigation-coverage\t52\ncontext\t{}\nsource-context\t{}\nsource\t{}\t{}\t{}\n",
            self.context.json(),
            self.source.context.json(),
            self.source.files,
            self.source.bytes,
            integrity::hex(&self.source.inventory_sha256)
        );
        for (map, output) in self.maps.iter().zip(&self.outputs) {
            writeln!(
                text,
                "map\t{}\t{}\t{}\t{}",
                map.id,
                map.root,
                map.state,
                map.tiles.len()
            )
            .unwrap();
            if let Some(global) = &output.global {
                writeln!(
                    text,
                    "global\t{}\t{}",
                    global.bytes,
                    integrity::hex(&global.sha256)
                )
                .unwrap();
            }
            for tile in &output.tiles {
                writeln!(
                    text,
                    "tile\t{}\t{}\t{}\t{}\t{}",
                    tile.tile.row,
                    tile.tile.column,
                    tile.written,
                    tile.bytes,
                    integrity::hex(&tile.sha256)
                )
                .unwrap();
            }
        }
        writeln!(text, "work\t{}\t{}", self.builder, self.save).unwrap();
        if text.len() > COVERAGE_MAX {
            return Err("product_navigation_coverage_limit".into());
        }
        Ok(text.into_bytes())
    }

    fn validate(&self) -> Result<(), Error> {
        self.context.validate()?;
        self.source.context.validate()?;
        let tiles: usize = self.maps.iter().map(|m| m.tiles.len()).sum();
        let output_bytes = self
            .outputs
            .iter()
            .flat_map(|out| out.global.iter().chain(&out.tiles))
            .try_fold(0u64, |total, tile| total.checked_add(tile.bytes))
            .filter(|total| *total <= offline_output_limits().total_bytes)
            .ok_or("product_navigation_coverage_limit")?;
        let estimated = self
            .maps
            .len()
            .checked_mul(256)
            .and_then(|size| tiles.checked_mul(128).and_then(|n| size.checked_add(n)))
            .and_then(|size| size.checked_add(4096))
            .filter(|size| *size <= COVERAGE_MAX)
            .ok_or("product_navigation_coverage_limit")?;
        let _ = (output_bytes, estimated);
        let limits =
            navigation_input::run::AvailableLimits::all_available_v52(self.maps.len(), tiles)
                .map_err(|e| Error::from(e.code()))?;
        if self.context.build != 70450
            || self.context.components != 32
            || self.source.context.build != 70450
            || self.source.context.components != 25
            || self.source.context.locale != self.context.locale
            || self.context.source_sha256 != identity(self.source)
            || self.source.files == 0
            || self.source.files > MAX_INPUT_FILES
            || self.maps.len() != self.outputs.len()
            || self.maps.len() > MAPS_MAX
            || tiles > TILES_MAX
            || self.builder > limits.builder_work
            || self.save > limits.save_work
            || self.maps.windows(2).any(|w| w[0].id >= w[1].id)
        {
            return Err("product_navigation_coverage".into());
        }
        for (map, out) in self.maps.iter().zip(&self.outputs) {
            if map.id != out.map
                || map.id > 9999
                || map.root > 9999
                || map.tiles.len() > 4096
                || map.tiles.len() != out.tiles.len()
                || map
                    .tiles
                    .windows(2)
                    .any(|w| (w[0].row, w[0].column) >= (w[1].row, w[1].column))
                || ((map.state == "planned") != (!map.tiles.is_empty() && out.global.is_some()))
                || (map.state != "planned" && (!map.tiles.is_empty() || out.global.is_some()))
            {
                return Err("product_navigation_coverage".into());
            }
            if let Some(g) = out.global
                && (g.bytes == 0 || g.sha256 == [0; 32])
            {
                return Err("product_navigation_coverage".into());
            }
            for (coordinate, tile) in map.tiles.iter().zip(&out.tiles) {
                if coordinate != &tile.tile
                    || coordinate.row >= 64
                    || coordinate.column >= 64
                    || (tile.written && (tile.bytes == 0 || tile.sha256 == [0; 32]))
                    || (!tile.written && (tile.bytes != 0 || tile.sha256 != [0; 32]))
                {
                    return Err("product_navigation_coverage".into());
                }
            }
        }
        Ok(())
    }

    fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > COVERAGE_MAX {
            return Err("product_navigation_coverage_limit".into());
        }
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::from("product_navigation_coverage"))?;
        let mut lines = text.lines().peekable();
        if lines.next() != Some("navigation-coverage\t52") {
            return Err("product_navigation_coverage".into());
        }
        let context = Context::parse(next(&mut lines, "context", 2)?[1])?;
        let source_context = Context::parse(next(&mut lines, "source-context", 2)?[1])?;
        let fields = next(&mut lines, "source", 4)?;
        let source = Summary {
            context: source_context,
            files: num(fields[1])?,
            bytes: num(fields[2])?,
            inventory_sha256: sha(fields[3])?,
        };
        let mut maps = Vec::new();
        let mut outputs = Vec::new();
        let mut total = 0usize;
        while lines.peek().is_some_and(|s| s.starts_with("map\t")) {
            if maps.len() >= MAPS_MAX {
                return Err("product_navigation_coverage_limit".into());
            }
            let fields = next(&mut lines, "map", 5)?;
            let id = num(fields[1])?;
            let root = num(fields[2])?;
            let state = match fields[3] {
                "planned" => "planned",
                "empty" => "empty",
                "root-absent" => "root-absent",
                _ => return Err("product_navigation_coverage".into()),
            };
            let count: usize = num(fields[4])?;
            total = total
                .checked_add(count)
                .filter(|n| *n <= TILES_MAX)
                .ok_or("product_navigation_coverage_limit")?;
            if count > 4096 {
                return Err("product_navigation_coverage_limit".into());
            }
            let global = if state == "planned" {
                let f = next(&mut lines, "global", 3)?;
                Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: num(f[1])?,
                    sha256: sha(f[2])?,
                })
            } else {
                None
            };
            let mut tiles = Vec::new();
            for _ in 0..count {
                let f = next(&mut lines, "tile", 6)?;
                tiles.push(TileOutput {
                    tile: Coordinate {
                        row: num(f[1])?,
                        column: num(f[2])?,
                    },
                    written: match f[3] {
                        "true" => true,
                        "false" => false,
                        _ => return Err("product_navigation_coverage".into()),
                    },
                    bytes: num(f[4])?,
                    sha256: sha(f[5])?,
                });
            }
            maps.push(MapFact {
                id,
                root,
                state,
                tiles: tiles.iter().map(|t| t.tile).collect(),
            });
            outputs.push(Output {
                map: id,
                global,
                tiles,
            });
        }
        let f = next(&mut lines, "work", 3)?;
        let result = Self {
            context,
            source,
            maps,
            outputs,
            builder: num(f[1])?,
            save: num(f[2])?,
        };
        if lines.next().is_some() || result.render()? != bytes {
            return Err("product_navigation_coverage".into());
        }
        Ok(result)
    }
}

fn next<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    key: &str,
    count: usize,
) -> Result<Vec<&'a str>, Error> {
    let fields: Vec<_> = lines
        .next()
        .ok_or("product_navigation_coverage")?
        .split('\t')
        .collect();
    if fields.len() != count || fields[0] != key {
        return Err("product_navigation_coverage".into());
    }
    Ok(fields)
}
fn num<T: std::str::FromStr>(text: &str) -> Result<T, Error> {
    text.parse()
        .map_err(|_| Error::from("product_navigation_coverage"))
}
fn sha(text: &str) -> Result<[u8; 32], Error> {
    if text.len() != 64 || !text.is_ascii() {
        return Err("product_navigation_coverage".into());
    }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error::from("product_navigation_coverage"))?;
    }
    Ok(out)
}

fn paths(input: &Path, plan: &Path, output: &Path) -> Result<(PathBuf, PlanGuard, PathBuf), Error> {
    let root = absolute_without_symlinks(input, false)?;
    let guard = PlanGuard::open(plan)?;
    let output = output_path(output)?;
    if overlaps(&root, &output) || overlaps(&guard.path, &output) || overlaps(&root, &guard.path) {
        return Err("product_navigation_path_overlap".into());
    }
    Ok((root, guard, output))
}

pub(crate) fn extract(
    product: &Plan,
    input: &Path,
    input_plan: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
) -> Result<Summary, Error> {
    driver::admit(product)?;
    if !product.is_v52() {
        return Err("product_navigation_profile".into());
    }
    let (root, mut plan, output) = paths(input, input_plan, output)?;
    let source = source(&root, &plan, product)?;
    let mut pending = None;
    verify_locked_package(
        LockedPackage {
            root: &root,
            expected_summary: source.summary,
            expected_artifacts: &source.artifacts,
            coverage_path: collision_geometry_v22::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH,
            coverage_sha256: source.coverage_sha,
            max_coverage: MAX_COVERAGE_VMAP,
            limits: source_limits(&plan.plan)?,
        },
        || {
            let maps = inventory(&root, &source)?;
            let tile_count = maps.iter().map(|m| m.tiles.len()).sum();
            let mut limits =
                navigation_input::run::AvailableLimits::all_available_v52(maps.len(), tile_count)
                    .map_err(|e| Error::from(e.code()))?;
            let context = Context {
                build: 70450,
                locale: product.locale,
                components: 32,
                source_sha256: identity(source.summary),
                plan_sha256: integrity::digest(product.render().as_bytes()),
                producer_sha256: producer,
            };
            let protected = [root.as_path(), plan.path.as_path()];
            let mut run = if resume {
                Run::resume(&output, context, &protected, offline_output_limits())?
            } else {
                Run::create(&output, context, &protected, offline_output_limits())?
            };
            let mut outputs = Vec::new();
            let mut builder = 0u64;
            let mut save = 0u64;
            for map in &maps {
                if map.state != "planned" {
                    outputs.push(Output {
                        map: map.id,
                        global: None,
                        tiles: Vec::new(),
                    });
                    continue;
                }
                let mut owner = owner(&root, &source, map.id)?;
                let mut sink = NavigationSink {
                    run: &mut run,
                    selected_map: map.id,
                    global: None,
                    tiles: Vec::with_capacity(map.tiles.len()),
                    partial_triangulations: 0,
                    detail_center_fallbacks: 0,
                    unset_height_samples: 0,
                };
                let done = navigation_input::run::execute_available_selected(
                    &mut owner,
                    source.liquids.types(),
                    &[],
                    navigation_input::run::Options {
                        selection: navigation_input::run::Selection::Map(map.id),
                        ..Default::default()
                    },
                    limits,
                    &mut sink,
                )
                .map_err(|e| {
                    if std::env::var_os("RCE_V52_DEBUG").is_some() {
                        eprintln!("{{\"map_id\":{},\"cause\":{:?}}}", map.id, e.cause);
                    }
                    match e.cause {
                        navigation_input::run::Cause::Run(e) => Error::from(e.code()),
                        navigation_input::run::Cause::Sink(e) => e,
                    }
                })?;
                limits.run.output_bytes = limits
                    .run
                    .output_bytes
                    .checked_sub(done.completion.progress.output_bytes)
                    .ok_or("product_navigation_limit")?;
                builder = builder
                    .checked_add(done.builder_work)
                    .ok_or("product_navigation_limit")?;
                save = save
                    .checked_add(done.save_work)
                    .ok_or("product_navigation_limit")?;
                limits.builder_work = limits
                    .builder_work
                    .checked_sub(done.builder_work)
                    .ok_or("product_navigation_limit")?;
                limits.save_work = limits
                    .save_work
                    .checked_sub(done.save_work)
                    .ok_or("product_navigation_limit")?;
                outputs.push(Output {
                    map: map.id,
                    global: sink.global,
                    tiles: sink.tiles,
                });
            }
            let coverage = Coverage52 {
                context,
                source: source.summary,
                maps,
                outputs,
                builder,
                save,
            };
            let bytes = coverage.render()?;
            run.put(Component::Navigation, COVERAGE, &bytes)?;
            pending = Some(run);
            Ok(())
        },
    )?;
    plan.verify()?;
    let mut run = pending.ok_or("product_navigation_output")?;
    run.finish_component(Component::Navigation)?;
    run.finish().map_err(Error::from)
}

fn verify_output(
    product: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<(Summary, Coverage52), Error> {
    driver::admit(product)?;
    if !product.is_v52() {
        return Err("product_navigation_profile".into());
    }
    let mut artifacts = Vec::new();
    let mut coverage = None;
    let mut failure: Option<Error> = None;
    let summary = package::verify_with_preflight(
        output,
        expected,
        offline_output_limits(),
        |entries| {
            if entries.len() > TILES_MAX + MAPS_MAX + 1 {
                return Err(package::Error::Limit);
            }
            artifacts.extend_from_slice(entries);
            Ok(())
        },
        |artifact, file| {
            let result = (|| {
                if artifact.component != Component::Navigation {
                    return Err("product_navigation_output".into());
                }
                if artifact.path == COVERAGE {
                    coverage = Some(Coverage52::parse(&read_bounded(
                        file,
                        artifact.bytes,
                        COVERAGE_MAX,
                    )?)?);
                }
                Ok(())
            })();
            result.map_err(|e| {
                failure = Some(e);
                package::Error::Schema
            })
        },
    )
    .map_err(|e| failure.unwrap_or_else(|| e.into()))?;
    let coverage = coverage.ok_or("product_navigation_coverage")?;
    if coverage.context != summary.context
        || summary.context.locale != product.locale
        || summary.context.plan_sha256 != integrity::digest(product.render().as_bytes())
    {
        return Err("product_navigation_identity".into());
    }
    let mut wanted = BTreeMap::new();
    for out in &coverage.outputs {
        if let Some(global) = out.global {
            wanted.insert(
                mmap::global_path(out.map),
                (global.bytes, global.sha256, Some(out.map)),
            );
        }
        for tile in &out.tiles {
            if tile.written {
                wanted.insert(
                    mmap::tile_path(out.map, tile.tile.row, tile.tile.column)
                        .map_err(|e| Error::from(e.code()))?,
                    (tile.bytes, tile.sha256, None),
                );
            }
        }
    }
    if artifacts.len() != wanted.len() + 1 {
        return Err("product_navigation_inventory".into());
    }
    let checked = package::verify_with(
        output,
        Some(summary.context),
        offline_output_limits(),
        |artifact, file| {
            let result = (|| {
                if artifact.path == COVERAGE {
                    return Ok(());
                }
                let (bytes, sha, map) = wanted
                    .get(&artifact.path)
                    .ok_or("product_navigation_inventory")?;
                if artifact.bytes != *bytes || artifact.sha256 != *sha {
                    return Err("product_navigation_inventory".into());
                }
                let bytes = read_bounded(file, artifact.bytes, 64 * 1024 * 1024)?;
                if let Some(id) = map {
                    let view = mmap::GlobalView::parse(&bytes, *id, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                    if view.sources().len() != 0 {
                        return Err("product_navigation_offmesh".into());
                    }
                } else {
                    mmap::TileView::parse(&bytes, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                }
                Ok(())
            })();
            result.map_err(|e| {
                failure = Some(e);
                package::Error::Schema
            })
        },
    )
    .map_err(|e| failure.unwrap_or_else(|| e.into()))?;
    if checked != summary {
        return Err("product_input_changed".into());
    }
    Ok((summary, coverage))
}

pub(crate) fn verify(
    product: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<Summary, Error> {
    verify_output(product, output, expected).map(|p| p.0)
}

pub(crate) fn verify_links(
    product: &Plan,
    input: &Path,
    input_plan: &Path,
    output: &Path,
    expected: Option<Context>,
) -> Result<Summary, Error> {
    let (root, mut plan, output) = paths(input, input_plan, output)?;
    let source = source(&root, &plan, product)?;
    let mut result = None;
    verify_locked_package(
        LockedPackage {
            root: &root,
            expected_summary: source.summary,
            expected_artifacts: &source.artifacts,
            coverage_path: collision_geometry_v22::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH,
            coverage_sha256: source.coverage_sha,
            max_coverage: MAX_COVERAGE_VMAP,
            limits: source_limits(&plan.plan)?,
        },
        || {
            let (summary, coverage) = verify_output(product, &output, expected)?;
            if coverage.source != source.summary || coverage.maps != inventory(&root, &source)? {
                return Err("product_navigation_input_context".into());
            }
            result = Some(summary);
            Ok(())
        },
    )?;
    plan.verify()?;
    result.ok_or_else(|| Error::from("product_navigation_output"))
}
