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
    let map_domain_capability = if plan.is_v22() {
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
    let terrain_capability = if plan.is_v22() {
        ",\"collision_source_profile\":\"linux70450-selected-map-geometry-v1\",\"coverage_profile\":\"linux70450-selected-map-geometry-coverage-v1\",\"coverage_authentication\":\"producer_reported\",\"map_source_scope\":\"available_sections\",\"liquid_sources\":\"LiquidMaterial+LiquidType_complete\",\"liquid_object_scope\":\"out_of_scope\",\"complete_global_map_coverage_claim\":false"
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
    let capability = format!(
        "{schema_capability}{decryption_capability}{collision_storage_capability}{offmesh_capability}{map_domain_capability}{terrain_capability}{navigation_capability}"
    );
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
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    if plan.is_v15() || plan.is_v14() || plan.is_v18() {
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
                        verify::links::SCOPE
                    },
                    if plan.is_v15() || plan.is_v18() {
                        forever_extractor::plan::COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
                    } else if plan.is_v22() {
                        forever_extractor::plan::SELECTED_GEOMETRY_VERIFY_PROFILE
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
            if plan.is_v23() {
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"selected_navigation_package_integrity_and_stored_provenance\",\"verification_profile\":\"linux70450-selected-navigation-v1\",\"client_reads\":false{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
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

fn parse_file_data_id(value: &str) -> Result<u32, Error> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("product_probe_file_data_id".into());
    }
    value
        .parse()
        .map_err(|_| Error::from("product_probe_file_data_id"))
}
