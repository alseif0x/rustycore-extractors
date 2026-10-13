// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit offline product invocation.
use forever_casc::integrity;
#[cfg(target_os = "linux")]
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
        return Ok("forever-extractor inspect --plan PLAN [--client CLIENT [--keys KEYS]]\nforever-extractor extract|resume --plan PLAN --client CLIENT --output NEW_OR_INCOMPLETE_OUTPUT [--keys KEYS] [--scratch PRIVATE_DIRECTORY]\nforever-extractor verify --plan PLAN --output OUTPUT [--expect-context CONTEXT]\nOffline Rust-local only; canonical TSV v1 N/Z, v2 provided-salsa20-v1, v3 leased disk-world collision, v4 build-70205 metadata, or v5 build-70338 metadata. Supplied-key v2-v5 client commands require --keys; disabled plans forbid it; plan-only inspection and verification need no keys. V3-v5 extract/resume with leased disk storage require --scratch; resident plans forbid it. Inspection and verification take no scratch. Linux product IO. Verification is bounded artifact/intrinsic-format scope; no runtime/full-client claim.".into());
    }
    if !["inspect", "extract", "resume", "verify"].contains(&command) || args.len() % 2 != 1 {
        return Err("product_usage".into());
    }
    let allowed = match command {
        "inspect" => &["--plan", "--client", "--keys"][..],
        "verify" => &["--plan", "--output", "--expect-context"][..],
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
    } else if command == "verify" {
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
        && command != "verify"
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
        forever_extractor::plan::SchemaCatalog::WowDbDefs70338V1 => {
            format!(",\"schema_catalog\":\"{}\"", plan.schema_catalog.id())
        }
    };
    let capability =
        format!("{schema_capability}{decryption_capability}{collision_storage_capability}");
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
    #[cfg(target_os = "linux")]
    {
        use forever_extractor::{driver, verify};
        if command == "verify" {
            let expected = if let Some(p) = options.get("--expect-context") {
                let b = input::bytes(p, 1024)?;
                Some(Context::parse(
                    std::str::from_utf8(&b).map_err(|_| Error::from("product_context"))?,
                )?)
            } else {
                None
            };
            let summary = verify::verify(&plan, path("--output")?, expected)?;
            return Ok(format!(
                "{{\"contract\":\"{}\",\"scope\":\"artifact_integrity_and_intrinsic_formats\"{capability},\"full_extractor_complete\":false,\"package\":{}}}",
                plan.contract(),
                summary.json()
            ));
        }
        // Hash the loaded Linux executable, even if its ordinary pathname was
        // replaced by a concurrent build. This kernel-owned link is deliberate.
        let producer = integrity::reader(
            std::fs::File::open("/proc/self/exe")
                .map_err(|_| Error::from("product_binary_identity"))?,
            forever_extractor::MAX_BINARY_BYTES,
        )
        .map_err(|_| Error::from("product_binary_identity"))?;
        // The explicit file is loaded only for client commands, after closed
        // plan/option admission. It is never hashed into context or reported.
        let keys = options.get("--keys").map(|p| input::keys(p)).transpose()?;
        if command == "inspect" {
            let store = match keys {
                Some(keys) => driver::open_with_keys(&plan, path("--client")?, keys.into_keyring()),
                None => driver::open(&plan, path("--client")?),
            }?;
            let context = driver::context(&plan, &store, producer)?;
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
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Err("product_platform".into())
    }
}
