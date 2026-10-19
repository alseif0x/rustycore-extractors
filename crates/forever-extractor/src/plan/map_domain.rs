// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: explicit Linux selected-map product domain grammar.
use super::{CollisionStorage, Component, Plan, SchemaCatalog, Selection, decimal, field};

pub const MAP_DOMAIN_PROFILE: &str = "map-and-ancestors-v1";
pub const GAMEOBJECT_SCOPE_PROFILE: &str = "global-startup-v1";
pub const PLACEMENT_ID_DOMAIN_PROFILE: &str = "selected-closure-serial-ids-v1";
pub const MAP_DOMAIN_VERIFICATION_PROFILE: &str = "forever-linux-map-domain-v1";
pub const MAP_DOMAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_intrinsic_formats_and_map_domain_links";

pub(super) fn parse<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<u32, &'static str> {
    if field(lines, "map-domain")? != MAP_DOMAIN_PROFILE {
        return Err("product_plan_map_domain");
    }
    let id = decimal(field(lines, "map-id")?)?;
    if field(lines, "gameobject-scope")? != GAMEOBJECT_SCOPE_PROFILE {
        return Err("product_plan_gameobject_scope");
    }
    if field(lines, "placement-ids")? != PLACEMENT_ID_DOMAIN_PROFILE {
        return Err("product_plan_placement_ids");
    }
    Ok(id)
}

pub(super) fn render(id: Option<u32>) -> String {
    id.map_or_else(String::new, |id| {
        format!(
            "map-domain\t{MAP_DOMAIN_PROFILE}\nmap-id\t{id}\ngameobject-scope\t{GAMEOBJECT_SCOPE_PROFILE}\nplacement-ids\t{PLACEMENT_ID_DOMAIN_PROFILE}\n"
        )
    })
}

pub(super) fn validate(plan: &Plan) -> Result<(), &'static str> {
    let Some(id) = plan.map_domain else {
        return Ok(());
    };
    let scope = if plan.is_v21() {
        plan.build == 70450 && plan.schema_catalog == SchemaCatalog::WowDbDefs70450V1
    } else {
        [70170, 70205].contains(&plan.build)
            && plan.schema_catalog == SchemaCatalog::PinnedMetadataV1
    };
    if !scope || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_)) {
        return Err("product_plan_map_domain_scope");
    }
    let navigation = if plan.is_v21() {
        Component::Navigation.bit()
    } else {
        0
    };
    if ![Component::Db2, Component::Terrain, Component::Collision]
        .into_iter()
        .all(|component| plan.contains(component))
        || plan.components & navigation != navigation
        || !plan.db2.iter().any(|entry| entry.name == "Map")
        || (plan.is_v21() && !plan.db2.iter().any(|entry| entry.name == "LiquidType"))
    {
        return Err("product_plan_map_domain_dependencies");
    }
    if plan.terrain != Selection::Map(id)
        || !matches!(plan.navigation, Selection::None | Selection::Map(_))
        || matches!(plan.navigation, Selection::Map(map) if map != id)
        || (plan.is_v21() && plan.navigation != Selection::Map(id))
    {
        return Err("product_plan_map_domain_selection");
    }
    if (plan.navigation != Selection::None) != plan.offmesh.is_some()
        || plan
            .offmesh
            .as_ref()
            .is_some_and(|connections| connections.iter().any(|connection| connection.map_id != id))
    {
        return Err("product_plan_map_domain_offmesh");
    }
    Ok(())
}
