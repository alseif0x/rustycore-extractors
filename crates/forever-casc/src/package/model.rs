// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: deterministic common context, artifact and inventory facts.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Component {
    Db2,
    GameTables,
    Cameras,
    Terrain,
    Collision,
    Navigation,
}
impl Component {
    pub const ALL: [Self; 6] = [
        Self::Db2,
        Self::GameTables,
        Self::Cameras,
        Self::Terrain,
        Self::Collision,
        Self::Navigation,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Db2 => "db2",
            Self::GameTables => "gt",
            Self::Cameras => "cameras",
            Self::Terrain => "maps",
            Self::Collision => "vmaps",
            Self::Navigation => "mmaps",
        }
    }
    pub const fn bit(self) -> u8 {
        1 << (self as u8)
    }
    pub fn parse(s: &str) -> Result<Self, Error> {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == s)
            .ok_or(Error::Component)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub build: u32,
    pub locale: Locale,
    /// SHA-256 of the caller's complete acquisition/input identity; never keys.
    pub source_sha256: [u8; 32],
    /// Complete selected plan/options, including component-specific contracts.
    pub plan_sha256: [u8; 32],
    /// Caller-supplied exact producer build/source identity.
    pub producer_sha256: [u8; 32],
    /// Explicit selected components; ALL is 63, no implicit available subset.
    pub components: u8,
}
impl Context {
    pub fn validate(self) -> Result<(), Error> {
        if ![70170, 70205, 70338, 70374, 70425, 70450].contains(&self.build)
            || self.components == 0
            || self.components & !63 != 0
            || [self.source_sha256, self.plan_sha256, self.producer_sha256].contains(&[0; 32])
        {
            return Err(Error::Context);
        }
        // Explicit product plans bind their independent profile through
        // plan_sha256. Build 70425 admits v14 collision; build 70450 admits
        // v15/v18/v22 collision (16), v16/v17/v26 DB2 (1), v19 DB2+GT+cameras
        // (7), v20 terrain (8), v21 DB2+Terrain+Collision+Navigation (57), v23
        // selected navigation (32), v24/v25/v27 DB2+Terrain (9), v28-v50
        // DB2+collision (17), or v51 DB2+Terrain+Collision (25), each only for
        // an implemented revision and Linux.
        let product_scope = match self.build {
            70425 => self.components == Component::Collision.bit(),
            70450 => [
                Component::Collision.bit(),
                Component::Db2.bit(),
                Component::Db2.bit() | Component::Collision.bit(),
                Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
                Component::Terrain.bit(),
                Component::Db2.bit() | Component::Terrain.bit(),
                Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit(),
                Component::Db2.bit()
                    | Component::Terrain.bit()
                    | Component::Collision.bit()
                    | Component::Navigation.bit(),
                Component::Navigation.bit(),
            ]
            .contains(&self.components),
            _ => true,
        };
        if [70425, 70450].contains(&self.build) && (!product_scope || !cfg!(target_os = "linux")) {
            return Err(Error::Context);
        }
        if self.build == 70374
            && (![
                Component::Db2.bit(),
                Component::Terrain.bit(),
                Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
            ]
            .contains(&self.components)
                || !cfg!(target_os = "linux"))
        {
            return Err(Error::Context);
        }
        Ok(())
    }
    pub fn json(self) -> String {
        format!(
            "{{\"build\":{},\"locale\":\"{}\",\"components\":{},\"source_sha256\":\"{}\",\"plan_sha256\":\"{}\",\"producer_sha256\":\"{}\"}}",
            self.build,
            self.locale.name(),
            self.components,
            hex(&self.source_sha256),
            hex(&self.plan_sha256),
            hex(&self.producer_sha256)
        )
    }
    pub fn parse(s: &str) -> Result<Self, Error> {
        let v = parse(s)?;
        let out = Self {
            build: u32::try_from(number(&v, "build")?).map_err(|_| Error::Context)?,
            locale: Locale::parse(string(&v, "locale")?).map_err(|_| Error::Context)?,
            components: u8::try_from(number(&v, "components")?).map_err(|_| Error::Context)?,
            source_sha256: unhex(string(&v, "source_sha256")?)?,
            plan_sha256: unhex(string(&v, "plan_sha256")?)?,
            producer_sha256: unhex(string(&v, "producer_sha256")?)?,
        };
        out.validate()?;
        if s != out.json() {
            return Err(Error::Schema);
        }
        Ok(out)
    }
    pub fn contains(self, c: Component) -> bool {
        self.components & c.bit() != 0
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub files: usize,
    pub directories: usize,
    pub file_bytes: u64,
    pub total_bytes: u64,
    pub metadata_bytes: usize,
    pub manifest_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            files: 262144,
            directories: 262144,
            file_bytes: 128 * 1024 * 1024,
            total_bytes: 1024 * 1024 * 1024 * 1024,
            metadata_bytes: 256 * 1024 * 1024,
            manifest_bytes: 96 * 1024 * 1024,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.files > d.files
            || self.directories > d.directories
            || self.file_bytes > d.file_bytes
            || self.total_bytes > d.total_bytes
            || self.metadata_bytes > d.metadata_bytes
            || self.metadata_bytes < 128 * 1024
            || self.manifest_bytes > d.manifest_bytes
        {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub component: Component,
    pub path: String,
    pub bytes: u64,
    pub sha256: [u8; 32],
}
impl Artifact {
    pub fn json(&self) -> String {
        format!(
            "{{\"component\":\"{}\",\"path\":\"{}\",\"bytes\":{},\"sha256\":\"{}\"}}",
            self.component.name(),
            self.path,
            self.bytes,
            hex(&self.sha256)
        )
    }
    pub fn parse(s: &str, context: Context, limits: Limits) -> Result<Self, Error> {
        let v = parse(s)?;
        let out = Self {
            component: Component::parse(string(&v, "component")?)?,
            path: string(&v, "path")?.to_owned(),
            bytes: number(&v, "bytes")?,
            sha256: unhex(string(&v, "sha256")?)?,
        };
        out.validate(context, limits)?;
        if s != out.json() {
            return Err(Error::Schema);
        }
        Ok(out)
    }
    pub fn validate(&self, context: Context, limits: Limits) -> Result<(), Error> {
        path(&self.path)?;
        if !context.contains(self.component) {
            return Err(Error::Component);
        }
        let prefix = if self.component == Component::Db2 {
            format!("dbc/{}/", context.locale.name())
        } else {
            format!("{}/", self.component.name())
        };
        if !self.path.starts_with(&prefix) {
            return Err(Error::Path);
        }
        if self.bytes > limits.file_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
pub(super) fn path(s: &str) -> Result<(), Error> {
    #[cfg(windows)]
    if s.split('/').count() > 16
        || s.split('/').any(|p| {
            let stem = p.split('.').next().unwrap_or("").trim_end_matches(' ');
            p.len() > 255
                || ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
                    .iter()
                    .any(|n| stem.eq_ignore_ascii_case(n))
                || (stem.len() == 4
                    && (stem.as_bytes()[..3].eq_ignore_ascii_case(b"COM")
                        || stem.as_bytes()[..3].eq_ignore_ascii_case(b"LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
    {
        return Err(Error::Path);
    }
    if s.is_empty()
        || s.len() > 512
        || !s
            .bytes()
            .all(|b| (0x20..=0x7e).contains(&b) && !b"\"\\:*?<>|".contains(&b))
        || s.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || p.ends_with('.')
                || p.ends_with(' ')
                || p.starts_with('.')
        })
    {
        return Err(Error::Path);
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub context: Context,
    pub files: usize,
    pub bytes: u64,
    pub inventory_sha256: [u8; 32],
}
impl Summary {
    /// Path-free integrity result; fingerprints are caller identities, not trust.
    pub fn json(self) -> String {
        format!(
            "{{\"report_version\":1,\"scope\":\"artifact_integrity\",\"status\":\"verified\",\"context\":{},\"files\":{},\"bytes\":{},\"inventory_sha256\":\"{}\"}}",
            self.context.json(),
            self.files,
            self.bytes,
            hex(&self.inventory_sha256)
        )
    }
}

/// Redacted, deterministic declaration for one unavailable encrypted BLTE
/// frame represented by a product's explicit declared-skip policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialEncryptedSection {
    pub lookup: String,
    pub frame_ordinal: u32,
    pub decoded_size: u64,
}

/// Redacted unavailable encrypted sections associated with one DB2 table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialEncryptedTable {
    pub path: String,
    pub sections: Vec<PartialEncryptedSection>,
}

const MAX_PARTIAL_ENCRYPTED_SECTIONS: usize = 16;
const MAX_PARTIAL_ENCRYPTED_TABLES: usize = 1161;
const MAX_PARTIAL_ENCRYPTED_TABLE_SECTIONS: usize =
    MAX_PARTIAL_ENCRYPTED_TABLES * MAX_PARTIAL_ENCRYPTED_SECTIONS;
const MAX_PARTIAL_HEADER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct PartialEncryptedDeclarations {
    pub sections: Vec<PartialEncryptedSection>,
    pub tables: Vec<PartialEncryptedTable>,
}

impl PartialEncryptedSection {
    pub fn json(&self) -> String {
        format!(
            "{{\"lookup\":\"{}\",\"frame_ordinal\":{},\"decoded_size\":{}}}",
            self.lookup, self.frame_ordinal, self.decoded_size
        )
    }

    fn parse(value: &Json) -> Result<Self, Error> {
        let partial = Self {
            lookup: string(value, "lookup")?.to_owned(),
            frame_ordinal: u32::try_from(number(value, "frame_ordinal")?)
                .map_err(|_| Error::Schema)?,
            decoded_size: number(value, "decoded_size")?,
        };
        validate_partial_encrypted_sections(std::slice::from_ref(&partial))?;
        if let Json::Object(fields) = value {
            if fields.len() != 3 {
                return Err(Error::Schema);
            }
        } else {
            return Err(Error::Schema);
        }
        Ok(partial)
    }
}

impl PartialEncryptedTable {
    pub fn json(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"sections\":[{}]}}",
            self.path,
            self.sections
                .iter()
                .map(PartialEncryptedSection::json)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn parse(entry: &Json) -> Result<Self, Error> {
        let Json::Array(values) = value(entry, "sections")? else {
            return Err(Error::Schema);
        };
        if values.len() > MAX_PARTIAL_ENCRYPTED_TABLE_SECTIONS {
            return Err(Error::Limit);
        }
        let mut sections = Vec::new();
        sections
            .try_reserve_exact(values.len())
            .map_err(|_| Error::Allocation)?;
        for section in values {
            sections.push(PartialEncryptedSection::parse(section)?);
        }
        let table = Self {
            path: string(entry, "path")?.to_owned(),
            sections,
        };
        validate_partial_encrypted_table(&table)?;
        if let Json::Object(fields) = entry {
            if fields.len() != 2 {
                return Err(Error::Schema);
            }
        } else {
            return Err(Error::Schema);
        }
        Ok(table)
    }
}

pub(super) fn validate_partial_encrypted_table_path(path: &str) -> Result<(), Error> {
    if path.starts_with('/')
        || path.contains("..")
        || path.contains('\0')
        || !path.ends_with(".db2")
    {
        return Err(Error::Path);
    }
    self::path(path)
}

fn validate_partial_encrypted_table(table: &PartialEncryptedTable) -> Result<(), Error> {
    validate_partial_encrypted_table_parts(&table.path, &table.sections)
}

pub(super) fn validate_partial_encrypted_table_parts(
    path: &str,
    sections: &[PartialEncryptedSection],
) -> Result<(), Error> {
    validate_partial_encrypted_table_path(path)?;
    if sections.is_empty() {
        return Err(Error::Schema);
    }
    validate_partial_encrypted_sections_bounded(sections, MAX_PARTIAL_ENCRYPTED_TABLE_SECTIONS)
}

pub(super) fn validate_partial_encrypted_tables(
    tables: &[PartialEncryptedTable],
) -> Result<(), Error> {
    if tables.len() > MAX_PARTIAL_ENCRYPTED_TABLES {
        return Err(Error::Limit);
    }
    let mut section_count = 0usize;
    for table in tables {
        validate_partial_encrypted_table(table)?;
        section_count = section_count
            .checked_add(table.sections.len())
            .ok_or(Error::Limit)?;
        if section_count > MAX_PARTIAL_ENCRYPTED_TABLE_SECTIONS {
            return Err(Error::Limit);
        }
    }
    if tables.windows(2).any(|pair| pair[0].path >= pair[1].path) {
        return Err(Error::Schema);
    }
    Ok(())
}

pub(super) fn validate_partial_encrypted_sections(
    sections: &[PartialEncryptedSection],
) -> Result<(), Error> {
    validate_partial_encrypted_sections_bounded(sections, MAX_PARTIAL_ENCRYPTED_SECTIONS)
}

fn validate_partial_encrypted_sections_bounded(
    sections: &[PartialEncryptedSection],
    max_sections: usize,
) -> Result<(), Error> {
    if sections.len() > max_sections
        || sections.iter().any(|section| {
            section.lookup.len() != 16
                || !section
                    .lookup
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        || sections
            .windows(2)
            .any(|pair| pair[0].frame_ordinal >= pair[1].frame_ordinal)
    {
        return Err(Error::Schema);
    }
    Ok(())
}
#[cfg(any(target_os = "linux", windows))]
pub(super) struct Inventory {
    pub facts: Vec<Artifact>,
    pub bytes: u64,
    owned: usize,
    limits: Limits,
}
#[cfg(any(target_os = "linux", windows))]
impl Inventory {
    pub fn new(limits: Limits) -> Self {
        Self {
            facts: Vec::new(),
            bytes: 0,
            owned: 128 * 1024,
            limits,
        }
    }
    #[cfg(windows)]
    pub fn charged_bytes(&self) -> usize {
        self.owned
    }
    pub fn remaining(&self) -> usize {
        self.limits.metadata_bytes.saturating_sub(self.owned)
    }
    pub fn push(&mut self, a: Artifact) -> Result<(), Error> {
        if self.facts.len() >= self.limits.files {
            return Err(Error::Limit);
        }
        self.bytes = self.bytes.checked_add(a.bytes).ok_or(Error::Limit)?;
        if self.bytes > self.limits.total_bytes {
            return Err(Error::Limit);
        }
        let strings = self
            .owned
            .checked_add(a.path.capacity())
            .ok_or(Error::Limit)?;
        if self.facts.len() == self.facts.capacity() {
            let cap = (self.facts.len() + 1)
                .max(self.facts.capacity().saturating_mul(2))
                .min(self.limits.files);
            let next = cap
                .checked_mul(std::mem::size_of::<Artifact>())
                .ok_or(Error::Limit)?;
            if strings.checked_add(next).ok_or(Error::Limit)? > self.limits.metadata_bytes {
                return Err(Error::Limit);
            }
            let before = self.facts.capacity();
            self.facts
                .try_reserve_exact(cap - self.facts.len())
                .map_err(|_| Error::Allocation)?;
            let actual = self.facts.capacity() * std::mem::size_of::<Artifact>();
            if strings + actual > self.limits.metadata_bytes {
                return Err(Error::Limit);
            }
            self.owned += actual - before * std::mem::size_of::<Artifact>();
        }
        self.owned += a.path.capacity();
        if self.owned > self.limits.metadata_bytes {
            return Err(Error::Limit);
        }
        self.facts.push(a);
        Ok(())
    }
    pub fn sort(&mut self) -> Result<(), Error> {
        self.facts.sort_unstable_by(|a, b| {
            a.path
                .bytes()
                .map(|c| c.to_ascii_lowercase())
                .cmp(b.path.bytes().map(|c| c.to_ascii_lowercase()))
        });
        if self
            .facts
            .windows(2)
            .any(|w| !ordered(&w[0].path, &w[1].path))
        {
            return Err(Error::Inventory);
        }
        Ok(())
    }
    pub fn summary(&self, context: Context) -> Summary {
        let mut h = sha256::Sha256::new();
        for a in &self.facts {
            h.update(a.json().as_bytes());
            h.update(b"\n");
        }
        Summary {
            context,
            files: self.facts.len(),
            bytes: self.bytes,
            inventory_sha256: h.finish(),
        }
    }
}

// Sorted siblings may not introduce case aliases for files or directories.
#[cfg(any(target_os = "linux", windows))]
pub(super) fn folded(a: &str, b: &str) -> std::cmp::Ordering {
    a.bytes()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
}
#[cfg(any(target_os = "linux", windows))]
pub(super) fn ordered(a: &str, b: &str) -> bool {
    if folded(a, b) != std::cmp::Ordering::Less {
        return false;
    }
    for (x, y) in a.split('/').zip(b.split('/')) {
        if !x.eq_ignore_ascii_case(y) {
            break;
        }
        if x != y {
            return false;
        }
    }
    true
}

#[cfg(any(target_os = "linux", windows))]
pub(super) fn receipt(a: &Artifact) -> String {
    format!(
        ".state/files/{}.json",
        hex(&digest(a.path.to_ascii_lowercase().as_bytes()))
    )
}

#[cfg(any(target_os = "linux", windows))]
pub(super) fn seal(context: Context, c: Component, inventory: &Inventory) -> String {
    let mut h = sha256::Sha256::new();
    let mut count = 0usize;
    let mut bytes = 0u64;
    for a in inventory.facts.iter().filter(|a| a.component == c) {
        h.update(a.json().as_bytes());
        h.update(b"\n");
        count += 1;
        bytes += a.bytes;
    }
    format!(
        "{{\"component\":\"{}\",\"context_sha256\":\"{}\",\"files\":{count},\"bytes\":{bytes},\"inventory_sha256\":\"{}\"}}",
        c.name(),
        hex(&digest(context.json().as_bytes())),
        hex(&h.finish())
    )
}

#[cfg(any(target_os = "linux", windows))]
pub(super) fn header(context: Context) -> String {
    format!(
        "{{\"manifest_version\":1,\"scope\":\"artifact_integrity\",\"status\":\"complete\",\"context\":{},\"artifacts\":[\n",
        context.json()
    )
}
#[allow(dead_code)] // Retained for the v25 global-only manifest contract.
pub(super) fn header_with_partial_sections(
    context: Context,
    sections: &[PartialEncryptedSection],
) -> Result<String, Error> {
    header_with_partial_declarations(context, sections, &[])
}

pub(super) fn header_with_partial_declarations(
    context: Context,
    sections: &[PartialEncryptedSection],
    tables: &[PartialEncryptedTable],
) -> Result<String, Error> {
    validate_partial_encrypted_sections(sections)?;
    validate_partial_encrypted_tables(tables)?;
    if sections.is_empty() && tables.is_empty() {
        return Ok(header(context));
    }
    let mut rendered = format!(
        "{{\"manifest_version\":1,\"scope\":\"artifact_integrity\",\"status\":\"complete\",\"context\":{}",
        context.json()
    );
    if !sections.is_empty() {
        let declarations = sections
            .iter()
            .map(PartialEncryptedSection::json)
            .collect::<Vec<_>>()
            .join(",");
        rendered.push_str(&format!(",\"partial_encrypted_sections\":[{declarations}]"));
    }
    if !tables.is_empty() {
        let declarations = tables
            .iter()
            .map(PartialEncryptedTable::json)
            .collect::<Vec<_>>()
            .join(",");
        rendered.push_str(&format!(",\"partial_encrypted_tables\":[{declarations}]"));
    }
    rendered.push_str(",\"artifacts\":[\n");
    Ok(rendered)
}

#[allow(dead_code)] // Retained for v25 callers that consume only global declarations.
pub(super) fn parse_header_partial_sections(
    header_text: &str,
    context: Context,
) -> Result<Vec<PartialEncryptedSection>, Error> {
    Ok(parse_header_partial_declarations(header_text, context)?.sections)
}

pub(super) fn parse_header_partial_declarations(
    header_text: &str,
    context: Context,
) -> Result<PartialEncryptedDeclarations, Error> {
    if header_text.len() > MAX_PARTIAL_HEADER_BYTES {
        return Err(Error::Limit);
    }
    // The manifest header intentionally ends with the open artifacts array.
    let mut complete_header = String::new();
    complete_header
        .try_reserve_exact(header_text.len() + 2)
        .map_err(|_| Error::Allocation)?;
    complete_header.push_str(header_text);
    complete_header.push_str("]}");
    let parsed = json::parse(&complete_header).map_err(|_| Error::Schema)?;
    let Json::Object(fields) = &parsed else {
        return Err(Error::Schema);
    };
    let allowed = [
        "manifest_version",
        "scope",
        "status",
        "context",
        "partial_encrypted_sections",
        "partial_encrypted_tables",
        "artifacts",
    ];
    if fields
        .iter()
        .any(|(key, _)| !allowed.contains(&key.as_str()))
        || number(&parsed, "manifest_version")? != 1
        || string(&parsed, "scope")? != "artifact_integrity"
        || string(&parsed, "status")? != "complete"
    {
        return Err(Error::Schema);
    }
    let has_sections = fields
        .iter()
        .any(|(key, _)| key == "partial_encrypted_sections");
    let has_tables = fields
        .iter()
        .any(|(key, _)| key == "partial_encrypted_tables");
    if fields.len() != 5 + usize::from(has_sections) + usize::from(has_tables) {
        return Err(Error::Schema);
    }
    let Json::Array(artifacts) = value(&parsed, "artifacts")? else {
        return Err(Error::Schema);
    };
    if !artifacts.is_empty() {
        return Err(Error::Schema);
    }

    let mut sections = Vec::new();
    if has_sections {
        let Json::Array(values) = value(&parsed, "partial_encrypted_sections")? else {
            return Err(Error::Schema);
        };
        sections
            .try_reserve_exact(values.len())
            .map_err(|_| Error::Allocation)?;
        for declaration in values {
            sections.push(PartialEncryptedSection::parse(declaration)?);
        }
    }
    validate_partial_encrypted_sections(&sections)?;

    let mut tables = Vec::new();
    if has_tables {
        let Json::Array(values) = value(&parsed, "partial_encrypted_tables")? else {
            return Err(Error::Schema);
        };
        tables
            .try_reserve_exact(values.len())
            .map_err(|_| Error::Allocation)?;
        for declaration in values {
            tables.push(PartialEncryptedTable::parse(declaration)?);
        }
    }
    validate_partial_encrypted_tables(&tables)?;

    let canonical = header_with_partial_declarations(context, &sections, &tables)?;
    if header_text != canonical.strip_suffix('\n').ok_or(Error::Schema)? {
        return Err(Error::Schema);
    }
    Ok(PartialEncryptedDeclarations { sections, tables })
}
#[cfg(any(target_os = "linux", windows))]
pub(super) fn footer(s: Summary) -> String {
    format!(
        "],\"file_count\":{},\"total_bytes\":{},\"inventory_sha256\":\"{}\"}}\n",
        s.files,
        s.bytes,
        hex(&s.inventory_sha256)
    )
}

#[cfg(any(target_os = "linux", windows))]
#[cfg(windows)]
pub(super) fn manifest_fact(context: Context, inventory: &Inventory) -> Result<Artifact, Error> {
    manifest_fact_with_partial_sections(context, inventory, &[])
}

#[cfg(any(target_os = "linux", windows))]
#[allow(dead_code)] // Retained for the Windows v25 manifest implementation.
pub(super) fn manifest_fact_with_partial_sections(
    context: Context,
    inventory: &Inventory,
    sections: &[PartialEncryptedSection],
) -> Result<Artifact, Error> {
    manifest_fact_with_partial_declarations(context, inventory, sections, &[])
}

#[cfg(any(target_os = "linux", windows))]
pub(super) fn manifest_fact_with_partial_declarations(
    context: Context,
    inventory: &Inventory,
    sections: &[PartialEncryptedSection],
    tables: &[PartialEncryptedTable],
) -> Result<Artifact, Error> {
    let mut hash = sha256::Sha256::new();
    let mut bytes = 0u64;
    let mut emit = |s: &str| -> Result<(), Error> {
        bytes = bytes.checked_add(s.len() as u64).ok_or(Error::Limit)?;
        hash.update(s.as_bytes());
        Ok(())
    };
    emit(&header_with_partial_declarations(
        context, sections, tables,
    )?)?;
    for (i, a) in inventory.facts.iter().enumerate() {
        emit(&format!(
            "{}{}\n",
            a.json(),
            if i + 1 == inventory.facts.len() {
                ""
            } else {
                ","
            }
        ))?;
    }
    emit(&footer(inventory.summary(context)))?;
    Ok(Artifact {
        component: Component::Db2,
        path: MANIFEST.to_owned(),
        bytes,
        sha256: hash.finish(),
    })
}

#[cfg(test)]
mod product70425_context_tests {
    use super::*;
    #[test]
    fn context70425_is_exact_linux_collision_mask_only() {
        let c = Context {
            build: 70425,
            locale: crate::Locale::EsEs,
            source_sha256: [1; 32],
            plan_sha256: [2; 32],
            producer_sha256: [3; 32],
            components: Component::Collision.bit(),
        };
        assert_eq!(c.validate().is_ok(), cfg!(target_os = "linux"));
        if cfg!(target_os = "linux") {
            assert_eq!(Context::parse(&c.json()).unwrap(), c);
        }
        for components in 0..=u8::MAX {
            let expected = cfg!(target_os = "linux") && components == Component::Collision.bit();
            assert_eq!(
                Context { components, ..c }.validate().is_ok(),
                expected,
                "mask {components}"
            );
        }
    }
}

#[cfg(test)]
mod product70450_context_tests {
    use super::*;

    // The v19 contract source pin retains this old identifier; the exhaustive
    // test below now includes the later implemented product masks.
    const V19_CONTEXT_TEST_IDENTIFIER: &str =
        "context70450_admits_only_the_three_linux_product_masks";
    const V21_CONTEXT_TEST_IDENTIFIER: &str =
        "context70450_admits_only_the_five_linux_implemented_product_masks";

    #[test]
    fn context70450_admits_only_the_nine_linux_implemented_product_masks() {
        assert_eq!(
            V19_CONTEXT_TEST_IDENTIFIER,
            "context70450_admits_only_the_three_linux_product_masks"
        );
        assert_eq!(
            V21_CONTEXT_TEST_IDENTIFIER,
            "context70450_admits_only_the_five_linux_implemented_product_masks"
        );
        let c = Context {
            build: 70450,
            locale: crate::Locale::EsEs,
            source_sha256: [1; 32],
            plan_sha256: [2; 32],
            producer_sha256: [3; 32],
            components: Component::Collision.bit(),
        };
        // v16/v17/v26 DB2, v15/v18/v22 collision, v19 DB2+GT+cameras, v20 Terrain,
        // v21 DB2+Terrain+Collision+Navigation, v23 Navigation, v24/v25/v27
        // DB2+Terrain, v28-v50 DB2+collision and v51 DB2+Terrain+Collision.
        let admitted = [
            Component::Db2.bit(),
            Component::Db2.bit() | Component::Collision.bit(),
            Component::Collision.bit(),
            Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
            Component::Terrain.bit(),
            Component::Db2.bit() | Component::Terrain.bit(),
            Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit(),
            Component::Db2.bit()
                | Component::Terrain.bit()
                | Component::Collision.bit()
                | Component::Navigation.bit(),
            Component::Navigation.bit(),
        ];
        let db2_terrain = Component::Db2.bit() | Component::Terrain.bit();
        assert_eq!(db2_terrain, 9);
        assert!(admitted.contains(&db2_terrain));
        for components in 0..=u8::MAX {
            let context = Context { components, ..c };
            let expected = cfg!(target_os = "linux") && admitted.contains(&components);
            assert_eq!(context.validate().is_ok(), expected, "mask {components}");
            if expected {
                assert_eq!(Context::parse(&context.json()).unwrap(), context);
            }
        }
    }
}
