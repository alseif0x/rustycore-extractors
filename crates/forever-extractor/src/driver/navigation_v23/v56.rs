// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[cfg(test)]
mod tests;

pub(crate) const COVERAGE: &str = "mmaps/available-navigation-v56.tsv";
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

#[cfg(test)]
pub(crate) use forever_navigation::package_v56::sha;
pub(crate) use forever_navigation::package_v56::{Coverage56, MapFact, Output};

fn identity(source: Summary) -> [u8; 32] {
    integrity::digest(
        format!("rustycore-navigation-v56-sealed-v51\n{}\n", source.json()).as_bytes(),
    )
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
    if !product.is_v56() {
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
                navigation_input::run::AvailableLimits::all_available_v56(maps.len(), tile_count)
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
                        partial_triangulations: 0,
                        detail_center_fallbacks: 0,
                        unset_height_samples: 0,
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
                    if std::env::var_os("RCE_V56_DEBUG").is_some() {
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
                    partial_triangulations: sink.partial_triangulations,
                    detail_center_fallbacks: sink.detail_center_fallbacks,
                    unset_height_samples: sink.unset_height_samples,
                });
            }
            let coverage = Coverage56 {
                context,
                source: source.summary,
                maps,
                outputs,
                builder,
                save,
            };
            let bytes = coverage
                .render()
                .map_err(|error| Error::from(error.code()))?;
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
) -> Result<(Summary, Coverage56), Error> {
    driver::admit(product)?;
    if !product.is_v56() {
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
                    coverage = Some(
                        Coverage56::parse(&read_bounded(file, artifact.bytes, COVERAGE_MAX)?)
                            .map_err(|error| Error::from(error.code()))?,
                    );
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
