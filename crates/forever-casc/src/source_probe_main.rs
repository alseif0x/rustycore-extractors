// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, pure-Rust checks of compatible-VMaps source readers.
use forever_casc::{Error, Locale, local_index, local_store::LocalStore, root};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
};

const MAX_BYTES: u32 = 128 * 1024 * 1024;
/// Default complete-file budget for an optional map WDT/tile probe.
const DEFAULT_TILE_BYTES: u32 = 1024 * 1024;
const MAX_MAP_TILES: usize = 4;
const SOURCE_ORDER: [SourceKind; 4] = [
    SourceKind::GameObjectDisplayInfo,
    SourceKind::Map,
    SourceKind::LiquidMaterial,
    SourceKind::LiquidType,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceKind {
    GameObjectDisplayInfo,
    Map,
    LiquidMaterial,
    LiquidType,
}
impl SourceKind {
    const fn name(self) -> &'static str {
        match self {
            Self::GameObjectDisplayInfo => "gameobject_display_info",
            Self::Map => "map",
            Self::LiquidMaterial => "liquid_material",
            Self::LiquidType => "liquid_type",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    client: PathBuf,
    locale: Locale,
    build: u32,
    max_bytes: u32,
    map: Option<u32>,
    tile_bytes: u32,
}

fn parse(args: Vec<OsString>) -> Result<Options, ()> {
    let (mut client, mut locale, mut build, mut max_bytes, mut map, mut tile_bytes) =
        (None, None, None, None, None, None);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let value = args.next().ok_or(())?;
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err(());
        }
        match arg.to_str().ok_or(())? {
            "--client" if client.is_none() => client = Some(PathBuf::from(value)),
            "--locale" if locale.is_none() => {
                locale = Some(Locale::parse(value.to_str().ok_or(())?).map_err(|_| ())?)
            }
            "--build" if build.is_none() => {
                let parsed = value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?;
                if ![70374, 70425, 70450].contains(&parsed) {
                    return Err(());
                }
                build = Some(parsed);
            }
            "--max-bytes" if max_bytes.is_none() => {
                max_bytes = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--map" if map.is_none() => {
                map = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--tile-bytes" if tile_bytes.is_none() => {
                tile_bytes = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            _ => return Err(()),
        }
    }
    let max_bytes = max_bytes.unwrap_or(MAX_BYTES);
    let tile_bytes_was_set = tile_bytes.is_some();
    let tile_bytes = tile_bytes.unwrap_or(DEFAULT_TILE_BYTES);
    if max_bytes == 0
        || max_bytes > MAX_BYTES
        || tile_bytes == 0
        || tile_bytes > MAX_BYTES
        || (tile_bytes_was_set && map.is_none())
    {
        return Err(());
    }
    Ok(Options {
        client: client.ok_or(())?,
        locale: locale.ok_or(())?,
        build: build.ok_or(())?,
        max_bytes,
        map,
        tile_bytes,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CryptoFacts {
    full_ckey_verified: bool,
    encoded_key_authenticated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceResult {
    name: &'static str,
    code: Option<&'static str>,
    crypto: Option<CryptoFacts>,
}
impl SourceResult {
    const fn success(name: &'static str, crypto: Option<CryptoFacts>) -> Self {
        Self {
            name,
            code: None,
            crypto,
        }
    }
    const fn failure(name: &'static str, code: &'static str, crypto: Option<CryptoFacts>) -> Self {
        Self {
            name,
            code: Some(code),
            crypto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReadStatus {
    status: &'static str,
    code: Option<&'static str>,
}
impl ReadStatus {
    const fn success() -> Self {
        Self {
            status: "success",
            code: None,
        }
    }
    const fn failure(code: &'static str) -> Self {
        Self {
            status: "error",
            code: Some(code),
        }
    }
}

#[derive(Debug)]
struct MapReport {
    map_id: u32,
    status: &'static str,
    code: Option<&'static str>,
    wdt: Option<ReadStatus>,
    tiles: Vec<ReadStatus>,
}
impl MapReport {
    fn unresolved(map_id: u32, status: &'static str, code: &'static str) -> Self {
        Self {
            map_id,
            status,
            code: Some(code),
            wdt: None,
            tiles: Vec::new(),
        }
    }
    fn render(&self) -> String {
        let mut json = format!("{{\"map\":{},\"status\":\"{}\"", self.map_id, self.status);
        if let Some(code) = self.code {
            json.push_str(&format!(",\"code\":\"{code}\""));
        }
        json.push_str(",\"wdt\":");
        json.push_str(
            &self
                .wdt
                .map_or_else(|| "null".to_owned(), render_read_status),
        );
        json.push_str(",\"tiles\":[");
        for (index, tile) in self.tiles.iter().enumerate() {
            if index != 0 {
                json.push(',');
            }
            json.push_str(&render_read_status(*tile));
        }
        json.push_str("]}");
        json
    }
}

fn render_read_status(status: ReadStatus) -> String {
    match status.code {
        Some(code) => format!("{{\"status\":\"{}\",\"code\":\"{code}\"}}", status.status),
        None => format!("{{\"status\":\"{}\"}}", status.status),
    }
}

#[derive(Debug)]
struct Report {
    requested_build: u32,
    resolved_build: Option<u32>,
    admitted: bool,
    admission_code: &'static str,
    build_info_code: Option<&'static str>,
    sources: Vec<SourceResult>,
    map: Option<MapReport>,
    exit_code: u8,
}
impl Report {
    fn render(&self) -> String {
        let resolved = self
            .resolved_build
            .map_or_else(|| "null".to_owned(), |build| build.to_string());
        let build_info = self
            .build_info_code
            .map_or_else(|| "null".to_owned(), |code| format!("\"{code}\""));
        let mut json = format!(
            "{{\"report_version\":1,\"requested_build\":{},\"resolved_build\":{resolved},\"admitted\":{},\"admission_code\":\"{}\",\"build_info_code\":{build_info},\"source_policy\":\"continue_in_product_order\",\"sources\":[",
            self.requested_build, self.admitted, self.admission_code
        );
        for (index, source) in self.sources.iter().enumerate() {
            if index != 0 {
                json.push(',');
            }
            let status = source.code.map_or_else(
                || "\"status\":\"success\"".to_owned(),
                |code| format!("\"status\":\"error\",\"code\":\"{code}\""),
            );
            json.push_str(&format!("{{\"name\":\"{}\",{status}", source.name));
            if let Some(crypto) = source.crypto {
                json.push_str(&format!(
                    ",\"full_ckey_verified\":{},\"encoded_key_authenticated\":{}",
                    crypto.full_ckey_verified, crypto.encoded_key_authenticated
                ));
            }
            json.push('}');
        }
        json.push(']');
        if let Some(map) = &self.map {
            json.push_str(&format!(",\"map\":{}", map.render()));
        }
        json.push_str(",\"paths_redacted\":true}");
        json
    }
}

fn error_report(code: &'static str) -> String {
    format!(
        "{{\"report_version\":1,\"status\":\"error\",\"code\":\"{code}\",\"paths_redacted\":true}}"
    )
}

#[cfg(target_os = "linux")]
fn admit(client: &Path, build: u32) -> Result<u32, Error> {
    match build {
        70374 => forever_casc::admit_db2_70374(client),
        70425 => forever_casc::admit_db2_70425(client),
        70450 => forever_casc::admit_db2_70450(client),
        _ => Err(Error::UnsupportedClient),
    }
}

#[cfg(not(target_os = "linux"))]
fn admit(_: &Path, _: u32) -> Result<u32, Error> {
    Err(Error::UnsupportedClient)
}

#[cfg(target_os = "linux")]
fn open_store(client: &Path, locale: Locale, build: u32) -> Result<LocalStore, Error> {
    let options = forever_casc::local_store::OpenOptions {
        locale,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: Default::default(),
    };
    match build {
        70374 => LocalStore::open_db2_70374(client, options).map_err(Error::LocalStore),
        70425 => LocalStore::open_db2_70425(client, options).map_err(Error::LocalStore),
        70450 => LocalStore::open_db2_70450(client, options).map_err(Error::LocalStore),
        _ => Err(Error::UnsupportedClient),
    }
}

#[cfg(not(target_os = "linux"))]
fn open_store(_: &Path, _: Locale, _: u32) -> Result<LocalStore, Error> {
    Err(Error::UnsupportedClient)
}

#[cfg(target_os = "linux")]
fn read_available(
    store: &mut LocalStore,
    build: u32,
    id: u32,
    max_bytes: u32,
) -> Result<forever_casc::local_store::availability::AvailableSource, &'static str> {
    let cap = u64::from(max_bytes);
    match build {
        70374 => store.read_file_data_id_available_with_limit(id, cap),
        70425 => store.read_file_data_id_available_70425_with_limit(id, cap),
        70450 => store.read_file_data_id_available_70450_with_limit(id, cap),
        _ => return Err("unsupported_client"),
    }
    .map_err(|error| error.code())
}

#[cfg(target_os = "linux")]
fn probe_available(
    store: &mut LocalStore,
    build: u32,
    name: &'static str,
    id: u32,
    max_bytes: u32,
    reader: impl FnOnce(
        &forever_casc::local_store::availability::AvailableSource,
    ) -> Result<(), forever_db2::RecordError>,
) -> SourceResult {
    let source = match read_available(store, build, id, max_bytes) {
        Ok(source) => source,
        Err(code) => return SourceResult::failure(name, code, None),
    };
    let crypto = Some(CryptoFacts {
        full_ckey_verified: source.full_ckey_verified(),
        encoded_key_authenticated: source.encoded_key_authenticated(),
    });
    match reader(&source) {
        Ok(()) => SourceResult::success(name, crypto),
        Err(error) => SourceResult::failure(name, error.code(), crypto),
    }
}

#[cfg(target_os = "linux")]
fn probe_complete(
    store: &mut LocalStore,
    name: &'static str,
    id: u32,
    max_bytes: u32,
    reader: impl FnOnce(&[u8]) -> Result<(), forever_db2::RecordError>,
) -> SourceResult {
    let file = match store.read_file_data_id_with_limit(id, u64::from(max_bytes)) {
        Ok(file) => file,
        Err(error) => return SourceResult::failure(name, error.code(), None),
    };
    match reader(&file.bytes) {
        Ok(()) => SourceResult::success(name, None),
        Err(error) => SourceResult::failure(name, error.code(), None),
    }
}

#[cfg(target_os = "linux")]
fn read_map_rows(
    source: &forever_casc::local_store::availability::AvailableSource,
    build: u32,
) -> Result<(), forever_db2::RecordError> {
    use forever_db2::availability;

    match build {
        70374 => availability::read_map_70374(source).map(|_| ()),
        70425 => availability::read_map_70425(source).map(|_| ()),
        70450 => availability::read_map_70450(source).map(|_| ()),
        _ => unreachable!(),
    }
}

#[cfg(target_os = "linux")]
fn read_vmap_map_rows(
    source: &forever_casc::local_store::availability::AvailableSource,
    build: u32,
) -> Result<
    forever_db2::availability::AvailableRecords<forever_db2::terrain_sources::VMapFields>,
    forever_db2::RecordError,
> {
    use forever_db2::availability;

    match build {
        70374 => availability::read_vmap_maps_70374(source),
        70425 => availability::read_vmap_maps_70425(source),
        70450 => availability::read_vmap_maps_70450(source),
        _ => unreachable!(),
    }
}

#[cfg(target_os = "linux")]
fn probe_map_source(
    store: &mut LocalStore,
    build: u32,
    max_bytes: u32,
    retain_for_map_probe: bool,
) -> (
    SourceResult,
    Option<Result<forever_db2::availability::MaterializedVMapMaps, &'static str>>,
) {
    let name = SourceKind::Map.name();
    let source = match read_available(
        store,
        build,
        forever_casc::collision::world::MAP_FILE_DATA_ID,
        max_bytes,
    ) {
        Ok(source) => source,
        Err(code) => {
            return (
                SourceResult::failure(name, code, None),
                retain_for_map_probe.then_some(Err(code)),
            );
        }
    };
    let crypto = Some(CryptoFacts {
        full_ckey_verified: source.full_ckey_verified(),
        encoded_key_authenticated: source.encoded_key_authenticated(),
    });
    if let Err(error) = read_map_rows(&source, build) {
        let code = error.code();
        return (
            SourceResult::failure(name, code, crypto),
            retain_for_map_probe.then_some(Err(code)),
        );
    }
    if !retain_for_map_probe {
        return (SourceResult::success(name, crypto), None);
    }
    let rows = match read_vmap_map_rows(&source, build) {
        Ok(rows) => rows,
        Err(error) => {
            let code = error.code();
            return (SourceResult::failure(name, code, crypto), Some(Err(code)));
        }
    };
    match rows.materialize() {
        Ok(rows) => (SourceResult::success(name, crypto), Some(Ok(rows))),
        Err(error) => {
            let code = error.code();
            (SourceResult::failure(name, code, crypto), Some(Err(code)))
        }
    }
}

#[cfg(target_os = "linux")]
fn map_not_resolved(map_id: u32, status: &'static str, code: &'static str) -> MapReport {
    MapReport::unresolved(map_id, status, code)
}

#[cfg(target_os = "linux")]
fn probe_map_reads(
    store: &mut LocalStore,
    map_id: u32,
    tile_bytes: u32,
    rows: Result<forever_db2::availability::MaterializedVMapMaps, &'static str>,
) -> MapReport {
    let rows = match rows {
        Ok(rows) => rows,
        Err(code) => return map_not_resolved(map_id, "error", code),
    };
    let direct_rows = rows.direct_records();
    let Some(row) = direct_rows.iter().find(|row| row.id == map_id) else {
        if rows.rows().iter().any(|row| row.id == map_id) {
            return map_not_resolved(map_id, "error", "collision_plan_copied_selection");
        }
        return map_not_resolved(map_id, "not_present", "map_not_present");
    };
    if row.fields.wdt_file_data_id == 0 {
        return map_not_resolved(map_id, "inactive", "map_inactive");
    }
    let maps = match forever_casc::collision::plan::Maps::from_materialized_selection(
        rows.rows(),
        direct_rows.len(),
        forever_casc::collision::plan::MapSelection::MapAndAncestors(map_id),
        Default::default(),
    ) {
        Ok(maps) => maps,
        Err(error) => return map_not_resolved(map_id, "error", error.code()),
    };
    let Some(map) = maps.get(map_id) else {
        return map_not_resolved(map_id, "not_present", "map_not_present");
    };

    let limits = forever_casc::collision::run::Limits::default();
    let wdt_cap = tile_bytes.min(limits.max_file_bytes.min(limits.wdt.max_wdt_bytes) as u32);
    let wdt_file =
        match store.read_file_data_id_with_limit(map.wdt_file_data_id(), u64::from(wdt_cap)) {
            Ok(file) => file,
            Err(error) => {
                return MapReport {
                    map_id,
                    status: "resolved",
                    code: None,
                    wdt: Some(ReadStatus::failure(error.code())),
                    tiles: Vec::new(),
                };
            }
        };
    let wdt = match forever_casc::collision::plan::Wdt::parse(&wdt_file.bytes, limits.wdt) {
        Ok(wdt) => wdt,
        Err(error) => {
            return MapReport {
                map_id,
                status: "resolved",
                code: None,
                wdt: Some(ReadStatus::failure(error.code())),
                tiles: Vec::new(),
            };
        }
    };
    let mut requests = match wdt.tiles(map) {
        Ok(requests) => requests,
        Err(error) => {
            return MapReport {
                map_id,
                status: "resolved",
                code: None,
                wdt: Some(ReadStatus::failure(error.code())),
                tiles: Vec::new(),
            };
        }
    };
    let tile_cap = tile_bytes.min(limits.max_file_bytes.min(limits.objects.max_input_bytes) as u32);
    let mut name_buffer = [0; forever_casc::collision::plan::MAX_NAME_BYTES];
    let mut tiles = Vec::with_capacity(MAX_MAP_TILES);
    for _ in 0..MAX_MAP_TILES {
        let Some(request) = requests.next(&mut name_buffer) else {
            break;
        };
        let status = match request.target {
            forever_casc::collision::plan::TileTarget::FileDataId(id) => {
                match store.read_file_data_id_with_limit(id, u64::from(tile_cap)) {
                    Ok(_) => ReadStatus::success(),
                    Err(error) => ReadStatus::failure(error.code()),
                }
            }
            forever_casc::collision::plan::TileTarget::Name(_) => {
                ReadStatus::failure("collision_tile_target_not_file_data_id")
            }
        };
        tiles.push(status);
    }
    MapReport {
        map_id,
        status: "resolved",
        code: None,
        wdt: Some(ReadStatus::success()),
        tiles,
    }
}

#[cfg(target_os = "linux")]
fn source_results(
    store: &mut LocalStore,
    build: u32,
    max_bytes: u32,
    map_requested: bool,
) -> (
    Vec<SourceResult>,
    Option<Result<forever_db2::availability::MaterializedVMapMaps, &'static str>>,
) {
    use forever_casc::collision::{gameobjects, world};
    use forever_db2::{availability, terrain_sources};

    let mut sources = Vec::with_capacity(SOURCE_ORDER.len());
    let mut map_rows = None;
    for source in SOURCE_ORDER {
        let result = match source {
            SourceKind::GameObjectDisplayInfo => probe_available(
                store,
                build,
                source.name(),
                gameobjects::TABLE_FILE_DATA_ID,
                max_bytes,
                |source| match build {
                    70374 => availability::read_gameobject_display_info_70374(source).map(|_| ()),
                    70425 => availability::read_gameobject_display_info_70425(source).map(|_| ()),
                    70450 => availability::read_gameobject_display_info_70450(source).map(|_| ()),
                    _ => unreachable!(),
                },
            ),
            SourceKind::Map => {
                let (result, rows) = probe_map_source(store, build, max_bytes, map_requested);
                map_rows = rows;
                result
            }
            SourceKind::LiquidMaterial => probe_complete(
                store,
                source.name(),
                world::MATERIAL_FILE_DATA_ID,
                max_bytes,
                |bytes| match build {
                    70374 => terrain_sources::read_liquid_materials(bytes).map(|_| ()),
                    70425 => terrain_sources::read_liquid_materials_70425(bytes).map(|_| ()),
                    70450 => terrain_sources::read_liquid_materials_70450(bytes).map(|_| ()),
                    _ => unreachable!(),
                },
            ),
            SourceKind::LiquidType => probe_complete(
                store,
                source.name(),
                world::TYPE_FILE_DATA_ID,
                max_bytes,
                |bytes| match build {
                    70374 => terrain_sources::read_liquid_types(bytes).map(|_| ()),
                    70425 => terrain_sources::read_liquid_types_70425(bytes).map(|_| ()),
                    70450 => terrain_sources::read_liquid_types_70450(bytes).map(|_| ()),
                    _ => unreachable!(),
                },
            ),
        };
        sources.push(result);
    }
    (sources, map_rows)
}

#[cfg(target_os = "linux")]
fn run(options: Options) -> Report {
    let admitted_build = match admit(&options.client, options.build) {
        Ok(build) if build == options.build => build,
        Ok(_) => {
            return Report {
                requested_build: options.build,
                resolved_build: None,
                admitted: false,
                admission_code: "unsupported_client",
                build_info_code: None,
                sources: Vec::new(),
                map: options.map.map(|map_id| {
                    MapReport::unresolved(map_id, "not_attempted", "unsupported_client")
                }),
                exit_code: 3,
            };
        }
        Err(error) => {
            return Report {
                requested_build: options.build,
                resolved_build: None,
                admitted: false,
                admission_code: error.code(),
                build_info_code: None,
                sources: Vec::new(),
                map: options
                    .map
                    .map(|map_id| MapReport::unresolved(map_id, "not_attempted", error.code())),
                exit_code: error.exit_code(),
            };
        }
    };
    let mut store = match open_store(&options.client, options.locale, admitted_build) {
        Ok(store) => store,
        Err(error) => {
            return Report {
                requested_build: options.build,
                resolved_build: Some(admitted_build),
                admitted: true,
                admission_code: "ok",
                build_info_code: Some(error.code()),
                sources: Vec::new(),
                map: options
                    .map
                    .map(|map_id| MapReport::unresolved(map_id, "not_attempted", error.code())),
                exit_code: error.exit_code(),
            };
        }
    };
    let resolved_build = store.build().build;
    let (sources, map_rows) = source_results(
        &mut store,
        resolved_build,
        options.max_bytes,
        options.map.is_some(),
    );
    let map = options.map.map(|map_id| {
        probe_map_reads(
            &mut store,
            map_id,
            options.tile_bytes,
            map_rows.unwrap_or(Err("map_source_not_available")),
        )
    });
    Report {
        requested_build: options.build,
        resolved_build: Some(resolved_build),
        admitted: true,
        admission_code: "ok",
        build_info_code: Some("ok"),
        sources,
        map,
        exit_code: 0,
    }
}

#[cfg(not(target_os = "linux"))]
fn run(options: Options) -> Report {
    let error = admit(&options.client, options.build).unwrap_err();
    Report {
        requested_build: options.build,
        resolved_build: None,
        admitted: false,
        admission_code: error.code(),
        build_info_code: None,
        sources: Vec::new(),
        map: options
            .map
            .map(|map_id| MapReport::unresolved(map_id, "not_attempted", error.code())),
        exit_code: error.exit_code(),
    }
}

fn main() -> ExitCode {
    let options = match parse(std::env::args_os().skip(1).collect()) {
        Ok(options) => options,
        Err(()) => {
            println!("{}", error_report("invalid_invocation"));
            return ExitCode::from(2);
        }
    };
    let report = run(options);
    println!("{}", report.render());
    ExitCode::from(report.exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &str) -> Vec<OsString> {
        input.split_whitespace().map(OsString::from).collect()
    }

    #[test]
    fn parser_accepts_only_bounded_supported_source_probe_invocations() {
        let parsed = parse(args(
            "--client local --locale esES --build 70450 --max-bytes 1024",
        ))
        .unwrap();
        assert_eq!(
            parsed,
            Options {
                client: PathBuf::from("local"),
                locale: Locale::EsEs,
                build: 70450,
                max_bytes: 1024,
                map: None,
                tile_bytes: DEFAULT_TILE_BYTES,
            }
        );
        let defaults = parse(args("--client c --locale enUS --build 70374")).unwrap();
        assert_eq!(defaults.max_bytes, MAX_BYTES);
        assert_eq!(defaults.map, None);
        assert_eq!(defaults.tile_bytes, DEFAULT_TILE_BYTES);
        let mapped = parse(args(
            "--client c --locale enUS --build 70450 --map 389 --tile-bytes 4096",
        ))
        .unwrap();
        assert_eq!(mapped.map, Some(389));
        assert_eq!(mapped.tile_bytes, 4096);
    }

    #[test]
    fn parser_rejects_missing_duplicate_unknown_and_out_of_range_values() {
        for malformed in [
            "",
            "--client c --locale enUS --build 70450 --build 70425",
            "--client c --locale enUS --build 70450 --max-bytes 1 --max-bytes 2",
            "--client c --locale enUS --build 70450 --unknown x",
            "--client c --locale enUS --build 99999",
            "--client c --locale enUS --build 70450 --map invalid",
            "--client c --locale enUS --build 70450 --map 4294967296",
            "--client c --locale enUS --build 70450 --map 389 --map 390",
            "--client c --locale enUS --build 70450 --tile-bytes 1024",
            "--client c --locale enUS --build 70450 --map 389 --tile-bytes 134217729",
            "--client c --locale enUS --build 70450 --max-bytes 0",
            "--client c --locale enUS --build 70450 --max-bytes 134217729",
            "--client c --locale zzZZ --build 70450",
        ] {
            assert!(parse(args(malformed)).is_err(), "{malformed}");
        }
    }

    #[test]
    fn report_is_single_line_redacted_and_preserves_source_order_and_crypto_facts() {
        let report = Report {
            requested_build: 70450,
            resolved_build: Some(70450),
            admitted: true,
            admission_code: "ok",
            build_info_code: Some("ok"),
            sources: SOURCE_ORDER
                .into_iter()
                .enumerate()
                .map(|(index, source)| {
                    if index == 0 {
                        SourceResult::failure(
                            source.name(),
                            "local_store_no_verified_encoding",
                            Some(CryptoFacts {
                                full_ckey_verified: false,
                                encoded_key_authenticated: true,
                            }),
                        )
                    } else {
                        SourceResult::success(source.name(), None)
                    }
                })
                .collect(),
            map: Some(MapReport {
                map_id: 389,
                status: "resolved",
                code: None,
                wdt: Some(ReadStatus::failure("local_store_no_verified_encoding")),
                tiles: vec![ReadStatus::success()],
            }),
            exit_code: 0,
        };
        let json = report.render();
        assert!(!json.contains('\n'));
        assert!(!json.contains("client-root"));
        assert!(json.contains("\"paths_redacted\":true"));
        assert!(json.contains("\"admission_code\":\"ok\""));
        assert!(json.contains("\"build_info_code\":\"ok\""));
        let names: Vec<_> = SOURCE_ORDER.map(SourceKind::name).into_iter().collect();
        let positions: Vec<_> = names
            .iter()
            .map(|name| json.find(&format!("\"name\":\"{name}\"")).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(json.contains("\"code\":\"local_store_no_verified_encoding\""));
        assert!(json.contains("\"encoded_key_authenticated\":true"));
        assert!(!json.contains("\"file_data_id\""));
        assert!(json.contains(
            "\"map\":{\"map\":389,\"status\":\"resolved\",\"wdt\":{\"status\":\"error\",\"code\":\"local_store_no_verified_encoding\"},\"tiles\":[{\"status\":\"success\"}]}"
        ));
        assert_eq!(
            MapReport::unresolved(777, "not_present", "map_not_present").render(),
            "{\"map\":777,\"status\":\"not_present\",\"code\":\"map_not_present\",\"wdt\":null,\"tiles\":[]}"
        );
    }

    #[test]
    fn malformed_invocation_report_is_redacted() {
        assert_eq!(
            error_report("invalid_invocation"),
            "{\"report_version\":1,\"status\":\"error\",\"code\":\"invalid_invocation\",\"paths_redacted\":true}"
        );
    }
}
