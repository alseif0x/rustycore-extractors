// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit, canonical offline product selections.
use forever_casc::{Locale, local_index, package::Component, root};
use forever_db2::catalog;
use std::{collections::BTreeSet, fmt::Write};

mod disk;
pub use disk::{CollisionStorage, DiskBudget};

pub const MAX_PLAN_BYTES: usize = 128 * 1024;
pub const MAX_DB2_ENTRIES: usize = 1126;
pub const MAX_70205_DB2_ENTRIES: usize = 1161;
pub const MAX_70338_DB2_ENTRIES: usize = 1161;

/// Explicit DB2 schema source. Existing product plans retain their legacy
/// pinned schema; builds 70205 and 70338 plans opt into separately pinned
/// metadata catalogs without relabeling older package identities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemaCatalog {
    #[default]
    PinnedMetadataV1,
    WowDbDefs70205V1,
    WowDbDefs70338V1,
}

impl SchemaCatalog {
    pub const fn id(self) -> &'static str {
        match self {
            Self::PinnedMetadataV1 => catalog::CATALOG_ID,
            Self::WowDbDefs70205V1 => catalog::WOWDBDEFS_70205_CATALOG_ID,
            Self::WowDbDefs70338V1 => catalog::WOWDBDEFS_70338_CATALOG_ID,
        }
    }

    const fn max_entries(self) -> usize {
        match self {
            Self::PinnedMetadataV1 => MAX_DB2_ENTRIES,
            Self::WowDbDefs70205V1 => MAX_70205_DB2_ENTRIES,
            Self::WowDbDefs70338V1 => MAX_70338_DB2_ENTRIES,
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
    pub build: u32,
    pub schema_catalog: SchemaCatalog,
    pub locale: Locale,
    pub index_layout: local_index::Layout,
    pub root_layout: root::Layout,
    pub decryption: Decryption,
    pub collision_storage: CollisionStorage,
    pub components: u8,
    pub db2: Vec<Entry>,
    pub terrain: Selection,
    pub navigation: Selection,
    pub pack: bool,
    pub big_base_unit: bool,
    pub skip_liquids: bool,
    pub maximum_slope: u8,
    pub non_steep_slope: u8,
}

impl Plan {
    /// Parse the complete v1-v5 grammar. Exact re-rendering closes spelling,
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
            5 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70338-v1" => SchemaCatalog::WowDbDefs70338V1,
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
            3..=5 => match field(&mut lines, "decryption")? {
                "disabled" => Decryption::Disabled,
                "provided-salsa20-v1" => Decryption::ProvidedSalsa20V1,
                _ => return Err("product_plan_decryption"),
            },
            _ => unreachable!("version was validated above"),
        };
        let collision_storage = if version == 3
            || (matches!(version, 4 | 5)
                && lines
                    .clone()
                    .next()
                    .is_some_and(|line| line.starts_with("collision-storage\t")))
        {
            CollisionStorage::parse(&mut lines)?
        } else {
            CollisionStorage::Resident
        };
        let mut components = 0;
        for name in field(&mut lines, "components")?.split(',') {
            let component = Component::parse(name).map_err(|_| "product_plan_components")?;
            if components & component.bit() != 0 {
                return Err("product_plan_components");
            }
            components |= component.bit();
        }
        let terrain = Selection::parse(field(&mut lines, "terrain")?)?;
        let pack = boolean(field(&mut lines, "pack")?)?;
        let navigation = Selection::parse(field(&mut lines, "navigation")?)?;
        let big_base_unit = boolean(field(&mut lines, "big-base-unit")?)?;
        let skip_liquids = boolean(field(&mut lines, "skip-liquids")?)?;
        let maximum_slope = decimal(field(&mut lines, "maximum-slope")?)?;
        let non_steep_slope = decimal(field(&mut lines, "non-steep-slope")?)?;
        if maximum_slope > 90 || non_steep_slope > 90 {
            return Err("product_plan_slopes");
        }
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
            build,
            schema_catalog,
            locale,
            index_layout,
            root_layout,
            decryption,
            collision_storage,
            components,
            db2,
            terrain,
            navigation,
            pack,
            big_base_unit,
            skip_liquids,
            maximum_slope: maximum_slope as u8,
            non_steep_slope: non_steep_slope as u8,
        };
        plan.validate()?;
        if plan.render() != input {
            return Err("product_plan_noncanonical");
        }
        Ok(plan)
    }

    /// Validate manually constructed plans before any source or output I/O.
    pub fn validate(&self) -> Result<(), &'static str> {
        if ![70170, 70205, 70338].contains(&self.build) {
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
        if self.build == 70338 && self.components != Component::Db2.bit() {
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
                SchemaCatalog::WowDbDefs70338V1 => {
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
        if self.contains(Component::Navigation) {
            if !self.contains(Component::Terrain)
                || !self.contains(Component::Collision)
                || !self.contains(Component::Db2)
                || self.terrain != Selection::All
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
        if self.render().len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        Ok(())
    }

    pub const fn contains(&self, component: Component) -> bool {
        self.components & component.bit() != 0
    }

    pub const fn version(&self) -> u8 {
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return 4,
            SchemaCatalog::WowDbDefs70338V1 => return 5,
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
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return "forever-product-v4",
            SchemaCatalog::WowDbDefs70338V1 => return "forever-product-v5",
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

    pub const fn requires_keys(&self) -> bool {
        matches!(self.decryption, Decryption::ProvidedSalsa20V1)
    }

    /// Emit exactly one newline-terminated canonical plan. Call validate first
    /// when the public fields were constructed or modified directly.
    pub fn render(&self) -> String {
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
        };
        let decryption = match (self.version(), self.decryption) {
            (3..=5, Decryption::Disabled) => "decryption\tdisabled\n",
            (_, Decryption::Disabled) => "",
            (_, Decryption::ProvidedSalsa20V1) => "decryption\tprovided-salsa20-v1\n",
        };
        let collision_storage = self.collision_storage.render();
        let mut output = format!(
            "forever-product\t{}\nbuild\t{}\nlocale\t{}\nindex\t{index}\nroot\t{root}\n{schema_catalog}{decryption}{collision_storage}components\t{components}\nterrain\t{}\npack\t{}\nnavigation\t{}\nbig-base-unit\t{}\nskip-liquids\t{}\nmaximum-slope\t{}\nnon-steep-slope\t{}\n",
            self.version(),
            self.build,
            self.locale.name(),
            self.terrain.render(),
            self.pack,
            self.navigation.render(),
            self.big_base_unit,
            self.skip_liquids,
            self.maximum_slope,
            self.non_steep_slope,
        );
        for entry in &self.db2 {
            writeln!(output, "db2\t{}\t{}", entry.name, entry.id)
                .expect("writing into String cannot fail");
        }
        output
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
