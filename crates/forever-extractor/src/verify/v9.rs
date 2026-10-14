// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete v9 metadata admission before any payload buffer/hash/read.
use super::*;

fn mismatch(code: &'static str, component: Component) -> Error {
    Error {
        code,
        component: Some(component),
    }
}

pub(super) fn admit_metadata(plan: &Plan, artifacts: &[Artifact]) -> Result<(), Error> {
    let mut db2 = BTreeSet::new();
    let mut gt_names = BTreeSet::new();
    let mut camera_ids = BTreeSet::new();
    let mut db2_total = 0u64;
    let mut gt_total = 0u64;
    let mut camera_total = 0u64;
    let mut source_bytes = None;
    let prefix = format!("dbc/{}/", plan.locale.name());
    for a in artifacts {
        let fail = |code| mismatch(code, a.component);
        if a.bytes == 0 {
            return Err(fail("product_empty_artifact"));
        }
        match a.component {
            Component::Db2 => {
                let name = a
                    .path
                    .strip_prefix(&prefix)
                    .and_then(|p| p.strip_suffix(".db2"))
                    .ok_or_else(|| fail("product_artifact_name"))?;
                if !plan.db2.iter().any(|e| e.name == name) || !db2.insert(name) {
                    return Err(fail("product_inventory"));
                }
                if a.bytes > cameras::MAX_SOURCE_BYTES as u64 {
                    return Err(fail("product_file_limit"));
                }
                db2_total = db2_total
                    .checked_add(a.bytes)
                    .filter(|n| *n <= 16 * 1024 * 1024 * 1024)
                    .ok_or_else(|| fail("product_db2_total_limit"))?;
                if name == "CinematicCamera" {
                    source_bytes = Some(a.bytes);
                }
            }
            Component::GameTables => {
                if !gt::Table::ALL.iter().any(|t| t.relative_path() == a.path)
                    || !gt_names.insert(&a.path)
                {
                    return Err(fail("product_artifact_name"));
                }
                if a.bytes > forever_casc::gametables::MAX_FILE_BYTES as u64 {
                    return Err(fail("product_file_limit"));
                }
                gt_total = gt_total
                    .checked_add(a.bytes)
                    .filter(|n| *n <= 80 * 1024 * 1024)
                    .ok_or_else(|| fail("product_file_limit"))?;
            }
            Component::Cameras => {
                let value = a
                    .path
                    .strip_prefix("cameras/FILE")
                    .and_then(|p| p.strip_suffix(".xxx"))
                    .and_then(|s| u32::from_str_radix(s, 16).ok())
                    .ok_or_else(|| fail("product_artifact_name"))?;
                if cameras::relative_path(value) != a.path || !camera_ids.insert(value) {
                    return Err(fail("product_artifact_name"));
                }
                if a.bytes > cameras::MAX_FILE_BYTES as u64 {
                    return Err(fail("product_file_limit"));
                }
                if camera_ids.len() > cameras::MAX_FILES {
                    return Err(fail("camera_policy_limit"));
                }
                camera_total = camera_total
                    .checked_add(a.bytes)
                    .filter(|n| *n <= cameras::MAX_TOTAL_BYTES as u64)
                    .ok_or_else(|| fail("camera_policy_limit"))?;
            }
            _ => return Err(fail("product_inventory")),
        }
    }
    let source_bytes =
        source_bytes.ok_or_else(|| mismatch("product_camera_source", Component::Cameras))?;
    if camera_total
        .checked_add(source_bytes)
        .is_none_or(|n| n > cameras::MAX_TOTAL_BYTES as u64)
    {
        return Err(mismatch("camera_policy_limit", Component::Cameras));
    }
    if db2 != plan.db2.iter().map(|e| e.name.as_str()).collect()
        || gt_names.len() != gt::Table::ALL.len()
    {
        return Err("product_inventory".into());
    }
    Ok(())
}
