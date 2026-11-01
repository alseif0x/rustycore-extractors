// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored 2026-10-11: explicit offline product invocation.
use forever_casc::integrity;
#[cfg(any(target_os = "linux", windows))]
use forever_casc::package::Context;
use forever_extractor::{
    Error, input,
    plan::{CollisionStorage, Plan},
};
use std::{collections::BTreeMap, path::PathBuf};

fn main() {
    let mut contract = forever_extractor::CONTRACT;
    match run(&mut contract) {
        Ok(json) => {
            let help = json.starts_with("forever-extractor inspect --plan ");
            if help {
                print!("{json}");
                println!(
                    "forever-extractor probe-encoding --plan PLAN --client CLIENT --output NEW_TRACE --file-data-id FDID [--file-data-id FDID ...] (Linux only; build 70450 revisions 15-20; --keys is forbidden)"
                );
                println!(
                    "Linux v20 selects build-70450 available-map terrain-only output; probe-encoding reports metadata/read diagnostics only."
                );
                println!(
                    "Linux v21 selects build-70450 DB2+Terrain+VMap+MMap navigation for one map and ancestors with complete keyless Map/LiquidType reads; real inputs are not accepted yet."
                );
                println!(
                    "Linux v22 selects build-70450 VMap geometry for one map and its available active ancestors; GameObject globals are excluded by policy, LiquidObject is out of scope, and no MMap/full-world/runtime/parity claim is made."
                );
                println!(
                    "Linux v23 selects build-70450 navigation for one map from sealed v20 terrain and v22 VMap packages; extract/resume and verify-links require both package/plan pairs. Source coverage remains producer-reported; this is not a full-extractor or runtime claim."
                );
                println!(
                    "v23 extract|resume and verify-links require --terrain-package PACKAGE --terrain-plan PLAN --vmap-package PACKAGE --vmap-plan PLAN; plan-only inspect is allowed, client inspect is not."
                );
                println!(
                    "Linux v24 combines the sealed build-70450 Map DB2 artifact with v20 available-map terrain; offline verification independently re-derives Map identities from that artifact. This is software scope, not full-world or runtime acceptance."
                );
                println!(
                    "Linux v25 combines build-70450 available Map terrain with an explicitly declared TACT-section skip projection; only checksum-verified S-cipher E frames matching tact_declared Map sections are represented, without decryption or a complete-source-decode claim."
                );
                println!(
                    "Linux v29 publishes Map and available VMap geometry for every admitted active map, including empty maps; verification independently derives that domain from sealed Map. GODI/global GameObjects, LiquidObject, terrain, MMaps and full-world/runtime claims remain excluded."
                );
                println!(
                    "Linux v30 publishes Map and available VMap geometry while declaring only proven root-absent WDT omissions. Missing WDTs receive no invented artifacts; root absence is producer-reported, and full-world, complete-component and runtime claims remain excluded."
                );
                println!(
                    "Linux v31 separately declares only WMO-doodad M2-by-FDID raw-ROOT absences and excludes those children and their derived spawns while retaining the parent WMO. Direct M2 and other missing sources remain fatal; omissions are producer-reported and complete-component, full-world and runtime claims remain false."
                );
                println!(
                    "Linux v32 preserves the v31 artifact and omission policies with a separately versioned resident request capacity. It does not relax source, dependency, model, byte or memory limits, and complete-component, full-world and runtime claims remain false."
                );
                println!(
                    "Linux v33 separately declares only WMO roots left with no geometry and no usable doodad selection after declared M2 ROOT absences. Those WMOs receive no VMO or placement IDs; all other NoCollision cases remain fatal and complete-component, full-world and runtime claims remain false."
                );
                println!(
                    "Linux v44 uses SourceExpandedV2 for build-70450 Map/VMap geometry: source reads are capped at 2 GiB, staged model output at 512 MiB, model identities at 65,536 and reads at 1,000,000. Coverage and WMO omissions are independently checked only within the declared available domain; no full-world, complete-component, runtime or parity claim is made."
                );
                println!(
                    "Linux v45 uses PlacementExpandedV1 with the v18 coverage identity: it preserves v44 acquisition/model limits, raises only global staged placements to 256 MiB, and keeps each map capped at 64 MiB. Coverage and omissions are independently checked within the declared available domain; no full-world, complete-component, runtime or parity claim is made."
                );
                println!(
                    "Linux v46 uses AssemblyExpandedV1 and RCVGEO1 v19: it preserves the v45 resource limits and adds an independent 256 MiB assembled-domain guard. The prior generic v45 failure and 128 MiB internal cap do not confirm cause or sufficiency; no real target acceptance, full-world, complete-component, runtime or parity claim is made."
                );
                println!(
                    "forever-extractor verify-trinity-compat --terrain-package V51_PACKAGE --terrain-plan V51_PLAN --navigation-package V57_PACKAGE --navigation-plan V57_PLAN (Linux only; verifies compatible-partial omissions without reextraction)"
                );
                println!(
                    "forever-extractor census-db2 --plan PLAN --client CLIENT --output NEW_REPORT (Linux only; build 70450 wowdbdefs-70450-v1, revisions 16+; --keys is forbidden)"
                );
                println!(
                    "forever-extractor inspect-db2 --plan PLAN --client CLIENT --output NEW_REPORT --file-data-id FDID [--file-data-id FDID ...] (Linux only; build 70450; extracts nothing; --keys is forbidden)"
                );
            } else {
                println!("{json}");
            }
        }
        Err(error) => {
            eprintln!(
                "{{\"contract\":\"{}\",\"status\":\"failed\",\"code\":\"{}\",\"component\":{},\"full_extractor_complete\":false}}",
                contract,
                error.code,
                error
                    .component
                    .map_or("null".into(), |c| format!("\"{}\"", c.name()))
            );
            std::process::exit(if error.code.contains("incomplete") {
                8
            } else if error.code.starts_with("product_plan_")
                || error.code == "product_usage"
                || error.code == "product_platform"
                || error.code == "product_keys_required"
                || error.code == "product_keys_forbidden"
                || error.code == "product_probe_profile"
                || error.code == "product_probe_file_data_id"
                || error.code == "product_census_limit"
                || error.code.starts_with("product_db2_inspection_")
                || error.code == "product_scratch_required"
                || error.code == "product_scratch_forbidden"
                || error.code == "product_disk_unsupported"
                || error.code == "product_navigation_inputs_required"
                || error.code == "product_navigation_inputs_forbidden"
                || error.code == "product_v23_client_inspect_unsupported"
            {
                2
            } else {
                9
            });
        }
    }
}
fn run(contract: &mut &'static str) -> Result<String, Error> {
    let args: Vec<_> = std::env::args_os().skip(1).take(138).collect();
    let Some(command) = args.first().and_then(|a| a.to_str()) else {
        return Err("product_usage".into());
    };
    if command == "--help" && args.len() == 1 {
        return Ok("forever-extractor inspect --plan PLAN [--client CLIENT [--keys KEYS]]\nforever-extractor extract|resume --plan PLAN --client CLIENT --output NEW_OR_INCOMPLETE_OUTPUT [--keys KEYS] [--scratch PRIVATE_DIRECTORY]\nforever-extractor verify --plan PLAN --output OUTPUT [--expect-context CONTEXT]\nforever-extractor verify-links --plan PLAN --output OUTPUT [--expect-context CONTEXT] (Linux only)\nOffline Rust-local only; canonical TSV v1 N/Z, v2 provided-salsa20-v1, v3 leased disk-world collision, v4 build-70205 metadata, v5 build-70338 metadata, v6 pinned-metadata inline off-mesh navigation, Linux v8 selected map and ancestors, Linux v9 build-70338 exact DB2+GT+cameras, Linux v10 build-70374 DB2-only, Linux v11 build-70374 exact DB2+GT+cameras, or Linux v12 build-70374 available-Map terrain-only full map/all-available-maps, disabled keys; Linux v13 separately records unavailable WDT/root ADT omissions (MissingFile only), with coverage v2 and unchanged fatal integrity/format/resource policies (v7 reserved). Supplied-key v2-v6/v8-v11 client commands require --keys; disabled plans forbid it; plan-only inspection and verification need no keys. V17 and v18 client inspect/extract/resume also require explicit --keys. V3-v6/v8 extract/resume with leased disk storage require --scratch; resident plans forbid it. Inspection and verification take no scratch. Legacy Linux and Windows resident product IO; v9-v11 require Linux; leased disk storage requires Linux. Windows client commands require the actual loaded sealed PE. Verification is bounded artifact/intrinsic-format scope; verify-links additionally checks published terrain tile sets and navigation tile/global presence. V8 navigation builds each map's own candidates across the selected map and ancestor dependencies; maps without own candidates emit no MMaps. V8 verification enforces that whole domain and its terrain/navigation links; global GameObject startup remains required. Linux v10 is explicit Rust-local build-70374 DB2-only with wowdbdefs-70374-v1; disabled/provided-salsa20-v1 follow the same explicit key rules. Native, other components and Windows v10 execution reject. Linux v11 is explicit Rust-local build-70374 exact DB2+GT+cameras with wowdbdefs-70374-v1 and the same key policies and limits; Windows/native/geometry remain unsupported. Linux v14 separately selects disabled-key70425 collision-only GNU15-v2 leased disk, genuine available-source coverage and complete declared VMap association verification; extract/resume require fresh private scratch and replay every final artifact. No runtime/full-client claim. Linux v16 separately selects disabled-key 70450 DB2-only using wowdbdefs-70450-v1, resident storage and the strict DB2 read route; encrypted sources remain fatal. Linux v17 separately selects build-70450 DB2-only with wowdbdefs-70450-v1 and provided-salsa20-v1; client inspect, extract and resume require explicit --keys. Linux v18 separately selects build-70450 compatible GNU15-v2 collision-only with provided-salsa20-v1; client inspect, extract and resume require --keys, and extract/resume also require --scratch. Linux v19 separately selects build-70450 exact DB2+GT+cameras with wowdbdefs-70450-v1 and disabled decryption; client inspect/extract/resume reject --keys, and resident storage forbids --scratch.".into());
    }
    #[cfg(not(target_os = "linux"))]
    if command == "verify-links" {
        return Err("product_platform".into());
    }
    let max_args = if matches!(command, "probe-encoding" | "inspect-db2") {
        137
    } else {
        16
    };
    if ![
        "inspect",
        "extract",
        "resume",
        "verify",
        "verify-links",
        "verify-trinity-compat",
        "probe-encoding",
        "census-db2",
        "inspect-db2",
    ]
    .contains(&command)
        || args.len() > max_args
        || args.len() % 2 != 1
    {
        return Err("product_usage".into());
    }
    let allowed = match command {
        "inspect" => &["--plan", "--client", "--keys"][..],
        "verify" => &["--plan", "--output", "--expect-context"][..],
        "verify-trinity-compat" => &[
            "--terrain-package",
            "--terrain-plan",
            "--navigation-package",
            "--navigation-plan",
        ][..],
        "verify-links" => &[
            "--plan",
            "--output",
            "--expect-context",
            "--terrain-package",
            "--terrain-plan",
            "--vmap-package",
            "--vmap-plan",
        ][..],
        "probe-encoding" | "inspect-db2" => {
            &["--plan", "--client", "--output", "--keys", "--file-data-id"][..]
        }
        "census-db2" => &["--plan", "--client", "--output", "--keys"][..],
        _ => &[
            "--plan",
            "--client",
            "--output",
            "--keys",
            "--scratch",
            "--terrain-package",
            "--terrain-plan",
            "--vmap-package",
            "--vmap-plan",
        ][..],
    };
    let mut options = BTreeMap::new();
    let mut file_data_ids = Vec::new();
    for pair in args[1..].as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or("product_usage")?;
        if matches!(command, "probe-encoding" | "inspect-db2") && key == "--file-data-id" {
            let error = if command == "inspect-db2" {
                "product_db2_inspection_file_data_id"
            } else {
                "product_probe_file_data_id"
            };
            if file_data_ids.len() == 64 {
                return Err(error.into());
            }
            let value = pair[1].to_str().ok_or(error)?;
            file_data_ids.push(parse_file_data_id(value).map_err(|_| Error::from(error))?);
            continue;
        }
        if !allowed.contains(&key)
            || pair[1].is_empty()
            || options.insert(key, PathBuf::from(&pair[1])).is_some()
        {
            return Err("product_usage".into());
        }
    }
    let path = |key| {
        options
            .get(key)
            .map(PathBuf::as_path)
            .ok_or_else(|| Error::from("product_usage"))
    };
    if command == "verify-trinity-compat" {
        *contract = "forever-trinity-compat-v1";
        #[cfg(target_os = "linux")]
        return trinity_compat_command(&options);
        #[cfg(not(target_os = "linux"))]
        return Err("product_platform".into());
    }
    if matches!(command, "probe-encoding" | "inspect-db2") {
        let error = if command == "inspect-db2" {
            "product_db2_inspection_file_data_id"
        } else {
            "product_probe_file_data_id"
        };
        if file_data_ids.is_empty() {
            return Err(error.into());
        }
        file_data_ids.sort_unstable();
        if file_data_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(error.into());
        }
    }
    let bytes = input::bytes(path("--plan")?, forever_extractor::MAX_PLAN_BYTES)?;
    let plan =
        Plan::parse(std::str::from_utf8(&bytes).map_err(|_| Error::from("product_plan_format"))?)?;
    *contract = plan.contract();
    if plan.is_v52()
        || plan.is_v53()
        || plan.is_v54()
        || plan.is_v55()
        || plan.is_v56()
        || plan.is_v57()
    {
        return available_navigation_command(command, &plan, &options);
    }
    #[cfg(not(target_os = "linux"))]
    if (plan.is_v49() || plan.is_v50() || plan.is_v51()) && command != "inspect" {
        return Err("product_platform".into());
    }
    if (plan.is_v49() || plan.is_v50() || plan.is_v51())
        && command == "inspect"
        && options.contains_key("--client")
    {
        return Err("product_platform".into());
    }
    #[cfg(not(target_os = "linux"))]
    if plan.is_v48() && command != "inspect" {
        return Err("product_platform".into());
    }
    if plan.is_v48() && command == "inspect" && options.contains_key("--client") {
        return Err("product_platform".into());
    }
    #[cfg(not(target_os = "linux"))]
    if plan.is_v47() && command != "inspect" {
        return Err("product_platform".into());
    }
    if plan.is_v47() && command == "inspect" && options.contains_key("--client") {
        return Err("product_platform".into());
    }
    validate_selected_navigation_cli(command, &plan, &options)?;
    #[cfg(not(target_os = "linux"))]
    if plan.is_v23() && command != "inspect" {
        return Err("product_platform".into());
    }
    if matches!(command, "probe-encoding" | "census-db2" | "inspect-db2")
        && options.contains_key("--keys")
    {
        return Err("product_keys_forbidden".into());
    }
    if matches!(
        command,
        "extract" | "resume" | "probe-encoding" | "census-db2" | "inspect-db2"
    ) {
        path("--client")?;
        path("--output")?;
    } else if matches!(command, "verify" | "verify-links") {
        path("--output")?;
    }
    if matches!(command, "extract" | "resume") {
        match (plan.collision_storage, options.contains_key("--scratch")) {
            (CollisionStorage::Resident, true) => return Err("product_scratch_forbidden".into()),
            (CollisionStorage::LeasedDiskV1(_), false) => {
                return Err("product_scratch_required".into());
            }
            _ => {}
        }
    }
    if options.contains_key("--keys") && !plan.requires_keys() {
        return Err("product_keys_forbidden".into());
    }
    if plan.requires_keys()
        && !matches!(command, "probe-encoding" | "census-db2" | "inspect-db2")
        && !matches!(command, "verify" | "verify-links")
        && (command != "inspect" || options.contains_key("--client"))
        && !options.contains_key("--keys")
    {
        return Err("product_keys_required".into());
    }
    let decryption_capability = if plan.requires_keys() {
        ",\"decryption\":\"provided-salsa20-v1\""
    } else if plan.version() >= 3 {
        ",\"decryption\":\"disabled\""
    } else {
        ""
    };
    let collision_storage_capability = match plan.collision_storage {
        CollisionStorage::Resident => "",
        CollisionStorage::LeasedDiskV1(_) => {
            ",\"collision_storage\":\"linux-leased-append-disk-world-v1\""
        }
    };
    let schema_capability = match plan.schema_catalog {
        forever_extractor::plan::SchemaCatalog::PinnedMetadataV1 => String::new(),
        forever_extractor::plan::SchemaCatalog::WowDbDefs70205V1 => {
            format!(",\"schema_catalog\":\"{}\"", plan.schema_catalog.id())
        }
        forever_extractor::plan::SchemaCatalog::WowDbDefs70338V1
        | forever_extractor::plan::SchemaCatalog::WowDbDefs70374V1
        | forever_extractor::plan::SchemaCatalog::WowDbDefs70425V1
        | forever_extractor::plan::SchemaCatalog::WowDbDefs70450V1 => {
            format!(",\"schema_catalog\":\"{}\"", plan.schema_catalog.id())
        }
    };
    let offmesh_capability = plan
        .offmesh
        .as_ref()
        .map_or_else(String::new, |connections| {
            format!(
                ",\"offmesh\":\"{}\",\"offmesh_connections\":{}",
                forever_extractor::plan::OFFMESH_PROFILE,
                connections.len()
            )
        });
    let map_domain_capability = if plan.is_v50() {
        ",\"collision_domain\":\"all-available-maps\",\"gameobject_scope\":\"source-only-v1\",\"model_scope\":\"source-only-v1\"".to_owned()
    } else if plan.is_v49() {
        ",\"collision_domain\":\"all-available-maps\",\"gameobject_scope\":\"available-fdid-direct-root-absent-or-encrypted-unavailable-v1\",\"model_scope\":\"placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1\"".to_owned()
    } else if plan.is_v48() {
        ",\"collision_domain\":\"all-available-maps\",\"gameobject_scope\":\"available-fdid-direct-missing-root-v1\",\"model_scope\":\"placement-and-gameobject-with-missing-root-v1\"".to_owned()
    } else if plan.is_v47() {
        ",\"collision_domain\":\"all-available-maps\",\"gameobject_scope\":\"available-fdid-direct-v1\",\"model_scope\":\"placement-and-gameobject-v1\"".to_owned()
    } else if plan.is_v46()
        || plan.is_v45()
        || plan.is_v44()
        || plan.is_v29()
        || plan.is_v30()
        || plan.is_v31()
        || plan.is_v32()
        || plan.is_v33()
        || plan.is_v34()
        || plan.is_v35()
        || plan.is_v36()
        || plan.is_v37()
        || plan.is_v38()
        || plan.is_v39()
        || plan.is_v40()
        || plan.is_v41()
        || plan.is_v43()
        || plan.is_v42()
    {
        ",\"collision_domain\":\"all-available-maps\",\"gameobject_scope\":\"excluded_by_profile\",\"model_scope\":\"placement_only\"".to_owned()
    } else if plan.has_selected_geometry() {
        format!(
            ",\"collision_domain\":\"selected-map-and-active-ancestors\",\"requested_map\":{},\"gameobject_scope\":\"excluded_by_profile\",\"model_scope\":\"placement_only\"",
            plan.map_domain.ok_or("product_plan_map_domain")?
        )
    } else if plan.is_v14() || plan.is_v15() || plan.is_v18() {
        plan.map_domain.map_or_else(
            || ",\"collision_domain\":\"all-available-maps\"".to_owned(),
            |id| format!(",\"collision_domain\":\"map-and-ancestors\",\"requested_map\":{id}"),
        )
    } else {
        plan.map_domain.map_or_else(String::new, |id| {
        format!(
            ",\"map_domain\":\"{}\",\"requested_map\":{id},\"gameobject_scope\":\"{}\",\"placement_ids\":\"{}\"",
            forever_extractor::plan::MAP_DOMAIN_PROFILE,
            forever_extractor::plan::GAMEOBJECT_SCOPE_PROFILE,
            forever_extractor::plan::PLACEMENT_ID_DOMAIN_PROFILE,
        )
    })
    };
    let gameobject_expanded_capability = gameobject_expanded_resource_capability();
    let assembly_expanded_capability = assembly_expanded_resource_capability();
    let placement_expanded_capability = placement_expanded_resource_capability();
    let gameobject_missing_root_capability = gameobject_missing_root_resource_capability();
    let gameobject_root_encrypted_unavailable_capability =
        gameobject_root_encrypted_unavailable_resource_capability();
    let liquid_object_complete_source_capability =
        liquid_object_complete_source_resource_capability();
    let terrain_capability: &str = if plan.is_v50() {
        &liquid_object_complete_source_capability
    } else if plan.is_v49() {
        &gameobject_root_encrypted_unavailable_capability
    } else if plan.is_v48() {
        &gameobject_missing_root_capability
    } else if plan.is_v47() {
        &gameobject_expanded_capability
    } else if plan.is_v46() {
        &assembly_expanded_capability
    } else if plan.is_v45() {
        &placement_expanded_capability
    } else if plan.is_v44() {
        ",\"source_bytes_max\":2147483648,\"model_bytes_max\":536870912,\"models_max\":65536,\"reads_max\":1000000,\"resource_profile\":\"linux-collision-source-expanded-resources-v2\",\"collision_source_profile\":\"linux70450-source-expanded-v2-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v43() {
        ",\"source_bytes_max\":1073741824,\"model_bytes_max\":536870912,\"models_max\":65536,\"reads_max\":1000000,\"resource_profile\":\"linux-collision-model-bytes-expanded-resources-v1\",\"collision_source_profile\":\"linux70450-model-bytes-expanded-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v42() {
        ",\"source_bytes_max\":1073741824,\"models_max\":65536,\"reads_max\":1000000,\"resource_profile\":\"linux-collision-model-output-expanded-resources-v1\",\"collision_source_profile\":\"linux70450-model-output-expanded-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-model-output-expanded-material-filtered-wmo-coverage-v15\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v41() {
        ",\"source_bytes_max\":1073741824,\"models_max\":65536,\"reads_max\":1000000,\"resource_profile\":\"linux-collision-source-expanded-resources-v1\",\"collision_source_profile\":\"linux70450-source-expanded-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-source-expanded-material-filtered-wmo-coverage-v14\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v40() {
        ",\"models_max\":65536,\"reads_max\":1000000,\"resource_profile\":\"linux-collision-model-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-model-heavy-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-model-heavy-material-filtered-wmo-coverage-v13\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v39() {
        ",\"reads_max\":1000000,\"resource_profile\":\"linux-collision-read-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-read-heavy-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-read-heavy-material-filtered-wmo-coverage-v12\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v38() {
        ",\"resource_profile\":\"linux-collision-source-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-source-heavy-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-source-heavy-material-filtered-wmo-coverage-v11\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v37() {
        ",\"resource_profile\":\"linux-collision-dependency-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-dependency-heavy-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-dependency-heavy-material-filtered-wmo-coverage-v10\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v36() {
        ",\"resource_profile\":\"linux-collision-request-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-material-filtered-wmo-coverage-v9\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v35() {
        ",\"resource_profile\":\"linux-collision-request-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-available-unselected-wmo-v1\",\"coverage_profile\":\"linux70450-available-unselected-wmo-coverage-v8\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v34() {
        ",\"resource_profile\":\"linux-collision-request-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-available-empty-wmo-v1\",\"coverage_profile\":\"linux70450-available-empty-wmo-coverage-v7\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v33() {
        ",\"resource_profile\":\"linux-collision-request-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-available-vacated-wmo-v1\",\"coverage_profile\":\"linux70450-available-vacated-wmo-coverage-v6\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v32() {
        ",\"resource_profile\":\"linux-collision-request-heavy-resources-v1\",\"collision_source_profile\":\"linux70450-available-map-model-root-omissions-request-capacity-v1\",\"coverage_profile\":\"linux70450-available-map-model-root-omissions-request-capacity-coverage-v5\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v31() {
        ",\"collision_source_profile\":\"linux70450-available-map-model-root-omissions-v1\",\"coverage_profile\":\"linux70450-available-map-model-root-omissions-coverage-v4\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v30() {
        ",\"collision_source_profile\":\"linux70450-available-map-root-omissions-v1\",\"coverage_profile\":\"linux70450-available-map-root-omissions-coverage-v3\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v29() {
        ",\"collision_source_profile\":\"linux70450-available-map-geometry-v1\",\"coverage_profile\":\"linux70450-available-map-geometry-coverage-v2\",\"coverage_authentication\":\"independently_derived\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v28() {
        ",\"collision_source_profile\":\"linux70450-authenticated-selected-map-geometry-v1\",\"coverage_profile\":\"linux70450-selected-map-geometry-coverage-v1\",\"coverage_authentication\":\"independently_derived\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
    } else if plan.is_v22() {
        ",\"collision_source_profile\":\"linux70450-selected-map-geometry-v1\",\"coverage_profile\":\"linux70450-selected-map-geometry-coverage-v1\",\"coverage_authentication\":\"producer_reported\",\"map_source_scope\":\"available_sections\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false"
    } else if plan.is_v27() {
        ",\"terrain_source_profile\":\"linux70450-partial-catalog-authenticated-map-terrain-v1\",\"verification_coverage_authentication\":\"independently_derived\",\"partial_encrypted_sections\":\"declared-skip\",\"partial_db2_declarations\":\"per-table-including-Map\",\"whole_client_map_coverage\":false"
    } else if plan.is_v24() {
        ",\"terrain_source_profile\":\"linux70450-authenticated-available-map-terrain-v1\",\"verification_coverage_authentication\":\"independently_derived\",\"db2_output\":[\"Map\"],\"whole_client_map_coverage\":false"
    } else if plan.is_v25() {
        ",\"terrain_source_profile\":\"linux70450-authenticated-available-map-terrain-partial-map-v1\",\"verification_coverage_authentication\":\"independently_derived\",\"db2_output\":[\"Map\"],\"partial_encrypted_sections\":\"declared-skip\",\"whole_client_map_coverage\":false"
    } else if plan.is_v20() {
        ",\"terrain_source_profile\":\"linux70450-compatible-full-maps-available-map-v1\",\"coverage_authentication\":\"producer_reported\",\"omission_policy\":\"missing-file-only\",\"whole_client_map_coverage\":false"
    } else if plan.is_v15() || plan.is_v18() {
        ",\"collision_source_profile\":\"linux70450-compatible-gnu15-v2\",\"coverage_authentication\":\"producer_reported\",\"whole_client_map_coverage\":false"
    } else if plan.is_v14() {
        ",\"collision_source_profile\":\"linux70425-compatible-gnu15-v2\",\"coverage_authentication\":\"producer_reported\",\"whole_client_map_coverage\":false"
    } else if plan.is_v13() {
        ",\"terrain_source_profile\":\"linux70374-compatible-full-maps-available-map-v1\",\"coverage_authentication\":\"producer_reported\",\"omission_policy\":\"missing-file-only\",\"whole_client_map_coverage\":false"
    } else if plan.is_v12() {
        ",\"terrain_source_profile\":\"linux70374-full-maps-available-map-v1\",\"coverage_authentication\":\"producer_reported\",\"whole_client_map_coverage\":false"
    } else {
        ""
    };
    let navigation_capability = if plan.is_v23() {
        ",\"navigation_source_profile\":\"linux70450-selected-navigation-v1\",\"navigation_inputs\":\"sealed-v20-terrain-and-v22-vmap\",\"input_coverage\":\"producer_reported\",\"complete_navigation_input_claim\":false"
    } else if plan.is_v21() {
        ",\"navigation_source_profile\":\"linux70450-map-domain-navigation-v1\",\"complete_navigation_input_claim\":false"
    } else {
        ""
    };
    let capability = if plan.is_v51() {
        use forever_extractor::plan::*;
        let mut combined = liquid_object_complete_source_capability.to_string();
        for (old, new) in [
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE,
                DB2_TERRAIN_LIQUID_VMAP_PRODUCT_PROFILE,
            ),
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE,
                DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE,
            ),
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE,
                DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE,
            ),
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_DOMAIN_PROFILE,
                DB2_TERRAIN_LIQUID_VMAP_DOMAIN_PROFILE,
            ),
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE,
                DB2_TERRAIN_LIQUID_VMAP_VERIFY_PROFILE,
            ),
            (
                LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE,
                DB2_TERRAIN_LIQUID_VMAP_VERIFY_SCOPE,
            ),
            ("per-table-including-Map", "GameObjectDisplayInfo+Map-only"),
        ] {
            combined = combined.replace(old, new);
        }
        format!(
            "{schema_capability}{decryption_capability}{collision_storage_capability}{combined},\"terrain_source_profile\":\"linux70450-authenticated-available-map-terrain-partial-map-v1\",\"terrain_selection\":\"all-available-maps\",\"completion\":\"{DB2_TERRAIN_LIQUID_VMAP_COMPLETION_PROFILE}\",\"collision_domain\":\"all-available-maps\",\"whole_client_map_coverage\":false"
        )
    } else {
        format!(
            "{schema_capability}{decryption_capability}{collision_storage_capability}{offmesh_capability}{map_domain_capability}{terrain_capability}{navigation_capability}"
        )
    };
    if command == "inspect" && !options.contains_key("--client") {
        if options.contains_key("--keys") {
            return Err("product_usage".into());
        }
        return Ok(format!(
            "{{\"contract\":\"{}\",\"status\":\"planned\",\"build\":{},\"locale\":\"{}\",\"components\":{},\"db2_files\":{},\"plan_sha256\":\"{}\",\"offline\":true{capability},\"full_extractor_complete\":false}}",
            plan.contract(),
            plan.build,
            plan.locale.name(),
            plan.components,
            plan.db2.len(),
            integrity::hex(&integrity::digest(&bytes))
        ));
    }
    #[cfg(any(target_os = "linux", windows))]
    {
        use forever_extractor::{driver, verify};
        driver::admit(&plan)?;
        if matches!(command, "verify" | "verify-links") {
            let expected = if let Some(p) = options.get("--expect-context") {
                let b = input::bytes(p, 1024)?;
                Some(Context::parse(
                    std::str::from_utf8(&b).map_err(|_| Error::from("product_context"))?,
                )?)
            } else {
                None
            };
            #[cfg(target_os = "linux")]
            if command == "verify-links" {
                if plan.is_v23() {
                    let inputs = driver::SelectedNavigationInputs {
                        terrain_package: path("--terrain-package")?,
                        terrain_plan: path("--terrain-plan")?,
                        vmap_package: path("--vmap-package")?,
                        vmap_plan: path("--vmap-plan")?,
                    };
                    let summary = verify::verify_selected_navigation_links(
                        &plan,
                        path("--output")?,
                        expected,
                        &inputs,
                    )?;
                    return Ok(format!(
                        "{{\"contract\":\"{}\",\"scope\":\"selected_navigation_linked_package_identities\",\"verification_profile\":\"linux70450-selected-navigation-links-v1\",\"linked_inputs\":\"offline_verified\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        plan.contract(),
                        summary.json()
                    ));
                }
                let summary = verify::verify_links(&plan, path("--output")?, expected)?;
                if plan.is_v51() {
                    return Ok(format!(
                        "{{\"contract\":\"forever-product-v51\",\"scope\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        forever_extractor::plan::DB2_TERRAIN_LIQUID_VMAP_VERIFY_SCOPE,
                        summary.json()
                    ));
                }
                if plan.is_v50() {
                    let capability = capability.replace(
                        ",\"verification_profile\":\"linux70450-liquid-object-complete-source-verification-v1\"",
                        "",
                    );
                    return Ok(format!(
                        "{{\"contract\":\"forever-product-v50\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE,
                        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE,
                        summary.json()
                    ));
                }
                if plan.is_v49() {
                    let capability = capability.replace(
                        ",\"verification_profile\":\"linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1\"",
                        "",
                    );
                    return Ok(format!(
                        "{{\"contract\":\"forever-product-v49\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                        forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                        summary.json()
                    ));
                }
                if plan.is_v48() {
                    let capability = capability.replace(
                        ",\"verification_profile\":\"linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1\"",
                        "",
                    );
                    return Ok(format!(
                        "{{\"contract\":\"forever-product-v48\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        forever_extractor::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                        forever_extractor::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                        summary.json()
                    ));
                }
                if plan.is_v47() {
                    let capability = capability.replace(
                        ",\"verification_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-verification-v1\"",
                        "",
                    );
                    return Ok(format!(
                        "{{\"contract\":\"forever-product-v47\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                        forever_extractor::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                        forever_extractor::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                        summary.json()
                    ));
                }
                let authentication = if plan.has_authenticated_map_terrain() {
                    ",\"coverage_authentication\":\"independently_derived\""
                } else {
                    ""
                };
                let client_reads = if plan.is_v47() || plan.is_v49() || plan.is_v50() {
                    ",\"client_reads\":false"
                } else {
                    ""
                };
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\"{authentication}{client_reads}{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    if plan.is_v15() || plan.is_v14() || plan.is_v18() {
                        forever_extractor::plan::COMPATIBLE_COLLISION_VERIFY_SCOPE
                    } else if plan.is_v50() {
                        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE
                    } else if plan.is_v49() {
                        forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v47() {
                        forever_extractor::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v46() {
                        forever_extractor::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v45() {
                        forever_extractor::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v44() {
                        forever_extractor::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v43() {
                        forever_extractor::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v42() {
                        forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v41() {
                        forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v40() {
                        forever_extractor::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v39() {
                        forever_extractor::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v38() {
                        forever_extractor::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v37() {
                        forever_extractor::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v36() {
                        forever_extractor::plan::MATERIAL_FILTERED_WMO_VERIFY_SCOPE
                    } else if plan.is_v35() {
                        forever_extractor::plan::UNSELECTED_WMO_VERIFY_SCOPE
                    } else if plan.is_v34() {
                        forever_extractor::plan::EMPTY_WMO_VERIFY_SCOPE
                    } else if plan.is_v33() {
                        forever_extractor::plan::VACATED_WMO_VERIFY_SCOPE
                    } else if plan.is_v32() {
                        forever_extractor::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v31() {
                        forever_extractor::plan::MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v30() {
                        forever_extractor::plan::ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v29() {
                        forever_extractor::plan::AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v28() {
                        forever_extractor::plan::AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v22() {
                        forever_extractor::plan::SELECTED_GEOMETRY_VERIFY_SCOPE
                    } else if plan.is_v27() {
                        forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_SCOPE
                    } else if plan.is_v24() {
                        forever_extractor::plan::AUTHENTICATED_TERRAIN_VERIFY_SCOPE
                    } else if plan.is_v25() {
                        forever_extractor::plan::AUTHENTICATED_PARTIAL_MAP_VERIFY_SCOPE
                    } else if plan.is_v20() {
                        forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_SCOPE_70450
                    } else if plan.is_v13() {
                        forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_SCOPE
                    } else if plan.is_v12() {
                        forever_extractor::plan::AVAILABLE_TERRAIN_VERIFY_SCOPE
                    } else if plan.map_domain.is_some() {
                        forever_extractor::plan::MAP_DOMAIN_VERIFY_SCOPE
                    } else {
                        verify::links::SCOPE
                    },
                    if plan.is_v15() || plan.is_v18() {
                        forever_extractor::plan::COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
                    } else if plan.is_v50() {
                        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE
                    } else if plan.is_v49() {
                        forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v47() {
                        forever_extractor::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v46() {
                        forever_extractor::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v45() {
                        forever_extractor::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v44() {
                        forever_extractor::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v43() {
                        forever_extractor::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v42() {
                        forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v41() {
                        forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v40() {
                        forever_extractor::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v39() {
                        forever_extractor::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v38() {
                        forever_extractor::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v37() {
                        forever_extractor::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v36() {
                        forever_extractor::plan::MATERIAL_FILTERED_WMO_VERIFY_PROFILE
                    } else if plan.is_v35() {
                        forever_extractor::plan::UNSELECTED_WMO_VERIFY_PROFILE
                    } else if plan.is_v34() {
                        forever_extractor::plan::EMPTY_WMO_VERIFY_PROFILE
                    } else if plan.is_v33() {
                        forever_extractor::plan::VACATED_WMO_VERIFY_PROFILE
                    } else if plan.is_v32() {
                        forever_extractor::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v31() {
                        forever_extractor::plan::MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v30() {
                        forever_extractor::plan::ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v29() {
                        forever_extractor::plan::AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v28() {
                        forever_extractor::plan::AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v22() {
                        forever_extractor::plan::SELECTED_GEOMETRY_VERIFY_PROFILE
                    } else if plan.is_v27() {
                        forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_PROFILE
                    } else if plan.is_v24() {
                        forever_extractor::plan::AUTHENTICATED_TERRAIN_VERIFY_PROFILE
                    } else if plan.is_v25() {
                        forever_extractor::plan::AUTHENTICATED_PARTIAL_MAP_VERIFY_PROFILE
                    } else if plan.is_v14() {
                        forever_extractor::plan::COMPATIBLE_COLLISION_VERIFY_PROFILE
                    } else if plan.is_v20() {
                        forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_PROFILE_70450
                    } else if plan.is_v13() {
                        forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_PROFILE
                    } else if plan.is_v12() {
                        forever_extractor::plan::AVAILABLE_TERRAIN_VERIFY_PROFILE
                    } else if plan.map_domain.is_some() {
                        forever_extractor::plan::MAP_DOMAIN_VERIFICATION_PROFILE
                    } else {
                        verify::links::PROFILE
                    },
                    summary.json()
                ));
            }
            let summary = verify::verify(&plan, path("--output")?, expected)?;
            if plan.is_v51() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v51\",\"scope\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::DB2_TERRAIN_LIQUID_VMAP_VERIFY_SCOPE,
                    summary.json()
                ));
            }
            if plan.is_v50() {
                let capability = capability.replace(
                    ",\"verification_profile\":\"linux70450-liquid-object-complete-source-verification-v1\"",
                    "",
                );
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v50\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE,
                    forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v49() {
                let capability = capability.replace(
                    ",\"verification_profile\":\"linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1\"",
                    "",
                );
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v49\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v48() {
                let capability = capability.replace(
                    ",\"verification_profile\":\"linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1\"",
                    "",
                );
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v48\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v47() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v47\",\"scope\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    summary.json()
                ));
            }
            if plan.is_v46() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v46\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v45() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v45\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v44() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v44\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v43() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v43\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v23() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"selected_navigation_package_integrity_and_stored_provenance\",\"verification_profile\":\"linux70450-selected-navigation-v1\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    summary.json()
                ));
            }
            if plan.is_v42() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v42\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v41() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v41\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v40() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v40\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v39() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v39\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v38() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v38\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v37() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v37\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v36() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v36\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v35() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v35\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::UNSELECTED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::UNSELECTED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v34() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v34\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::EMPTY_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::EMPTY_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v30() || plan.is_v31() || plan.is_v32() || plan.is_v33() {
                let (scope, profile) = if plan.is_v33() {
                    (
                        forever_extractor::plan::VACATED_WMO_VERIFY_SCOPE,
                        forever_extractor::plan::VACATED_WMO_VERIFY_PROFILE,
                    )
                } else if plan.is_v32() {
                    (
                        forever_extractor::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_SCOPE,
                        forever_extractor::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_PROFILE,
                    )
                } else if plan.is_v31() {
                    (
                        forever_extractor::plan::MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE,
                        forever_extractor::plan::MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE,
                    )
                } else {
                    (
                        forever_extractor::plan::ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE,
                        forever_extractor::plan::ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE,
                    )
                };
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    scope,
                    profile,
                    summary.json()
                ));
            }
            if plan.is_v29() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    forever_extractor::plan::AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_SCOPE,
                    forever_extractor::plan::AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v28() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    forever_extractor::plan::AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_SCOPE,
                    forever_extractor::plan::AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v27() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_SCOPE,
                    forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v24() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    forever_extractor::plan::AUTHENTICATED_TERRAIN_VERIFY_SCOPE,
                    forever_extractor::plan::AUTHENTICATED_TERRAIN_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v25() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false,\"coverage_authentication\":\"independently_derived\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    forever_extractor::plan::AUTHENTICATED_PARTIAL_MAP_VERIFY_SCOPE,
                    forever_extractor::plan::AUTHENTICATED_PARTIAL_MAP_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v42() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v42\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            if plan.is_v41() {
                return Ok(format!(
                    "{{\"contract\":\"forever-product-v41\",\"scope\":\"{}\",\"verification_profile\":\"{}\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
                    forever_extractor::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
                    summary.json()
                ));
            }
            let scope = if plan.is_v15() || plan.is_v14() || plan.is_v18() {
                forever_extractor::plan::COMPATIBLE_COLLISION_VERIFY_SCOPE
            } else if plan.is_v22() {
                forever_extractor::plan::SELECTED_GEOMETRY_VERIFY_SCOPE
            } else if plan.is_v20() {
                forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_SCOPE_70450
            } else if plan.is_v13() {
                forever_extractor::plan::COMPATIBLE_TERRAIN_VERIFY_SCOPE
            } else if plan.is_v12() {
                forever_extractor::plan::AVAILABLE_TERRAIN_VERIFY_SCOPE
            } else if plan.map_domain.is_some() {
                forever_extractor::plan::MAP_DOMAIN_VERIFY_SCOPE
            } else {
                "artifact_integrity_and_intrinsic_formats"
            };
            return Ok(format!(
                "{{\"contract\":\"{}\",\"scope\":\"{scope}\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                plan.contract(),
                summary.json()
            ));
        }
        if command == "probe-encoding" {
            #[cfg(target_os = "linux")]
            {
                let (count, trace_sha256) = driver::probe_encoding_command(
                    &plan,
                    path("--client")?,
                    path("--output")?,
                    &file_data_ids,
                )?;
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"status\":\"probe_complete\",\"file_data_ids\":{count},\"trace_sha256\":\"{}\",\"offline\":true,\"full_extractor_complete\":false}}",
                    plan.contract(),
                    integrity::hex(&trace_sha256),
                ));
            }
            #[cfg(not(target_os = "linux"))]
            return Err("product_platform".into());
        }
        if command == "census-db2" {
            #[cfg(target_os = "linux")]
            {
                let declaration_count =
                    driver::census_db2_command(&plan, path("--client")?, path("--output")?)?;
                return Ok(format!(
                    "{{\"contract\":\"forever-db2-census-contract-v1\",\"status\":\"census_written\",\"catalogue\":\"wowdbdefs-70450-v1\",\"declarations\":{declaration_count},\"keyless\":true,\"complete_extraction_claim\":false}}"
                ));
            }
            #[cfg(not(target_os = "linux"))]
            return Err("product_platform".into());
        }
        if command == "inspect-db2" {
            #[cfg(target_os = "linux")]
            {
                let declaration_count = driver::inspect_db2_command(
                    &plan,
                    path("--client")?,
                    path("--output")?,
                    &file_data_ids,
                )?;
                return Ok(format!(
                    "{{\"contract\":\"forever-db2-inspection-contract-v1\",\"status\":\"inspection_written\",\"build\":70450,\"declarations\":{declaration_count},\"keyless\":true,\"extracts_nothing\":true}}"
                ));
            }
            #[cfg(not(target_os = "linux"))]
            return Err("product_platform".into());
        }
        // Hash the loaded Linux executable, even if its ordinary pathname was
        // replaced by a concurrent build. This kernel-owned link is deliberate.
        #[cfg(target_os = "linux")]
        let producer = integrity::reader(
            std::fs::File::open("/proc/self/exe")
                .map_err(|_| Error::from("product_binary_identity"))?,
            forever_extractor::MAX_BINARY_BYTES,
        )
        .map_err(|_| Error::from("product_binary_identity"))?;
        #[cfg(windows)]
        let producer = forever_extractor::producer::loaded_identity()
            .map_err(|error| Error::from(error.code()))?;
        // The explicit file is loaded only for client commands, after closed
        // plan/option admission. It is never hashed into context or reported.
        let keys = options.get("--keys").map(|p| input::keys(p)).transpose()?;
        if command == "inspect" {
            let context = driver::inspect(&plan, path("--client")?, producer, keys)?;
            return Ok(format!(
                "{{\"contract\":\"{}\",\"status\":\"admitted\",\"context\":{},\"availability_checked\":false{capability},\"full_extractor_complete\":false}}",
                plan.contract(),
                context.json()
            ));
        }
        let summary = if plan.is_v23() {
            #[cfg(target_os = "linux")]
            {
                let inputs = driver::SelectedNavigationInputs {
                    terrain_package: path("--terrain-package")?,
                    terrain_plan: path("--terrain-plan")?,
                    vmap_package: path("--vmap-package")?,
                    vmap_plan: path("--vmap-plan")?,
                };
                driver::extract_selected_navigation(
                    &plan,
                    path("--client")?,
                    path("--output")?,
                    producer,
                    command == "resume",
                    &inputs,
                )
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Err("product_platform".into());
            }
        } else if let Some(scratch) = options.get("--scratch") {
            driver::extract_disk(
                &plan,
                path("--client")?,
                path("--output")?,
                producer,
                command == "resume",
                keys,
                scratch,
            )
        } else {
            match keys {
                Some(keys) => driver::extract_with_keys(
                    &plan,
                    path("--client")?,
                    path("--output")?,
                    producer,
                    command == "resume",
                    keys,
                ),
                None => driver::extract(
                    &plan,
                    path("--client")?,
                    path("--output")?,
                    producer,
                    command == "resume",
                ),
            }
        }?;
        Ok(format!(
            "{{\"contract\":\"{}\",\"status\":\"selected_producers_complete\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
            plan.contract(),
            summary.json()
        ))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = path;
        Err("product_platform".into())
    }
}

#[cfg(target_os = "linux")]
fn trinity_compat_command(options: &BTreeMap<&str, PathBuf>) -> Result<String, Error> {
    if options.len() != 4 {
        return Err("product_navigation_inputs_required".into());
    }
    let read_plan = |key| {
        let bytes = input::bytes(options[key].as_path(), forever_extractor::MAX_PLAN_BYTES)?;
        Plan::parse(std::str::from_utf8(&bytes).map_err(|_| Error::from("product_plan_format"))?)
            .map_err(Error::from)
    };
    let terrain_plan = read_plan("--terrain-plan")?;
    let navigation_plan = read_plan("--navigation-plan")?;
    forever_extractor::driver::admit(&terrain_plan)?;
    forever_extractor::driver::admit(&navigation_plan)?;
    let report = forever_extractor::verify::verify_trinity_compat(
        &terrain_plan,
        &options["--terrain-package"],
        &options["--terrain-plan"],
        &navigation_plan,
        &options["--navigation-package"],
    )?;
    Ok(report.json())
}

fn available_navigation_command(
    command: &str,
    plan: &Plan,
    options: &BTreeMap<&str, PathBuf>,
) -> Result<String, Error> {
    let capability = ",\"navigation_selection\":\"all-available-maps\",\"navigation_inputs\":\"sealed-v51-combined\",\"client_reads\":false,\"full_world_complete\":false,\"runtime_accepted\":false,\"offmesh_connections\":0";
    if command == "inspect" && options.len() == 1 {
        return Ok(format!(
            "{{\"contract\":\"{}\",\"status\":\"planned\",\"components\":32{capability},\"full_extractor_complete\":false}}",
            plan.contract()
        ));
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = plan;
        Err("product_platform".into())
    }
    #[cfg(target_os = "linux")]
    {
        let allowed = if command == "verify" {
            &["--plan", "--output", "--expect-context"][..]
        } else {
            &[
                "--plan",
                "--output",
                "--terrain-package",
                "--terrain-plan",
                "--expect-context",
            ][..]
        };
        if !["extract", "resume", "verify", "verify-links"].contains(&command)
            || options.keys().any(|key| !allowed.contains(key))
            || (matches!(command, "extract" | "resume") && options.contains_key("--expect-context"))
        {
            return Err("product_usage".into());
        }
        let path = |key| {
            options
                .get(key)
                .map(PathBuf::as_path)
                .ok_or_else(|| Error::from("product_navigation_inputs_required"))
        };
        let expected = options
            .get("--expect-context")
            .map(|p| {
                let bytes = input::bytes(p, 1024)?;
                Context::parse(
                    std::str::from_utf8(&bytes).map_err(|_| Error::from("product_context"))?,
                )
                .map_err(Error::from)
            })
            .transpose()?;
        let summary = match command {
            "verify" => forever_extractor::verify::verify(plan, path("--output")?, expected)?,
            "verify-links" => forever_extractor::verify::verify_available_navigation_links(
                plan,
                path("--terrain-package")?,
                path("--terrain-plan")?,
                path("--output")?,
                expected,
            )?,
            _ => {
                let producer = integrity::reader(
                    std::fs::File::open("/proc/self/exe")
                        .map_err(|_| Error::from("product_binary_identity"))?,
                    forever_extractor::MAX_BINARY_BYTES,
                )
                .map_err(|_| Error::from("product_binary_identity"))?;
                forever_extractor::driver::extract_available_navigation(
                    plan,
                    path("--terrain-package")?,
                    path("--terrain-plan")?,
                    path("--output")?,
                    producer,
                    command == "resume",
                )?
            }
        };
        Ok(format!(
            "{{\"contract\":\"{}\",\"status\":\"selected_producers_complete\",\"components\":32{capability},\"full_extractor_complete\":false,\"package\":{}}}",
            plan.contract(),
            summary.json()
        ))
    }
}

fn validate_selected_navigation_cli(
    command: &str,
    plan: &Plan,
    options: &BTreeMap<&str, PathBuf>,
) -> Result<(), Error> {
    if plan.is_v23() && command == "inspect" && options.contains_key("--client") {
        return Err("product_v23_client_inspect_unsupported".into());
    }

    let supplied = [
        "--terrain-package",
        "--terrain-plan",
        "--vmap-package",
        "--vmap-plan",
    ]
    .into_iter()
    .filter(|option| options.contains_key(*option))
    .count();
    let requires_pairs = plan.is_v23() && matches!(command, "extract" | "resume" | "verify-links");
    if requires_pairs && supplied != 4 {
        return Err("product_navigation_inputs_required".into());
    }
    if !requires_pairs && supplied != 0 {
        return Err("product_navigation_inputs_forbidden".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn assembly_expanded_resource_capability() -> String {
    let mut capability = placement_expanded_resource_capability();
    capability = capability.replace(
        "\"resource_profile\":\"linux-collision-placement-expanded-resources-v1\"",
        "\"resource_profile\":\"linux-collision-assembly-expanded-resources-v1\"",
    );
    capability = capability.replace(
        "\"collision_source_profile\":\"linux70450-placement-expanded-material-filtered-wmo-v1\"",
        "\"collision_source_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-v1\"",
    );
    capability = capability.replace(
        "\"coverage_profile\":\"linux70450-placement-expanded-material-filtered-wmo-coverage-v18\"",
        "\"coverage_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-coverage-v19\"",
    );
    format!("{capability},\"assembled_domain_bytes_max\":268435456")
}

#[cfg(target_os = "linux")]
fn liquid_object_complete_source_resource_capability() -> String {
    let limits = forever_casc::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
        .collector_limits();
    format!(
        ",\"source_bytes_max\":2147483648,\"model_bytes_max\":536870912,\"placement_bytes_max\":268435456,\"placement_bytes_max_per_map\":67108864,\"models_max\":{},\"reads_max\":{},\"request_calls_max\":{},\"dependency_requests_max\":{},\"raw_bytes_max\":{},\"assembled_domain_bytes_max\":268435456,\"resource_profile\":\"{}\",\"collision_source_profile\":\"{}\",\"coverage_profile\":\"{}\",\"codec_profile\":\"{}\",\"domain_profile\":\"{}\",\"verification_profile\":\"{}\",\"verification_scope\":\"{}\",\"coverage_authentication\":\"map_godi_domain_and_geometry_independently_derived;liquid_object_source_facts_bound_to_sealed_db2\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"complete_source\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\"",
        limits.max_models,
        limits.max_reads,
        limits.max_requests,
        limits.max_dependency_requests,
        limits.max_raw_bytes,
        forever_casc::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1.id(),
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE,
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE,
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE,
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_DOMAIN_PROFILE,
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE,
        forever_extractor::plan::LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE,
    )
}

#[cfg(not(target_os = "linux"))]
fn liquid_object_complete_source_resource_capability() -> &'static str {
    ",\"source_bytes_max\":2147483648,\"model_bytes_max\":536870912,\"placement_bytes_max\":268435456,\"placement_bytes_max_per_map\":67108864,\"models_max\":65536,\"reads_max\":1000000,\"request_calls_max\":16000000,\"dependency_requests_max\":16000000,\"raw_bytes_max\":268435456,\"assembled_domain_bytes_max\":268435456,\"resource_profile\":\"linux-collision-liquid-object-complete-source-resources-v1\",\"collision_source_profile\":\"linux70450-liquid-object-complete-source-vmap-product-v1\",\"coverage_profile\":\"linux70450-liquid-object-complete-source-coverage-v23\",\"domain_profile\":\"liquid-object-complete-source-v1\",\"verification_profile\":\"linux70450-liquid-object-complete-source-verification-v1\",\"verification_scope\":\"sealed_liquid_object_source_facts_and_v49_available_map_geometry\",\"coverage_authentication\":\"map_godi_domain_and_geometry_independently_derived;liquid_object_source_facts_bound_to_sealed_db2\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"complete_source_only\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
}

#[cfg(target_os = "linux")]
fn gameobject_expanded_resource_capability() -> String {
    let mut capability = assembly_expanded_resource_capability();
    capability = capability.replace(
        "\"resource_profile\":\"linux-collision-assembly-expanded-resources-v1\"",
        "\"resource_profile\":\"linux-collision-gameobject-expanded-resources-v1\"",
    );
    capability = capability.replace(
        "\"collision_source_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-v1\"",
        "\"collision_source_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-v1\"",
    );
    capability = capability.replace(
        "\"coverage_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-coverage-v19\"",
        "\"coverage_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20\"",
    );
    capability = capability.replace(
        "\"gameobject_scope\":\"excluded_by_profile\"",
        "\"gameobject_scope\":\"available-fdid-direct-v1\"",
    );
    capability = capability.replace(
        "\"model_scope\":\"placement_only\"",
        "\"model_scope\":\"placement-and-gameobject-v1\"",
    );
    capability = capability.replace(
        "\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\"",
        "\"model_source_scope\":\"placement-and-gameobject-v1\"",
    );
    capability = capability.replace(
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"liquid_object_scope\":\"out_of_scope\",\"gameobject_model_tree\":\"vmaps/GameObjectModels.dtree\",\"gameobject_tree_encoding\":\"VMAP_4.E\"",
    );
    format!(
        "{capability},\"completion\":\"linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1\",\"verification_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-verification-v1\",\"verification_scope\":\"artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage\""
    )
}

#[cfg(not(target_os = "linux"))]
fn gameobject_expanded_resource_capability() -> String {
    let mut capability = assembly_expanded_resource_capability();
    capability = capability.replace(
        "\"resource_profile\":\"linux-collision-assembly-expanded-resources-v1\"",
        "\"resource_profile\":\"linux-collision-gameobject-expanded-resources-v1\"",
    );
    capability = capability.replace(
        "\"collision_source_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-v1\"",
        "\"collision_source_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-v1\"",
    );
    capability = capability.replace(
        "\"coverage_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-coverage-v19\"",
        "\"coverage_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20\"",
    );
    capability = capability.replace(
        "\"gameobject_scope\":\"excluded_by_profile\"",
        "\"gameobject_scope\":\"available-fdid-direct-v1\"",
    );
    capability = capability.replace(
        "\"model_scope\":\"placement_only\"",
        "\"model_scope\":\"placement-and-gameobject-v1\"",
    );
    capability = capability.replace(
        "\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\"",
        "\"model_source_scope\":\"placement-and-gameobject-v1\"",
    );
    capability = capability.replace(
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"liquid_object_scope\":\"out_of_scope\",\"gameobject_model_tree\":\"vmaps/GameObjectModels.dtree\",\"gameobject_tree_encoding\":\"VMAP_4.E\"",
    );
    format!(
        "{capability},\"completion\":\"linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1\",\"verification_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-verification-v1\",\"verification_scope\":\"artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage\""
    )
}

#[cfg(target_os = "linux")]
fn gameobject_missing_root_resource_capability() -> String {
    let capability = gameobject_expanded_resource_capability();
    gameobject_missing_root_capability_from(capability)
}

#[cfg(not(target_os = "linux"))]
fn gameobject_missing_root_resource_capability() -> String {
    let capability = gameobject_expanded_resource_capability();
    gameobject_missing_root_capability_from(capability)
}

#[cfg(target_os = "linux")]
fn gameobject_root_encrypted_unavailable_resource_capability() -> String {
    let capability = gameobject_expanded_resource_capability();
    gameobject_root_encrypted_unavailable_capability_from(capability)
}

#[cfg(not(target_os = "linux"))]
fn gameobject_root_encrypted_unavailable_resource_capability() -> String {
    let capability = gameobject_expanded_resource_capability();
    gameobject_root_encrypted_unavailable_capability_from(capability)
}

fn gameobject_missing_root_capability_from(mut capability: String) -> String {
    capability = capability.replace(
        "linux-collision-gameobject-expanded-resources-v1",
        "linux-collision-gameobject-missing-root-expanded-resources-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-v1",
        "linux70450-gameobject-missing-root-material-filtered-wmo-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20",
        "linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21",
    );
    capability = capability.replace(
        "available-fdid-direct-v1",
        "available-fdid-direct-missing-root-v1",
    );
    capability = capability.replace(
        "placement-and-gameobject-v1",
        "placement-and-gameobject-with-missing-root-v1",
    );
    capability = capability.replace(
        "linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1",
        "linux70450-available-gameobject-missing-root-material-filtered-wmo-collision-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-verification-v1",
        "linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1",
    );
    capability = capability.replace(
        "artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage",
        "artifact_integrity_gameobject_missing_root_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage",
    );
    capability = capability.replace(
        "\"coverage_authentication\":\"independently_derived\"",
        "\"coverage_authentication\":\"map_godi_domain_and_geometry_independently_derived_except_producer_reported_gameobject_root_absence\"",
    );
    format!(
        "{capability},\"gameobject_root_absence_authentication\":\"producer_reported\",\"gameobject_missing_root_max\":4096"
    )
}

fn gameobject_root_encrypted_unavailable_capability_from(mut capability: String) -> String {
    capability = capability.replace(
        "linux-collision-gameobject-expanded-resources-v1",
        "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-v1",
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20",
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22",
    );
    capability = capability.replace(
        "available-fdid-direct-v1",
        "available-fdid-direct-root-absent-or-encrypted-unavailable-v1",
    );
    capability = capability.replace(
        "placement-and-gameobject-v1",
        "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1",
    );
    capability = capability.replace(
        "linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1",
        "linux70450-available-gameobject-root-encrypted-unavailable-material-filtered-wmo-collision-v1",
    );
    capability = capability.replace(
        "linux70450-gameobject-expanded-material-filtered-wmo-verification-v1",
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1",
    );
    capability = capability.replace(
        "artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage",
        "artifact_integrity_gameobject_root_and_encrypted_unavailable_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage",
    );
    capability = capability.replace(
        "\"coverage_authentication\":\"independently_derived\"",
        "\"coverage_authentication\":\"map_godi_domain_and_geometry_independently_derived_except_producer_reported_gameobject_absences\"",
    );
    format!(
        "{capability},\"gameobject_absence_authentication\":\"producer_reported_outcomes\",\"gameobject_missing_root_max\":4096,\"gameobject_encrypted_unavailable_max\":4096"
    )
}

#[cfg(target_os = "linux")]
fn placement_expanded_resource_capability() -> String {
    use forever_casc::collision::resources::Profile;

    let profile = Profile::PlacementExpandedV1;
    let limits = profile.collector_limits();
    format!(
        ",\"source_bytes_max\":{},\"model_bytes_max\":536870912,\"placement_bytes_max\":268435456,\"placement_bytes_max_per_map\":67108864,\"models_max\":{},\"reads_max\":{},\"request_calls_max\":{},\"dependency_requests_max\":{},\"raw_bytes_max\":{},\"resource_profile\":\"{}\",\"collision_source_profile\":\"linux70450-placement-expanded-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-placement-expanded-material-filtered-wmo-coverage-v18\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\"",
        limits.max_source_bytes,
        limits.max_models,
        limits.max_reads,
        limits.max_requests,
        limits.max_dependency_requests,
        limits.max_raw_bytes,
        profile.id(),
    )
}

#[cfg(not(target_os = "linux"))]
fn placement_expanded_resource_capability() -> &'static str {
    ",\"source_bytes_max\":2147483648,\"model_bytes_max\":536870912,\"placement_bytes_max\":268435456,\"placement_bytes_max_per_map\":67108864,\"models_max\":65536,\"reads_max\":1000000,\"request_calls_max\":16000000,\"dependency_requests_max\":16000000,\"raw_bytes_max\":268435456,\"resource_profile\":\"linux-collision-placement-expanded-resources-v1\",\"collision_source_profile\":\"linux70450-placement-expanded-material-filtered-wmo-v1\",\"coverage_profile\":\"linux70450-placement-expanded-material-filtered-wmo-coverage-v18\",\"coverage_authentication\":\"independently_derived\",\"root_absence_authentication\":\"producer_reported\",\"model_absence_authentication\":\"producer_reported\",\"wmo_absence_authentication\":\"producer_reported\",\"material_filtered_wmo_authentication\":\"producer_reported\",\"source_authentication_flags\":\"producer_reported\",\"map_source_scope\":\"available_sections+vmap_materialization+root_absence_declared\",\"model_source_scope\":\"wmo-doodad-m2-fdid-root-absence-declared+vacated-wmo-declared+empty-wmo-declared+unselected-wmo-declared+material-filtered-wmo-declared\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false,\"complete_vmap_component\":false,\"partial_db2_declarations\":\"per-table-including-Map\""
}

#[cfg(not(target_os = "linux"))]
fn assembly_expanded_resource_capability() -> String {
    let mut capability = placement_expanded_resource_capability().to_owned();
    capability = capability.replace(
        "\"resource_profile\":\"linux-collision-placement-expanded-resources-v1\"",
        "\"resource_profile\":\"linux-collision-assembly-expanded-resources-v1\"",
    );
    capability = capability.replace(
        "\"collision_source_profile\":\"linux70450-placement-expanded-material-filtered-wmo-v1\"",
        "\"collision_source_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-v1\"",
    );
    capability = capability.replace(
        "\"coverage_profile\":\"linux70450-placement-expanded-material-filtered-wmo-coverage-v18\"",
        "\"coverage_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-coverage-v19\"",
    );
    format!("{capability},\"assembled_domain_bytes_max\":268435456")
}

fn parse_file_data_id(value: &str) -> Result<u32, Error> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("product_probe_file_data_id".into());
    }
    value
        .parse()
        .map_err(|_| Error::from("product_probe_file_data_id"))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn placement_expanded_inspect_capability_reports_collector_limits() {
        let capability = super::placement_expanded_resource_capability();
        for expected in [
            "\"source_bytes_max\":2147483648,",
            "\"models_max\":65536,",
            "\"reads_max\":1000000,",
            "\"request_calls_max\":16000000,",
            "\"dependency_requests_max\":16000000,",
            "\"raw_bytes_max\":268435456,",
            "\"resource_profile\":\"linux-collision-placement-expanded-resources-v1\",",
        ] {
            assert!(
                capability.contains(expected),
                "missing {expected}: {capability}"
            );
        }
    }

    #[test]
    fn gameobject_expanded_inspect_capability_keeps_v46_limits_and_uses_v47_identity() {
        let capability = super::gameobject_expanded_resource_capability();
        for expected in [
            "\"source_bytes_max\":2147483648,",
            "\"model_bytes_max\":536870912,",
            "\"placement_bytes_max\":268435456,",
            "\"placement_bytes_max_per_map\":67108864,",
            "\"assembled_domain_bytes_max\":268435456,",
            "\"models_max\":65536,",
            "\"reads_max\":1000000,",
            "\"request_calls_max\":16000000,",
            "\"dependency_requests_max\":16000000,",
            "\"raw_bytes_max\":268435456,",
            "\"resource_profile\":\"linux-collision-gameobject-expanded-resources-v1\",",
            "\"collision_source_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-v1\",",
            "\"coverage_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20\",",
            "\"gameobject_model_tree\":\"vmaps/GameObjectModels.dtree\",",
            "\"gameobject_tree_encoding\":\"VMAP_4.E\",",
            "\"completion\":\"linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1\",",
            "\"verification_profile\":\"linux70450-gameobject-expanded-material-filtered-wmo-verification-v1\",",
            "\"verification_scope\":\"artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage\"",
        ] {
            assert!(
                capability.contains(expected),
                "missing {expected}: {capability}"
            );
        }
        assert!(!capability.contains("linux-collision-assembly-expanded-resources-v1"));
        assert!(!capability.contains("assembly-expanded-material-filtered-wmo-coverage-v19"));
    }
}
