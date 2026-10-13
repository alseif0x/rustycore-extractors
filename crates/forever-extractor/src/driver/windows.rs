// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: Windows resident product dispatch and supplied-key leases.
//! Complete existing producers share the Windows package owner. Supplied keys
//! remain leased until completion; disk storage is an independently admitted
//! profile and never falls back to resident or path-only producer identity.
use super::*;
use forever_platform::windows::ReadFile;
pub(super) mod disk;

pub(super) fn verify_keys(source: Option<&ReadFile>) -> Result<(), Error> {
    if let Some(source) = source {
        source
            .verify_unchanged()
            .map_err(|_| Error::from("product_keys_input_changed"))?;
    }
    Ok(())
}

pub(super) fn inspect(
    plan: &Plan,
    client: &Path,
    producer: [u8; 32],
    keys: Option<crate::input::SuppliedKeys>,
) -> Result<Context, Error> {
    let (keyring, source) = match keys {
        Some(keys) => {
            let (keyring, _, source) = keys.into_windows_parts();
            (Some(keyring), Some(source))
        }
        None => (None, None),
    };
    verify_keys(source.as_ref())?;
    let result =
        open_inner(plan, client, keyring).and_then(|store| context(plan, &store, producer));
    // The source lease survives context admission and is rechecked even on
    // client failure. Key names/material/identity never enter the context.
    verify_keys(source.as_ref())?;
    result
}

pub(super) fn extract_inner(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: Option<crate::input::SuppliedKeys>,
    scratch: Option<&Path>,
) -> Result<package::Summary, Error> {
    admit(plan)?;
    let disk = match (plan.collision_storage, scratch) {
        (CollisionStorage::Resident, None) => None,
        (CollisionStorage::Resident, Some(_)) => return Err("product_scratch_forbidden".into()),
        (CollisionStorage::WindowsDiskV1(budget), Some(scratch)) => Some((budget, scratch)),
        (CollisionStorage::WindowsDiskV1(_), None) => {
            return Err("product_scratch_required".into());
        }
        (CollisionStorage::LeasedDiskV1(_), _) => return Err("product_disk_unsupported".into()),
    };
    validate_keys(plan, keys.is_some())?;
    let (keyring, key_path, key_source) = match keys {
        Some(keys) => {
            let (keyring, path, source) = keys.into_windows_parts();
            (Some(keyring), Some(path), Some(source))
        }
        None => (None, None, None),
    };
    verify_keys(key_source.as_ref())?;
    let result = (|| {
        let preflight = disk
            .map(|(budget, scratch)| {
                disk::Preflight::open(
                    budget,
                    scratch,
                    output,
                    client,
                    key_path.as_deref(),
                    key_source.as_ref(),
                )
            })
            .transpose()?;
        let result = (|| {
            let mut store = open_inner(plan, client, keyring)?;
            let context = context(plan, &store, producer)?;
            let data = client.join("Data");
            let archives = data.join("data");
            let configs = data.join("config");
            let protected = [
                client,
                data.as_path(),
                archives.as_path(),
                configs.as_path(),
            ];
            let mut limits = package::Limits::default();
            if let Some(budget) = plan.collision_storage.disk_budget() {
                limits.total_bytes = budget.output_bytes;
            }
            if let Some(preflight) = &preflight {
                preflight.verify_unchanged()?;
            }
            let make_run = |sources: &[&ReadFile]| {
                if resume {
                    Run::resume_with_sources(output, context, &protected, limits, sources)
                } else {
                    Run::create_with_sources(output, context, &protected, limits, sources)
                }
            };
            let run = match key_source.as_ref() {
                Some(source) => make_run(&[source]),
                None => make_run(&[]),
            }?;
            execute(
                plan,
                &mut store,
                output,
                run,
                key_source.as_ref(),
                preflight.as_ref(),
            )
        })();
        if let Some(preflight) = &preflight {
            preflight.verify_unchanged()?;
        }
        result
    })();
    // Keep the lease alive through finish and check it on every return path.
    verify_keys(key_source.as_ref())?;
    result
}
