// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: product inventory and bounded intrinsic format checks.
//! Does not prove VMap associations, Detour topology, original output parity,
//! full selected geometry inventories or runtime/client completeness.
use crate::{
    Error,
    driver::validate_db2,
    plan::{CollisionStorage, OffMeshConnection, Plan, Selection},
};
use forever_casc::{
    cameras,
    gametables::catalog as gt,
    integrity,
    package::{self, Artifact, Component, Context},
    terrain,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

#[cfg(target_os = "linux")]
mod compatible_vmaps;
#[cfg(test)]
mod tests;
#[cfg(target_os = "linux")]
mod v10;
#[cfg(target_os = "linux")]
mod v9;

#[cfg(target_os = "linux")]
pub mod links;

#[cfg(target_os = "linux")]
type ArtifactReader<'a> = &'a mut std::fs::File;
#[cfg(not(target_os = "linux"))]
type ArtifactReader<'a> = &'a mut (dyn Read + 'a);

#[derive(Clone, Copy)]
enum Scope {
    Intrinsic,
    #[cfg(target_os = "linux")]
    Links,
}

pub fn verify(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    verify_inner(plan, output, expected, Scope::Intrinsic)
}

/// Linux-only cross-artifact checks over the same complete verification traversal.
/// Does not establish VMap associations, topology or unobserved map completeness.
#[cfg(target_os = "linux")]
pub fn verify_links(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    verify_inner(plan, output, expected, Scope::Links)
}

fn verify_inner(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
    scope: Scope,
) -> Result<package::Summary, Error> {
    crate::driver::admit(plan)?;
    #[cfg(target_os = "linux")]
    if plan.is_v14() || plan.is_v15() || plan.is_v18() {
        return compatible_vmaps::verify(plan, output, expected);
    }
    let mut limits = package::Limits::default();
    if let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage {
        limits.total_bytes = budget.output_bytes;
    }
    #[cfg(target_os = "linux")]
    if plan.is_available_terrain() {
        limits = crate::driver::terrain_v12::package_limits();
    }
    let mut facts = Facts::default();
    #[cfg(target_os = "linux")]
    let mut link_facts = (!plan.is_available_terrain()
        && (matches!(scope, Scope::Links) || plan.map_domain.is_some()))
    .then(links::Facts::default);
    #[cfg(not(target_os = "linux"))]
    let _ = scope;
    let mut failure = None;
    let inspect = |artifact: &Artifact, file: ArtifactReader<'_>| {
        let mut bytes = Vec::new();
        let result = (|| {
            bytes
                .try_reserve_exact(artifact.bytes as usize)
                .map_err(|_| Error::from("product_allocation"))?;
            file.take(artifact.bytes + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::from("product_input_io"))?;
            if bytes.len() as u64 != artifact.bytes {
                return Err("product_input_changed".into());
            }
            facts.inspect(plan, artifact, &bytes)?;
            #[cfg(target_os = "linux")]
            if let Some(links) = &mut link_facts {
                links.inspect(artifact, &bytes)?;
            }
            Ok(())
        })();
        result.map_err(|mut error: Error| {
            error.component = Some(artifact.component);
            failure = Some(error);
            package::Error::Schema
        })
    };
    #[cfg(target_os = "linux")]
    let mut metadata_failure = None;
    #[cfg(target_os = "linux")]
    let verified = if plan.is_gt_cameras()
        || plan.is_v10()
        || plan.is_v16()
        || plan.is_v17()
        || plan.is_available_terrain()
    {
        package::verify_with_preflight(
            output,
            expected,
            limits,
            |artifacts| {
                let admitted = if plan.is_v13() {
                    crate::driver::terrain_v13::admit_metadata(plan, artifacts)
                } else if plan.is_v12() {
                    crate::driver::terrain_v12::admit_metadata(plan, artifacts)
                } else if plan.is_v10() || plan.is_v16() || plan.is_v17() {
                    v10::admit_metadata(plan, artifacts)
                } else {
                    v9::admit_metadata(plan, artifacts)
                };
                admitted.map_err(|error| {
                    metadata_failure = Some(error);
                    package::Error::Schema
                })
            },
            inspect,
        )
    } else {
        package::verify_with(output, expected, limits, inspect)
    };
    #[cfg(not(target_os = "linux"))]
    let verified = package::verify_with(output, expected, limits, inspect);
    #[cfg(target_os = "linux")]
    let summary = verified.map_err(|e| metadata_failure.or(failure).unwrap_or_else(|| e.into()))?;
    #[cfg(not(target_os = "linux"))]
    let summary = verified.map_err(|e| failure.unwrap_or_else(|| e.into()))?;
    let context = summary.context;
    if context.build != plan.build
        || context.locale != plan.locale
        || context.components != plan.components
        || context.plan_sha256 != integrity::digest(plan.render().as_bytes())
    {
        return Err("product_plan_mismatch".into());
    }
    if facts.db2 != plan.db2.iter().map(|e| e.name.clone()).collect()
        || (plan.contains(Component::GameTables) && facts.gt != gt::Table::ALL.len())
        || (plan.contains(Component::Cameras)
            && &facts.cameras
                != facts
                    .camera_source
                    .as_ref()
                    .ok_or("product_camera_source")?)
        || (plan.contains(Component::Collision) && !facts.dtree)
    {
        return Err("product_inventory".into());
    }
    #[cfg(target_os = "linux")]
    if plan.is_v13() {
        facts
            .terrain_v13
            .take()
            .ok_or("product_terrain_coverage")?
            .finish(plan, context)?;
    }
    #[cfg(target_os = "linux")]
    if plan.is_v12() {
        facts
            .terrain_v12
            .take()
            .ok_or("product_terrain_coverage")?
            .finish(plan, context)?;
    }
    facts.finish_domain(plan)?;
    #[cfg(target_os = "linux")]
    if let Some(links) = link_facts {
        links.finish(plan)?;
    }
    Ok(summary)
}

#[derive(Default)]
struct Facts {
    #[cfg(target_os = "linux")]
    terrain_v12: Option<crate::driver::terrain_v12::VerifyFacts>,
    #[cfg(target_os = "linux")]
    terrain_v13: Option<crate::driver::terrain_v13::VerifyFacts>,
    db2: BTreeSet<String>,
    gt: usize,
    cameras: BTreeSet<u32>,
    camera_source: Option<BTreeSet<u32>>,
    trees: BTreeMap<u32, usize>,
    dtree: bool,
    domain: Option<crate::map_domain::Domain>,
    domain_terrain_maps: Vec<u32>,
    domain_terrain_lists: Vec<u32>,
    domain_collision_maps: Vec<u32>,
    domain_collision_payload_maps: Vec<u32>,
    domain_navigation_globals: Vec<u32>,
}
impl Facts {
    fn finish_domain(&self, plan: &Plan) -> Result<(), Error> {
        if plan.map_domain.is_none() {
            return Ok(());
        }
        let domain = self.domain.as_ref().ok_or("product_map_domain_source")?;
        if Some(domain.requested()) != plan.map_domain {
            return Err("product_map_domain_source".into());
        }
        let mismatch = |code, component| Error {
            code,
            component: Some(component),
        };
        if domain.ids().len() != self.domain_terrain_lists.len()
            || domain
                .ids()
                .iter()
                .any(|id| !self.domain_terrain_lists.contains(id))
            || self
                .domain_terrain_maps
                .iter()
                .any(|&id| !domain.contains(id))
        {
            return Err(mismatch("product_map_domain_terrain", Component::Terrain));
        }
        if self
            .domain_collision_maps
            .iter()
            .any(|&id| !domain.contains(id))
            || self.trees.len() != domain.ids().len()
            || domain.ids().iter().any(|id| !self.trees.contains_key(id))
        {
            return Err(mismatch(
                "product_map_domain_collision",
                Component::Collision,
            ));
        }
        let own_navigation = |id: &u32| {
            self.domain_terrain_maps.contains(id) || self.domain_collision_payload_maps.contains(id)
        };
        let mut expected_globals = domain.ids().iter().filter(|id| own_navigation(id));
        if plan.contains(Component::Navigation)
            && (self.domain_navigation_globals.len() != expected_globals.clone().count()
                || expected_globals.any(|id| !self.domain_navigation_globals.contains(id)))
        {
            return Err(mismatch(
                "product_map_domain_navigation",
                Component::Navigation,
            ));
        }
        Ok(())
    }
    fn inspect(&mut self, plan: &Plan, artifact: &Artifact, bytes: &[u8]) -> Result<(), Error> {
        if bytes.is_empty() {
            return Err("product_empty_artifact".into());
        }
        #[cfg(target_os = "linux")]
        if plan.is_v13() {
            return self
                .terrain_v13
                .get_or_insert_with(crate::driver::terrain_v13::VerifyFacts::new)
                .inspect(artifact, bytes);
        }
        #[cfg(target_os = "linux")]
        if plan.is_v12() {
            return self
                .terrain_v12
                .get_or_insert_with(crate::driver::terrain_v12::VerifyFacts::new)
                .inspect(artifact, bytes);
        }
        let path = &artifact.path;
        match artifact.component {
            Component::Db2 => {
                let prefix = format!("dbc/{}/", plan.locale.name());
                let name = path
                    .strip_prefix(&prefix)
                    .and_then(|v| v.strip_suffix(".db2"))
                    .ok_or("product_artifact_name")?;
                if !plan.db2.iter().any(|e| e.name == name) || bytes.len() > 16 * 1024 * 1024 {
                    return Err("product_inventory".into());
                }
                validate_db2(plan, name, bytes)?;
                if name == "Map"
                    && let Some(requested) = plan.map_domain
                {
                    self.domain = Some(crate::map_domain::Domain::derive(
                        bytes,
                        requested,
                        plan.contains(Component::Navigation),
                    )?);
                }
                if name == "Map"
                    && plan
                        .offmesh
                        .as_ref()
                        .is_some_and(|connections| !connections.is_empty())
                {
                    let source = forever_db2::terrain_sources::read_navigation_maps(bytes)
                        .map_err(|error| Error::from(error.code()))?;
                    if source.direct_records().is_empty() {
                        return Err("product_offmesh_empty_map".into());
                    }
                }
                if name == "CinematicCamera" && plan.contains(Component::Cameras) {
                    self.camera_source = Some(
                        if plan.is_v11() {
                            cameras::discover_70374(bytes)?
                        } else if plan.is_v9() {
                            cameras::discover_with_contract(
                                cameras::CameraContract::Build70338V1,
                                bytes,
                            )?
                        } else {
                            cameras::discover(bytes)?
                        }
                        .file_ids()
                        .clone(),
                    );
                }
                self.db2.insert(name.into());
            }
            Component::GameTables => {
                let table = gt::Table::ALL
                    .into_iter()
                    .find(|t| t.relative_path() == path)
                    .ok_or("product_artifact_name")?;
                if bytes.len() > forever_casc::gametables::MAX_FILE_BYTES {
                    return Err("product_file_limit".into());
                }
                if table.value_columns().is_some() {
                    gt::parse_numeric(bytes, table).map_err(|e| Error::from(e.code()))?;
                }
                self.gt += 1;
            }
            Component::Cameras => {
                let value = path
                    .strip_prefix("cameras/FILE")
                    .and_then(|v| v.strip_suffix(".xxx"))
                    .ok_or("product_artifact_name")?;
                let id = u32::from_str_radix(value, 16)
                    .map_err(|_| Error::from("product_artifact_name"))?;
                if cameras::relative_path(id) != *path || bytes.len() > cameras::MAX_FILE_BYTES {
                    return Err("product_artifact_name".into());
                }
                self.cameras.insert(id);
            }
            Component::Terrain => {
                if let Some(map) = path
                    .strip_prefix("maps/")
                    .and_then(|v| v.strip_suffix(".tilelist"))
                {
                    let id = decimal(map)?;
                    if plan.map_domain.is_some() {
                        domain_map(&mut self.domain_terrain_lists, id)?;
                    } else {
                        admit(plan.terrain, id, None, false)?;
                    }
                    if path != &format!("maps/{id:04}.tilelist") {
                        return Err("product_artifact_name".into());
                    }
                    terrain::tilelist::verify(bytes, plan.build)
                        .map_err(|_| Error::from("product_tilelist"))?;
                } else {
                    let (map, x, y) = terrain_map_name(path)?;
                    if plan.map_domain.is_some() {
                        domain_map(&mut self.domain_terrain_maps, map)?;
                    } else {
                        admit(plan.terrain, map, Some((x, y)), false)?;
                    }
                    forever_terrain::maps::verify(bytes, plan.build, 128 * 1024 * 1024)
                        .map_err(|e| Error::from(e.code()))?;
                }
            }
            Component::Collision => {
                use forever_vmap::{assembly, gameobject_models, vmo};
                if path == "vmaps/GameObjectModels.dtree" {
                    gameobject_models::Dtree::parse(bytes, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                    self.dtree = true;
                } else if path.ends_with(".vmo") && path[6..].split('/').count() == 1 {
                    vmo::View::parse(bytes, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                } else {
                    let (folder, file) = path
                        .strip_prefix("vmaps/")
                        .and_then(|v| v.split_once('/'))
                        .ok_or("product_artifact_name")?;
                    let map = decimal(folder)?;
                    if plan.map_domain.is_some() {
                        domain_map(&mut self.domain_collision_maps, map)?;
                    }
                    if folder != format!("{map:04}") {
                        return Err("product_artifact_name".into());
                    }
                    if file == format!("{map:04}.vmtree") {
                        let tree = assembly::Tree::parse_intrinsic(bytes, Default::default())
                            .map_err(|e| Error::from(e.code()))?;
                        self.trees.insert(map, tree.bih().stats().primitive_count);
                    } else {
                        let count = self.trees.get(&map).copied().ok_or("product_vmap_tree")?;
                        collision_tile_name(file, map)?;
                        if file.ends_with(".vmtileidx") {
                            assembly::IndexView::parse(bytes, count, Default::default())
                                .map_err(|e| Error::from(e.code()))?;
                        } else {
                            assembly::TileView::parse(bytes, Default::default())
                                .map_err(|e| Error::from(e.code()))?;
                            if plan.map_domain.is_some() {
                                domain_map(&mut self.domain_collision_payload_maps, map)?;
                            }
                        }
                    }
                }
            }
            Component::Navigation => {
                use forever_navigation::mmap;
                let name = path.strip_prefix("mmaps/").ok_or("product_artifact_name")?;
                if let Some(map) = name.strip_suffix(".mmap") {
                    let id = decimal(map)?;
                    if plan.map_domain.is_none() {
                        admit(plan.navigation, id, None, true)?;
                    }
                    if plan.map_domain.is_some() {
                        domain_map(&mut self.domain_navigation_globals, id)?;
                    }
                    if path != &mmap::global_path(id) {
                        return Err("product_artifact_name".into());
                    }
                    let global = mmap::GlobalView::parse(bytes, id, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                    verify_offmesh_sources(plan, id, &global)?;
                } else {
                    let parts: Vec<_> = name
                        .strip_suffix(".mmtile")
                        .ok_or("product_artifact_name")?
                        .split('_')
                        .collect();
                    if parts.len() != 3 {
                        return Err("product_artifact_name".into());
                    }
                    let map = decimal(parts[0])?;
                    let row = decimal(parts[1])?;
                    let col = decimal(parts[2])?;
                    if plan.map_domain.is_none() {
                        admit(plan.navigation, map, Some((col, row)), true)?;
                    }
                    if row >= 64
                        || col >= 64
                        || mmap::tile_path(map, row as u8, col as u8)
                            .map_err(|e| Error::from(e.code()))?
                            != *path
                    {
                        return Err("product_artifact_name".into());
                    }
                    mmap::TileView::parse(bytes, Default::default())
                        .map_err(|e| Error::from(e.code()))?;
                }
            }
        }
        Ok(())
    }
}
// Five sorted vectors each retain at most1024u32s; one old/new growth
// overlap adds at most another4096bytes. These facts reserve24KiB separately
// from the compact source domain's64KiB, not allocator/RSS accounting.
fn domain_map(maps: &mut Vec<u32>, id: u32) -> Result<(), Error> {
    match maps.binary_search(&id) {
        Ok(_) => Ok(()),
        Err(at) => {
            if maps.len() >= 1024 {
                return Err("product_map_domain_limit".into());
            }
            maps.try_reserve_exact(1)
                .map_err(|_| Error::from("product_allocation"))?;
            if maps.capacity() > 1024 {
                return Err("product_map_domain_limit".into());
            }
            maps.insert(at, id);
            Ok(())
        }
    }
}
fn verify_offmesh_sources(
    plan: &Plan,
    map: u32,
    global: &forever_navigation::mmap::GlobalView<'_>,
) -> Result<(), Error> {
    match &plan.offmesh {
        None if global.sources().len() != 0 => Err("product_offmesh_unsupported".into()),
        None => Ok(()),
        Some(connections) => {
            let expected = connections
                .iter()
                .filter(|connection| connection.map_id == map)
                .copied()
                .map(Ok);
            if global
                .sources()
                .map(OffMeshConnection::from_source)
                .eq(expected)
            {
                Ok(())
            } else {
                Err("product_offmesh_mismatch".into())
            }
        }
    }
}

#[cfg(test)]
mod offmesh_tests;
fn decimal(value: &str) -> Result<u32, Error> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("product_artifact_name".into());
    }
    value.parse().map_err(|_| "product_artifact_name".into())
}
fn collision_tile_name(file: &str, map: u32) -> Result<(), Error> {
    let (body, suffix) = if let Some(v) = file.strip_suffix(".vmtileidx") {
        (v, "vmtileidx")
    } else {
        (
            file.strip_suffix(".vmtile")
                .ok_or("product_artifact_name")?,
            "vmtile",
        )
    };
    let parts: Vec<_> = body.split('_').collect();
    if parts.len() != 3 {
        return Err("product_artifact_name".into());
    }
    let x = decimal(parts[1])?;
    let y = decimal(parts[2])?;
    if x > 32767 || y > 255 || file != format!("{map:04}_{x:02}_{y:02}.{suffix}") {
        return Err("product_artifact_name".into());
    }
    Ok(())
}

fn terrain_map_name(path: &str) -> Result<(u32, u32, u32), Error> {
    let parts: Vec<_> = path
        .strip_prefix("maps/")
        .and_then(|v| v.strip_suffix(".map"))
        .ok_or("product_artifact_name")?
        .split('_')
        .collect();
    if parts.len() != 3 {
        return Err("product_artifact_name".into());
    }
    let map = decimal(parts[0])?;
    let y = decimal(parts[1])?;
    let x = decimal(parts[2])?;
    if x >= 64
        || y >= 64
        || terrain::plan::map_relative_path(map, x as u8, y as u8)
            .map_err(|e| Error::from(e.code()))?
            != path
    {
        return Err("product_artifact_name".into());
    }
    Ok((map, x, y))
}

fn admit(
    selection: Selection,
    map: u32,
    tile: Option<(u32, u32)>,
    global: bool,
) -> Result<(), Error> {
    let admitted = match selection {
        Selection::None => false,
        Selection::All => true,
        Selection::Map(id) => id == map,
        Selection::Tile { map: id, x, y } => {
            id == map && tile.map_or(global, |(tx, ty)| tx == u32::from(x) && ty == u32::from(y))
        }
    };
    if admitted {
        Ok(())
    } else {
        Err("product_selection_mismatch".into())
    }
}

#[cfg(test)]
mod domain_tests;
