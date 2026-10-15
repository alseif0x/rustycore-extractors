// SPDX-License-Identifier: GPL-3.0-or-later
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
        Ok(json) => println!("{json}"),
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
                || error.code == "product_scratch_required"
                || error.code == "product_scratch_forbidden"
                || error.code == "product_disk_unsupported"
            {
                2
            } else {
                9
            });
        }
    }
}
fn run(contract: &mut &'static str) -> Result<String, Error> {
    let args: Vec<_> = std::env::args_os().skip(1).take(16).collect();
    let Some(command) = args.first().and_then(|a| a.to_str()) else {
        return Err("product_usage".into());
    };
    if command == "--help" && args.len() == 1 {
        return Ok("forever-extractor inspect --plan PLAN [--client CLIENT [--keys KEYS]]\nforever-extractor extract|resume --plan PLAN --client CLIENT --output NEW_OR_INCOMPLETE_OUTPUT [--keys KEYS] [--scratch PRIVATE_DIRECTORY]\nforever-extractor verify --plan PLAN --output OUTPUT [--expect-context CONTEXT]\nforever-extractor verify-links --plan PLAN --output OUTPUT [--expect-context CONTEXT] (Linux only)\nOffline Rust-local only; canonical TSV v1 N/Z, v2 provided-salsa20-v1, v3 leased disk-world collision, v4 build-70205 metadata, v5 build-70338 metadata, v6 pinned-metadata inline off-mesh navigation, Linux v8 selected map and ancestors, Linux v9 build-70338 exact DB2+GT+cameras, Linux v10 build-70374 DB2-only, Linux v11 build-70374 exact DB2+GT+cameras, or Linux v12 build-70374 available-Map terrain-only full map/all-available-maps, disabled keys (v7 reserved). Supplied-key v2-v6/v8-v11 client commands require --keys; disabled plans forbid it; plan-only inspection and verification need no keys. V3-v6/v8 extract/resume with leased disk storage require --scratch; resident plans forbid it. Inspection and verification take no scratch. Legacy Linux and Windows resident product IO; v9-v11 require Linux; leased disk storage requires Linux. Windows client commands require the actual loaded sealed PE. Verification is bounded artifact/intrinsic-format scope; verify-links additionally checks published terrain tile sets and navigation tile/global presence. V8 navigation builds each map's own candidates across the selected map and ancestor dependencies; maps without own candidates emit no MMaps. V8 verification enforces that whole domain and its terrain/navigation links; global GameObject startup remains required. Linux v10 is explicit Rust-local build-70374 DB2-only with wowdbdefs-70374-v1; disabled/provided-salsa20-v1 follow the same explicit key rules. Native, other components and Windows v10 execution reject. Linux v11 is explicit Rust-local build-70374 exact DB2+GT+cameras with wowdbdefs-70374-v1 and the same key policies and limits; Windows/native/geometry remain unsupported. No runtime/full-client claim.".into());
    }
    #[cfg(not(target_os = "linux"))]
    if command == "verify-links" {
        return Err("product_platform".into());
    }
    if !["inspect", "extract", "resume", "verify", "verify-links"].contains(&command)
        || args.len() % 2 != 1
    {
        return Err("product_usage".into());
    }
    let allowed = match command {
        "inspect" => &["--plan", "--client", "--keys"][..],
        "verify" | "verify-links" => &["--plan", "--output", "--expect-context"][..],
        _ => &["--plan", "--client", "--output", "--keys", "--scratch"][..],
    };
    let mut options = BTreeMap::new();
    for pair in args[1..].as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or("product_usage")?;
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
    let bytes = input::bytes(path("--plan")?, forever_extractor::MAX_PLAN_BYTES)?;
    let plan =
        Plan::parse(std::str::from_utf8(&bytes).map_err(|_| Error::from("product_plan_format"))?)?;
    *contract = plan.contract();
    if matches!(command, "extract" | "resume") {
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
        | forever_extractor::plan::SchemaCatalog::WowDbDefs70374V1 => {
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
    let map_domain_capability = plan.map_domain.map_or_else(String::new, |id| {
        format!(
            ",\"map_domain\":\"{}\",\"requested_map\":{id},\"gameobject_scope\":\"{}\",\"placement_ids\":\"{}\"",
            forever_extractor::plan::MAP_DOMAIN_PROFILE,
            forever_extractor::plan::GAMEOBJECT_SCOPE_PROFILE,
            forever_extractor::plan::PLACEMENT_ID_DOMAIN_PROFILE,
        )
    });
    let terrain_capability = if plan.is_v12() {
        ",\"terrain_source_profile\":\"linux70374-full-maps-available-map-v1\",\"coverage_authentication\":\"producer_reported\",\"whole_client_map_coverage\":false"
    } else {
        ""
    };
    let capability = format!(
        "{schema_capability}{decryption_capability}{collision_storage_capability}{offmesh_capability}{map_domain_capability}{terrain_capability}"
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
                let summary = verify::verify_links(&plan, path("--output")?, expected)?;
                return Ok(format!(
                    "{{\"contract\":\"{}\",\"scope\":\"{}\",\"verification_profile\":\"{}\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                    plan.contract(),
                    if plan.is_v12() {
                        forever_extractor::plan::AVAILABLE_TERRAIN_VERIFY_SCOPE
                    } else if plan.map_domain.is_some() {
                        forever_extractor::plan::MAP_DOMAIN_VERIFY_SCOPE
                    } else {
                        verify::links::SCOPE
                    },
                    if plan.is_v12() {
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
            let scope = if plan.is_v12() {
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
        let summary = if let Some(scratch) = options.get("--scratch") {
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
