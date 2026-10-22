// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored 2026-10-11: explicit, canonical offline product selections.
use forever_casc::{Locale, local_index, package::Component, root};
use forever_db2::catalog;
use std::{collections::BTreeSet, fmt::Write};

mod publication;
pub use publication::PublicationProfile;
mod disk;
pub use disk::{CollisionStorage, DiskBudget};
mod offmesh;
pub use offmesh::{MAX_OFFMESH_CONNECTIONS, OFFMESH_PROFILE, OffMeshConnection};
mod map_domain;
pub use map_domain::{
    GAMEOBJECT_SCOPE_PROFILE, MAP_DOMAIN_PROFILE, MAP_DOMAIN_VERIFICATION_PROFILE,
    MAP_DOMAIN_VERIFY_SCOPE, PLACEMENT_ID_DOMAIN_PROFILE,
};

pub const AVAILABLE_TERRAIN_VERIFY_PROFILE: &str = "linux70374-available-map-terrain-product-v1";
pub const AVAILABLE_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_coverage_links";

pub const COMPATIBLE_TERRAIN_VERIFY_PROFILE: &str =
    "linux70374-compatible-available-map-terrain-product-v1";
pub const COMPATIBLE_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_omission_coverage_links";
pub const COMPATIBLE_TERRAIN_VERIFY_PROFILE_70450: &str =
    "linux70450-compatible-available-map-terrain-product-v1";
pub const COMPATIBLE_TERRAIN_VERIFY_SCOPE_70450: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_omission_coverage_links";

pub const MAX_PLAN_BYTES: usize = 128 * 1024;
pub const MAX_DB2_ENTRIES: usize = 1126;
pub const MAX_70205_DB2_ENTRIES: usize = 1161;
pub const MAX_70338_DB2_ENTRIES: usize = 1161;
pub const MAX_70374_DB2_ENTRIES: usize = 1161;
pub const MAX_70425_DB2_ENTRIES: usize = 1161;
pub const MAX_70450_DB2_ENTRIES: usize = 1161;
pub const COMPATIBLE_COLLISION_VERIFY_PROFILE: &str = "linux70425-compatible-gnu15-vmap-product-v1";
pub const COMPATIBLE_COLLISION_VERIFY_PROFILE_70450: &str =
    "linux70450-compatible-gnu15-vmap-product-v1";
pub const COMPATIBLE_COLLISION_VERIFY_SCOPE: &str = "artifact_integrity_complete_declared_vmap_associations_and_producer_reported_available_source_coverage";
pub const SELECTED_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-selected-map-geometry-vmap-product-v1";
pub const SELECTED_GEOMETRY_COVERAGE_PROFILE: &str = "linux70450-selected-map-geometry-coverage-v1";
pub const SELECTED_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-selected-map-geometry-verification-v1";
pub const SELECTED_GEOMETRY_VERIFY_SCOPE: &str =
    "artifact_integrity_selected_map_geometry_associations_and_declared_partial_source_coverage";
pub const SELECTED_GEOMETRY_GAMEOBJECT_SCOPE: &str = "excluded-by-profile-v1";
pub const SELECTED_GEOMETRY_MODEL_SCOPE: &str = "placement-only-v1";
pub const AUTHENTICATED_TERRAIN_VERIFY_PROFILE: &str =
    "linux70450-authenticated-available-map-terrain-product-v1";
pub const AUTHENTICATED_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_and_terrain_coverage_independently_derived_from_sealed_map_db2";

/// Explicit DB2 schema source. Existing product plans retain their legacy
/// pinned schema; builds 70205 and 70338 plans opt into separately pinned
/// metadata catalogs without relabeling older package identities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemaCatalog {
    #[default]
    PinnedMetadataV1,
    WowDbDefs70205V1,
    WowDbDefs70338V1,
    WowDbDefs70374V1,
    WowDbDefs70425V1,
    WowDbDefs70450V1,
}

impl SchemaCatalog {
    pub const fn id(self) -> &'static str {
        match self {
            Self::PinnedMetadataV1 => catalog::CATALOG_ID,
            Self::WowDbDefs70205V1 => catalog::WOWDBDEFS_70205_CATALOG_ID,
            Self::WowDbDefs70338V1 => catalog::WOWDBDEFS_70338_CATALOG_ID,
            Self::WowDbDefs70374V1 => catalog::WOWDBDEFS_70374_CATALOG_ID,
            Self::WowDbDefs70425V1 => catalog::WOWDBDEFS_70425_CATALOG_ID,
            Self::WowDbDefs70450V1 => catalog::WOWDBDEFS_70450_CATALOG_ID,
        }
    }

    const fn max_entries(self) -> usize {
        match self {
            Self::PinnedMetadataV1 => MAX_DB2_ENTRIES,
            Self::WowDbDefs70205V1 => MAX_70205_DB2_ENTRIES,
            Self::WowDbDefs70338V1 => MAX_70338_DB2_ENTRIES,
            Self::WowDbDefs70374V1 => MAX_70374_DB2_ENTRIES,
            Self::WowDbDefs70425V1 => MAX_70425_DB2_ENTRIES,
            Self::WowDbDefs70450V1 => MAX_70450_DB2_ENTRIES,
        }
    }
}

/// An explicit decoding capability, without key material or keyset identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decryption {
    Disabled,
    ProvidedSalsa20V1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub id: u32,
}

/// Terrain uses x=column/y=row. Navigation adapters must use row=y/column=x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    None,
    All,
    Map(u32),
    Tile { map: u32, x: u8, y: u8 },
}
impl Selection {
    fn parse(value: &str) -> Result<Self, &'static str> {
        let fields: Vec<_> = value.split(':').collect();
        match fields.as_slice() {
            ["none"] => Ok(Self::None),
            ["all"] => Ok(Self::All),
            ["map", map] => Ok(Self::Map(decimal(map)?)),
            ["tile", map, x, y] => {
                let x = decimal(x)?;
                let y = decimal(y)?;
                if x >= 64 || y >= 64 {
                    return Err("product_plan_selection");
                }
                Ok(Self::Tile {
                    map: decimal(map)?,
                    x: x as u8,
                    y: y as u8,
                })
            }
            _ => Err("product_plan_selection"),
        }
    }
    fn render(self) -> String {
        match self {
            Self::None => "none".into(),
            Self::All => "all".into(),
            Self::Map(map) => format!("map:{map}"),
            Self::Tile { map, x, y } => format!("tile:{map}:{x}:{y}"),
        }
    }
    fn validate(self) -> Result<(), &'static str> {
        if matches!(self, Self::Tile { x, y, .. } if x >= 64 || y >= 64) {
            Err("product_plan_selection")
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Explicit component publication contract; old plans retain capability inference.
    pub publication_profile: PublicationProfile,
    pub build: u32,
    pub schema_catalog: SchemaCatalog,
    pub locale: Locale,
    pub index_layout: local_index::Layout,
    pub root_layout: root::Layout,
    pub decryption: Decryption,
    pub collision_storage: CollisionStorage,
    /// Some explicitly selects Linux v8; v7 is reserved for the deferred draft.
    pub map_domain: Option<u32>,
    pub components: u8,
    pub db2: Vec<Entry>,
    pub terrain: Selection,
    pub navigation: Selection,
    pub pack: bool,
    pub big_base_unit: bool,
    pub skip_liquids: bool,
    pub maximum_slope: u8,
    pub non_steep_slope: u8,
    /// None retains v1-v5; Some (including empty) explicitly selects v6.
    pub offmesh: Option<Vec<OffMeshConnection>>,
}

impl Plan {
    /// Parse v1-v6 and the explicit Linux v8 grammar; v7 remains reserved.
    /// Exact re-rendering closes spelling,
    /// ordering, whitespace, numeric representation and final-newline aliases.
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        if input.len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        if !input.is_ascii()
            || input
                .bytes()
                .any(|b| !(b == b'\n' || b == b'\t' || (32..=126).contains(&b)))
        {
            return Err("product_plan_format");
        }
        let mut lines = input.split_terminator('\n');
        let version = match field(&mut lines, "forever-product")? {
            "1" => 1,
            "2" => 2,
            "3" => 3,
            "4" => 4,
            "5" => 5,
            "6" => 6,
            "8" => 8,
            "9" => 9,
            "10" => 10,
            "11" => 11,
            "12" => 12,
            "13" => 13,
            "14" => 14,
            "15" => 15,
            "16" => 16,
            "17" => 17,
            "18" => 18,
            "19" => 19,
            "20" => 20,
            "21" => 21,
            "22" => 22,
            "23" => 23,
            "24" => 24,
            _ => return Err("product_plan_version"),
        };
        let build = decimal(field(&mut lines, "build")?)?;
        let locale = match field(&mut lines, "locale")? {
            "enUS" => Locale::EnUs,
            "esES" => Locale::EsEs,
            _ => return Err("product_plan_locale"),
        };
        let index_layout = match field(&mut lines, "index")? {
            "contiguous" => local_index::Layout::Contiguous,
            "paged" => local_index::Layout::Paged,
            _ => return Err("product_plan_index"),
        };
        let root_layout = match field(&mut lines, "root")? {
            "headerless-18125" => root::Layout::Headerless18125,
            "header-30080" => root::Layout::Header30080,
            "header-50893-v1" => root::Layout::Header50893V1,
            "header-50893-v2" => root::Layout::Header50893V2,
            _ => return Err("product_plan_root"),
        };
        let schema_catalog = match version {
            4 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70205-v1" => SchemaCatalog::WowDbDefs70205V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            5 | 9 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70338-v1" => SchemaCatalog::WowDbDefs70338V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            14 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70425-v1" => SchemaCatalog::WowDbDefs70425V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            15..=24 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70450-v1" => SchemaCatalog::WowDbDefs70450V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            10..=13 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70374-v1" => SchemaCatalog::WowDbDefs70374V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            _ => SchemaCatalog::PinnedMetadataV1,
        };
        let decryption = match version {
            1 => Decryption::Disabled,
            2 => match field(&mut lines, "decryption")? {
                "provided-salsa20-v1" => Decryption::ProvidedSalsa20V1,
                _ => return Err("product_plan_decryption"),
            },
            3..=6 | 8..=24 => match field(&mut lines, "decryption")? {
                "disabled" => Decryption::Disabled,
                "provided-salsa20-v1" => Decryption::ProvidedSalsa20V1,
                _ => return Err("product_plan_decryption"),
            },
            _ => unreachable!("version was validated above"),
        };
        let collision_storage = if matches!(version, 3 | 8 | 14 | 15 | 18 | 21 | 22)
            || (matches!(version, 4..=6)
                && lines
                    .clone()
                    .next()
                    .is_some_and(|line| line.starts_with("collision-storage\t")))
        {
            CollisionStorage::parse(&mut lines)?
        } else {
            CollisionStorage::Resident
        };
        let map_domain = if version == 22 {
            if field(&mut lines, "collision-profile")? != SELECTED_GEOMETRY_COLLISION_PROFILE {
                return Err("product_plan_collision_profile");
            }
            let domain = decimal(
                field(&mut lines, "collision-domain")?
                    .strip_prefix("map:")
                    .ok_or("product_plan_collision_domain")?,
            )?;
            if field(&mut lines, "collision-coverage")? != SELECTED_GEOMETRY_COVERAGE_PROFILE {
                return Err("product_plan_collision_coverage");
            }
            if field(&mut lines, "gameobject-scope")? != SELECTED_GEOMETRY_GAMEOBJECT_SCOPE {
                return Err("product_plan_gameobject_scope");
            }
            if field(&mut lines, "model-scope")? != SELECTED_GEOMETRY_MODEL_SCOPE {
                return Err("product_plan_model_scope");
            }
            Some(domain)
        } else if matches!(version, 14 | 15 | 18) {
            let profile = if version == 14 {
                COMPATIBLE_COLLISION_VERIFY_PROFILE
            } else {
                COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
            };
            if field(&mut lines, "collision-profile")? != profile {
                return Err("product_plan_collision_profile");
            }
            let domain = match field(&mut lines, "collision-domain")? {
                "all-available-maps" => None,
                text => Some(decimal(
                    text.strip_prefix("map:")
                        .ok_or("product_plan_collision_domain")?,
                )?),
            };
            if field(&mut lines, "collision-coverage")? != "compatible-collision-coverage-v1" {
                return Err("product_plan_collision_coverage");
            }
            domain
        } else if matches!(version, 8 | 21) {
            Some(map_domain::parse(&mut lines)?)
        } else {
            None
        };
        let mut components = 0;
        for name in field(&mut lines, "components")?.split(',') {
            let component = Component::parse(name).map_err(|_| "product_plan_components")?;
            if components & component.bit() != 0 {
                return Err("product_plan_components");
            }
            components |= component.bit();
        }
        let terrain_text = field(&mut lines, "terrain")?;
        let terrain =
            if matches!(version, 12 | 13 | 20 | 24) && terrain_text == "all-available-maps" {
                Selection::All
            } else {
                let selected = Selection::parse(terrain_text)?;
                if matches!(version, 12 | 13 | 20 | 24) && selected == Selection::All {
                    return Err("product_plan_selection");
                }
                selected
            };
        let pack = boolean(field(&mut lines, "pack")?)?;
        let navigation = Selection::parse(field(&mut lines, "navigation")?)?;
        let big_base_unit = boolean(field(&mut lines, "big-base-unit")?)?;
        let skip_liquids = boolean(field(&mut lines, "skip-liquids")?)?;
        let maximum_slope = decimal(field(&mut lines, "maximum-slope")?)?;
        let non_steep_slope = decimal(field(&mut lines, "non-steep-slope")?)?;
        if maximum_slope > 90 || non_steep_slope > 90 {
            return Err("product_plan_slopes");
        }
        let offmesh = if version == 6
            || (version == 8 && navigation != Selection::None)
            || (version == 21 && navigation != Selection::None)
        {
            if field(&mut lines, "offmesh")? != OFFMESH_PROFILE {
                return Err("product_plan_offmesh");
            }
            let mut connections = Vec::new();
            while lines
                .clone()
                .next()
                .is_some_and(|line| line.starts_with("offmesh-connection\t"))
            {
                if connections.len() == MAX_OFFMESH_CONNECTIONS {
                    return Err("product_plan_offmesh_limit");
                }
                connections.push(OffMeshConnection::parse(
                    lines.next().ok_or("product_plan_format")?,
                )?);
            }
            Some(connections)
        } else {
            None
        };
        let mut db2 = Vec::new();
        for line in lines {
            if db2.len() == schema_catalog.max_entries() {
                return Err("product_plan_size");
            }
            let fields: Vec<_> = line.split('\t').collect();
            let ["db2", name, id] = fields.as_slice() else {
                return Err("product_plan_format");
            };
            db2.push(Entry {
                name: (*name).into(),
                id: decimal(id)?,
            });
        }
        let plan = Self {
            publication_profile: if version == 17 {
                PublicationProfile::Build70450Db2ProvidedKeysV1
            } else if version == 18 {
                PublicationProfile::Build70450CollisionProvidedKeysV1
            } else if version == 19 {
                PublicationProfile::Build70450Db2GtCamerasV1
            } else if version == 20 {
                PublicationProfile::Build70450CompatibleTerrainV1
            } else if version == 21 {
                PublicationProfile::Build70450NavigationV1
            } else if version == 22 {
                PublicationProfile::Build70450SelectedGeometryV1
            } else if version == 23 {
                PublicationProfile::Build70450SelectedNavigationV1
            } else if version == 24 {
                PublicationProfile::Build70450AuthenticatedTerrainV1
            } else if version == 16 {
                PublicationProfile::Build70450Db2OnlyV1
            } else if version == 15 {
                PublicationProfile::Build70450CompatibleCollisionV1
            } else if version == 14 {
                PublicationProfile::Build70425CompatibleCollisionV1
            } else if version == 13 {
                PublicationProfile::Build70374CompatibleTerrainV1
            } else if version == 12 {
                PublicationProfile::Build70374AvailableTerrainV1
            } else if version == 9 {
                PublicationProfile::Build70338Db2GtCamerasV1
            } else if version == 11 {
                PublicationProfile::Build70374Db2GtCamerasV1
            } else if version == 10 {
                PublicationProfile::Build70374Db2OnlyV1
            } else {
                PublicationProfile::Legacy
            },
            build,
            schema_catalog,
            locale,
            index_layout,
            root_layout,
            decryption,
            collision_storage,
            map_domain,
            components,
            db2,
            terrain,
            navigation,
            pack,
            big_base_unit,
            skip_liquids,
            maximum_slope: maximum_slope as u8,
            non_steep_slope: non_steep_slope as u8,
            offmesh,
        };
        plan.validate()?;
        if plan.render() != input {
            return Err("product_plan_noncanonical");
        }
        Ok(plan)
    }

    /// Validate manually constructed plans before any source or output I/O.
    pub fn validate(&self) -> Result<(), &'static str> {
        self.publication_profile.validate(self)?;
        if !self.is_v14()
            && !self.is_v15()
            && !self.is_v18()
            && !self.is_v22()
            && !self.is_v23()
            && !self.is_v24()
        {
            map_domain::validate(self)?;
        }
        if let Some(connections) = &self.offmesh {
            let legacy = self.schema_catalog == SchemaCatalog::PinnedMetadataV1
                && [70170, 70205].contains(&self.build);
            let build_70450 = self.is_v21()
                && self.build == 70450
                && self.schema_catalog == SchemaCatalog::WowDbDefs70450V1;
            if !(legacy || build_70450) || !self.contains(Component::Navigation) {
                return Err("product_plan_offmesh_scope");
            }
            if connections.len() > MAX_OFFMESH_CONNECTIONS {
                return Err("product_plan_offmesh_limit");
            }
            for connection in connections {
                connection.validate()?;
            }
        }
        if ![70170, 70205, 70338].contains(&self.build)
            && !self.is_v10()
            && !self.is_v11()
            && !self.is_available_terrain()
            && !self.is_v14()
            && !self.is_v15()
            && !self.is_v16()
            && !self.is_v17()
            && !self.is_v18()
            && !self.is_v19()
            && !self.is_v20()
            && !self.is_v21()
            && !self.is_v22()
            && !self.is_v23()
            && !self.is_v24()
        {
            return Err("product_plan_build");
        }
        if self.schema_catalog == SchemaCatalog::WowDbDefs70205V1 && self.build != 70205 {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70338V1 && self.build != 70338)
            || (self.build == 70338 && self.schema_catalog != SchemaCatalog::WowDbDefs70338V1)
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70374V1 || self.build == 70374)
            && !self.is_v10()
            && !self.is_v11()
            && !self.is_available_terrain()
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70425V1 || self.build == 70425)
            && !self.is_v14()
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70450V1 || self.build == 70450)
            && !self.is_v15()
            && !self.is_v16()
            && !self.is_v17()
            && !self.is_v18()
            && !self.is_v19()
            && !self.is_v20()
            && !self.is_v21()
            && !self.is_v22()
            && !self.is_v23()
            && !self.is_v24()
        {
            return Err("product_plan_schema_catalog");
        }
        if self.publication_profile == PublicationProfile::Legacy
            && self.build == 70338
            && self.components != Component::Db2.bit()
        {
            return Err("product_plan_component_scope");
        }
        if self.components == 0 || self.components & !63 != 0 {
            return Err("product_plan_components");
        }
        if let CollisionStorage::LeasedDiskV1(budget) = self.collision_storage {
            if !self.contains(Component::Collision) {
                return Err("product_plan_dependencies");
            }
            budget.validate()?;
        }
        if self.db2.len() > self.schema_catalog.max_entries() {
            return Err("product_plan_size");
        }
        let mut ids = BTreeSet::new();
        let mut previous: Option<&str> = None;
        for entry in &self.db2 {
            let metadata = match self.schema_catalog {
                SchemaCatalog::PinnedMetadataV1 => {
                    catalog::find(&entry.name).ok_or("product_plan_schema")?
                }
                SchemaCatalog::WowDbDefs70205V1 => {
                    catalog::resolve_for(self.schema_catalog.id(), &entry.name)
                        .map_err(|_| "product_plan_schema")?
                        .ok_or("product_plan_schema")?
                        .metadata()
                }
                SchemaCatalog::WowDbDefs70338V1
                | SchemaCatalog::WowDbDefs70374V1
                | SchemaCatalog::WowDbDefs70425V1
                | SchemaCatalog::WowDbDefs70450V1 => {
                    catalog::resolve_for(self.schema_catalog.id(), &entry.name)
                        .map_err(|_| "product_plan_schema")?
                        .ok_or("product_plan_schema")?
                        .metadata()
                }
            };
            if metadata.file_data_id != entry.id
                || !ids.insert(entry.id)
                || previous.is_some_and(|name| name >= entry.name.as_str())
            {
                return Err("product_plan_schema");
            }
            previous = Some(&entry.name);
        }
        if self.contains(Component::Db2) == self.db2.is_empty() {
            return Err("product_plan_dependencies");
        }
        self.terrain.validate()?;
        self.navigation.validate()?;
        if self.contains(Component::Terrain) == (self.terrain == Selection::None)
            || self.contains(Component::Navigation) == (self.navigation == Selection::None)
        {
            return Err("product_plan_dependencies");
        }
        if self.maximum_slope > 90 || self.non_steep_slope > self.maximum_slope {
            return Err("product_plan_slopes");
        }
        let has_db2 = |name: &str| self.db2.iter().any(|entry| entry.name == name);
        if self.contains(Component::Cameras)
            && (!self.contains(Component::Db2) || !has_db2("CinematicCamera"))
        {
            return Err("product_plan_dependencies");
        }
        if self.contains(Component::Navigation) && !self.is_v23() {
            if !self.contains(Component::Terrain)
                || !self.contains(Component::Collision)
                || !self.contains(Component::Db2)
                || (self.map_domain.is_none() && self.terrain != Selection::All)
                || !has_db2("Map")
                || !has_db2("LiquidType")
            {
                return Err("product_plan_dependencies");
            }
        } else if self.big_base_unit
            || self.skip_liquids
            || self.maximum_slope != 55
            || self.non_steep_slope != 55
        {
            return Err("product_plan_options");
        }
        if self.try_render()?.len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        Ok(())
    }

    pub const fn contains(&self, component: Component) -> bool {
        self.components & component.bit() != 0
    }

    pub const fn version(&self) -> u8 {
        if self.is_v24() {
            return 24;
        }
        if self.is_v23() {
            return 23;
        }
        if self.is_v22() {
            return 22;
        }
        if self.is_v21() {
            return 21;
        }
        if self.is_v20() {
            return 20;
        }
        if self.is_v19() {
            return 19;
        }
        if self.is_v18() {
            return 18;
        }
        if self.is_v17() {
            return 17;
        }
        if self.is_v16() {
            return 16;
        }
        if self.is_v15() {
            return 15;
        }
        if self.is_v14() {
            return 14;
        }
        if self.is_v13() {
            return 13;
        }
        if self.is_v12() {
            return 12;
        }
        if self.is_v11() {
            return 11;
        }
        if self.is_v10() {
            return 10;
        }
        if self.is_v9() {
            return 9;
        }
        if self.map_domain.is_some() {
            return 8;
        }
        if self.offmesh.is_some() {
            return 6;
        }
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return 4,
            SchemaCatalog::WowDbDefs70338V1 => return 5,
            SchemaCatalog::WowDbDefs70374V1
            | SchemaCatalog::WowDbDefs70425V1
            | SchemaCatalog::WowDbDefs70450V1 => {}
            SchemaCatalog::PinnedMetadataV1 => {}
        }
        if matches!(self.collision_storage, CollisionStorage::LeasedDiskV1(_)) {
            return 3;
        }
        match self.decryption {
            Decryption::Disabled => 1,
            Decryption::ProvidedSalsa20V1 => 2,
        }
    }

    pub const fn contract(&self) -> &'static str {
        if self.is_v24() {
            return "forever-product-v24";
        }
        if self.is_v23() {
            return "forever-product-v23";
        }
        if self.is_v22() {
            return "forever-product-v22";
        }
        if self.is_v21() {
            return "forever-product-v21";
        }
        if self.is_v20() {
            return "forever-product-v20";
        }
        if self.is_v19() {
            return "forever-product-v19";
        }
        if self.is_v18() {
            return "forever-product-v18";
        }
        if self.is_v17() {
            return "forever-product-v17";
        }
        if self.is_v16() {
            return "forever-product-v16";
        }
        if self.is_v15() {
            return "forever-product-v15";
        }
        if self.is_v14() {
            return "forever-product-v14";
        }
        if self.is_v13() {
            return "forever-product-v13";
        }
        if self.is_v12() {
            return "forever-product-v12";
        }
        if self.is_v11() {
            return "forever-product-v11";
        }
        if self.is_v10() {
            return "forever-product-v10";
        }
        if self.is_v9() {
            return "forever-product-v9";
        }
        if self.map_domain.is_some() {
            return "forever-product-v8";
        }
        if self.offmesh.is_some() {
            return "forever-product-v6";
        }
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return "forever-product-v4",
            SchemaCatalog::WowDbDefs70338V1 => return "forever-product-v5",
            SchemaCatalog::WowDbDefs70374V1
            | SchemaCatalog::WowDbDefs70425V1
            | SchemaCatalog::WowDbDefs70450V1 => {}
            SchemaCatalog::PinnedMetadataV1 => {}
        }
        if matches!(self.collision_storage, CollisionStorage::LeasedDiskV1(_)) {
            return "forever-product-v3";
        }
        match self.decryption {
            Decryption::Disabled => "forever-product-v1",
            Decryption::ProvidedSalsa20V1 => "forever-product-v2",
        }
    }

    pub const fn is_v9(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70338Db2GtCamerasV1
        )
    }

    pub const fn is_v10(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374Db2OnlyV1
        )
    }

    pub const fn is_v12(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374AvailableTerrainV1
        )
    }

    /// Explicit Linux70425 disabled-key compatible GNU15-v2 collision publication.
    pub const fn is_v14(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70425CompatibleCollisionV1
        )
    }

    /// Explicit Linux70450 disabled-key compatible GNU15-v2 collision publication.
    pub const fn is_v15(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CompatibleCollisionV1
        )
    }

    /// Explicit Linux build70450 disabled-key DB2-only publication.
    pub const fn is_v16(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2OnlyV1
        )
    }

    /// Explicit Linux build70450 DB2-only publication with supplied keys.
    pub const fn is_v17(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2ProvidedKeysV1
        )
    }

    /// Explicit Linux build70450 compatible GNU15-v2 collision with supplied keys.
    pub const fn is_v18(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CollisionProvidedKeysV1
        )
    }

    /// Explicit Linux build70450 DB2+GT+cameras publication.
    pub const fn is_v19(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2GtCamerasV1
        )
    }

    /// Explicit Linux build70450 compatible available-Map terrain publication.
    pub const fn is_v20(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CompatibleTerrainV1
        )
    }

    /// Explicit Linux build70450 compatible map-domain navigation publication.
    pub const fn is_v21(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450NavigationV1
        )
    }

    /// Explicit Linux build-70450 selected-map collision-geometry product.
    pub const fn is_v22(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SelectedGeometryV1
        )
    }

    /// Explicit Linux build-70450 selected navigation over sealed inputs.
    pub const fn is_v23(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SelectedNavigationV1
        )
    }

    /// Explicit Linux build-70450 authenticated Map+terrain product.
    pub const fn is_v24(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AuthenticatedTerrainV1
        )
    }

    pub const fn is_v13(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374CompatibleTerrainV1
        )
    }

    pub const fn is_available_terrain(&self) -> bool {
        self.is_v12() || self.is_v13() || self.is_v20() || self.is_v24()
    }

    pub const fn is_any_available_terrain(&self) -> bool {
        self.is_available_terrain() || self.is_v20() || self.is_v24()
    }

    pub const fn is_compatible_terrain(&self) -> bool {
        self.is_v13() || self.is_v20()
    }

    pub const fn is_v11(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374Db2GtCamerasV1
        )
    }

    pub const fn is_gt_cameras(&self) -> bool {
        self.is_v9() || self.is_v11() || self.is_v19()
    }

    pub const fn requires_keys(&self) -> bool {
        matches!(self.decryption, Decryption::ProvidedSalsa20V1)
    }

    /// Compatibility renderer for already-validated plans.
    ///
    /// Use `try_render` when a plan may have been constructed or modified
    /// directly; this helper returns an empty string rather than panicking if
    /// rendering cannot satisfy the v22 map-domain invariant.
    pub fn render(&self) -> String {
        self.try_render().unwrap_or_default()
    }

    /// Emit a canonical plan or return a typed plan error for an incomplete
    /// manually-constructed profile.
    pub fn try_render(&self) -> Result<String, &'static str> {
        let index = match self.index_layout {
            local_index::Layout::Contiguous => "contiguous",
            local_index::Layout::Paged => "paged",
        };
        let root = match self.root_layout {
            root::Layout::Headerless18125 => "headerless-18125",
            root::Layout::Header30080 => "header-30080",
            root::Layout::Header50893V1 => "header-50893-v1",
            root::Layout::Header50893V2 => "header-50893-v2",
        };
        let components = Component::ALL
            .into_iter()
            .filter(|component| self.contains(*component))
            .map(Component::name)
            .collect::<Vec<_>>()
            .join(",");
        let schema_catalog = match self.schema_catalog {
            SchemaCatalog::PinnedMetadataV1 => "",
            SchemaCatalog::WowDbDefs70205V1 => "schema-catalog\twowdbdefs-70205-v1\n",
            SchemaCatalog::WowDbDefs70338V1 => "schema-catalog\twowdbdefs-70338-v1\n",
            SchemaCatalog::WowDbDefs70374V1 => "schema-catalog\twowdbdefs-70374-v1\n",
            SchemaCatalog::WowDbDefs70425V1 => "schema-catalog\twowdbdefs-70425-v1\n",
            SchemaCatalog::WowDbDefs70450V1 => "schema-catalog\twowdbdefs-70450-v1\n",
        };
        let decryption = match (self.version(), self.decryption) {
            (3..=6 | 8..=24, Decryption::Disabled) => "decryption\tdisabled\n",
            (_, Decryption::Disabled) => "",
            (_, Decryption::ProvidedSalsa20V1) => "decryption\tprovided-salsa20-v1\n",
        };
        let collision_storage = self.collision_storage.render();
        let map_domain = if self.is_v22() {
            let id = self.map_domain.ok_or("product_plan_map_domain")?;
            format!(
                "collision-profile\t{SELECTED_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tmap:{id}\ncollision-coverage\t{SELECTED_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v14() || self.is_v15() || self.is_v18() {
            let selected = self
                .map_domain
                .map_or_else(|| "all-available-maps".to_owned(), |id| format!("map:{id}"));
            let profile = if self.is_v15() || self.is_v18() {
                COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
            } else {
                COMPATIBLE_COLLISION_VERIFY_PROFILE
            };
            format!(
                "collision-profile\t{profile}\ncollision-domain\t{selected}\ncollision-coverage\tcompatible-collision-coverage-v1\n"
            )
        } else {
            map_domain::render(self.map_domain)
        };
        let mut output = format!(
            "forever-product\t{}\nbuild\t{}\nlocale\t{}\nindex\t{index}\nroot\t{root}\n{schema_catalog}{decryption}{collision_storage}{map_domain}components\t{components}\nterrain\t{}\npack\t{}\nnavigation\t{}\nbig-base-unit\t{}\nskip-liquids\t{}\nmaximum-slope\t{}\nnon-steep-slope\t{}\n",
            self.version(),
            self.build,
            self.locale.name(),
            if self.is_any_available_terrain() && self.terrain == Selection::All {
                "all-available-maps".into()
            } else {
                self.terrain.render()
            },
            self.pack,
            self.navigation.render(),
            self.big_base_unit,
            self.skip_liquids,
            self.maximum_slope,
            self.non_steep_slope,
        );
        if let Some(connections) = &self.offmesh {
            writeln!(output, "offmesh\t{OFFMESH_PROFILE}")
                .expect("writing into String cannot fail");
            for connection in connections {
                connection.render(&mut output);
            }
        }
        for entry in &self.db2 {
            writeln!(output, "db2\t{}\t{}", entry.name, entry.id)
                .expect("writing into String cannot fail");
        }
        Ok(output)
    }
}

fn field<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let line = lines.next().ok_or("product_plan_format")?;
    let (name, value) = line.split_once('\t').ok_or("product_plan_format")?;
    if name != key || value.is_empty() || value.contains('\t') {
        return Err("product_plan_format");
    }
    Ok(value)
}
fn decimal(value: &str) -> Result<u32, &'static str> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("product_plan_decimal");
    }
    value.parse().map_err(|_| "product_plan_decimal")
}
fn boolean(value: &str) -> Result<bool, &'static str> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("product_plan_boolean"),
    }
}

#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod disk_tests;
#[cfg(test)]
mod map_domain_tests;
#[cfg(test)]
pub(crate) mod offmesh_tests;
#[cfg(test)]
mod v10_tests;
#[cfg(test)]
mod v9_tests;

#[cfg(test)]
mod v11_tests;

#[cfg(test)]
mod v12_tests;

#[cfg(test)]
mod v13_tests;

#[cfg(test)]
pub(crate) mod v14_tests;

#[cfg(test)]
pub(crate) mod v15_tests;

#[cfg(test)]
mod v16_tests;

#[cfg(test)]
mod v17_tests;

#[cfg(test)]
mod v18_tests;

#[cfg(test)]
mod v19_tests;

#[cfg(test)]
mod v20_tests;

#[cfg(test)]
mod v21_tests;

#[cfg(test)]
mod v22_tests;
#[cfg(test)]
mod v23_tests;

#[cfg(test)]
mod v24_tests;
