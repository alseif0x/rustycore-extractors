// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete target DB2 inventory admission before any payload buffer/hash/read.
use super::*;
pub(super) fn admit_metadata(plan: &Plan, artifacts: &[Artifact]) -> Result<(), Error> {
    let prefix = format!("dbc/{}/", plan.locale.name());
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for artifact in artifacts {
        if artifact.component != Component::Db2 {
            return Err("product_inventory".into());
        }
        if artifact.bytes == 0 {
            return Err("product_empty_artifact".into());
        }
        if artifact.bytes > 16 * 1024 * 1024 {
            return Err("product_file_limit".into());
        }
        let name = artifact
            .path
            .strip_prefix(&prefix)
            .and_then(|s| s.strip_suffix(".db2"))
            .ok_or("product_artifact_name")?;
        if !plan.db2.iter().any(|e| e.name == name) || !names.insert(name) {
            return Err("product_inventory".into());
        }
        total = total
            .checked_add(artifact.bytes)
            .filter(|n| *n <= 16 * 1024 * 1024 * 1024)
            .ok_or("product_db2_total_limit")?;
    }
    if names != plan.db2.iter().map(|e| e.name.as_str()).collect() {
        return Err("product_inventory".into());
    }
    Ok(())
}
