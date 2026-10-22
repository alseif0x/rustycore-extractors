// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit same-context state v2 recovery, no v1 migration.
use super::*;
pub(super) const PROTOCOL: &str = ".state/protocol.json";
pub(super) const TRANSACTION: &str = ".state/transaction.json";
pub(super) fn protocol_json(context: Context) -> String {
    format!(
        "{{\"state_version\":2,\"context_sha256\":\"{}\"}}",
        hex(&digest(context.json().as_bytes()))
    )
}
pub(super) fn protocol(root: &Root, context: Context, limits: Limits) -> Result<(), Error> {
    let bytes = root.small(PROTOCOL, limits)?;
    let s = std::str::from_utf8(&bytes).map_err(|_| Error::Schema)?;
    let v = parse(s)?;
    if number(&v, "state_version")? != 2 {
        return Err(Error::StateVersion);
    }
    if string(&v, "context_sha256")? != hex(&digest(context.json().as_bytes())) {
        return Err(Error::Context);
    }
    if bytes != protocol_json(context).as_bytes() {
        return Err(Error::Schema);
    }
    Ok(())
}
fn read_partial_sections_from_manifest(
    root: &Root,
    path: &str,
    context: Context,
    limits: Limits,
) -> Result<Vec<PartialEncryptedSection>, Error> {
    let mut file = root.open_file(path, limits)?;
    let before = file.metadata().map_err(|_| Error::Io)?;
    if !before.is_file() || before.len() > limits.manifest_bytes {
        return Err(Error::Path);
    }
    let mut reader = io::BufReader::new(&mut file);
    let line = manifest_header_line(&mut reader)?;
    let text = std::str::from_utf8(line.strip_suffix(b"\n").ok_or(Error::Schema)?)
        .map_err(|_| Error::Schema)?;
    let sections = parse_header_partial_sections(text, context)?;
    if Identity::of(&before) != Identity::of(&file.metadata().map_err(|_| Error::Changed)?) {
        return Err(Error::Changed);
    }
    root.named_file(path, &file, limits)?;
    Ok(sections)
}
pub(super) fn transaction(context: Context, a: &Artifact) -> String {
    format!(
        "{{\"transaction_version\":2,\"context_sha256\":\"{}\",\"artifact\":{}}}",
        hex(&digest(context.json().as_bytes())),
        a.json()
    )
}
fn parse_transaction(bytes: &[u8], context: Context, limits: Limits) -> Result<Artifact, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::Schema)?;
    let v = parse(s)?;
    let a = value(&v, "artifact")?;
    let a = Artifact {
        component: Component::parse(string(a, "component")?)?,
        path: string(a, "path")?.to_owned(),
        bytes: number(a, "bytes")?,
        sha256: unhex(string(a, "sha256")?)?,
    };
    a.validate(context, limits)?;
    if s != transaction(context, &a) {
        return Err(Error::Context);
    }
    Ok(a)
}
pub(super) fn output(output: &Path, protected: &[&Path]) -> Result<PathBuf, Error> {
    let parent = output.parent().ok_or(Error::Path)?;
    safe(parent)?;
    if output
        .components()
        .any(|p| matches!(p, Part::CurDir | Part::ParentDir))
        || output
            .as_os_str()
            .as_encoded_bytes()
            .split(|&b| b == b'/')
            .any(|s| s == b"." || s == b"..")
    {
        return Err(Error::Path);
    }
    let parent = fs::canonicalize(parent).map_err(|_| Error::Path)?;
    if parent.ancestors().any(|p| p.join(".git").exists()) {
        return Err(Error::Path);
    }
    let output = parent.join(output.file_name().ok_or(Error::Path)?);
    for p in protected {
        let source = fs::canonicalize(p).map_err(|_| Error::Path)?;
        if output.starts_with(&source) || source.starts_with(&output) {
            return Err(Error::Path);
        }
    }
    Ok(output)
}
fn meta(root: &Root, name: &str) -> Result<Option<Metadata>, Error> {
    root.guard()?;
    let mut fd = root.held.try_clone().map_err(|_| Error::Io)?;
    let mut parts = name.split('/').peekable();
    let mut depth = 0;
    while let Some(part) = parts.next() {
        if part.is_empty() || part == "." || part == ".." {
            return Err(Error::Path);
        }
        let path = anchor(&fd).join(part);
        if parts.peek().is_none() {
            return match fs::symlink_metadata(path) {
                Ok(m) => Ok(Some(m)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(_) => Err(Error::Io),
            };
        }
        depth += 1;
        if depth > 16 {
            return Err(Error::Limit);
        }
        match OpenOptions::new()
            .read(true)
            .custom_flags(NOFOLLOW | DIRECTORY | NONBLOCK)
            .open(path)
        {
            Ok(f) => fd = f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Path),
        }
    }
    Err(Error::Path)
}
fn pending(name: &str) -> Result<String, Error> {
    let (parent, leaf) = name.rsplit_once('/').unwrap_or(("", name));
    if leaf.is_empty() {
        return Err(Error::Path);
    }
    Ok(if parent.is_empty() {
        format!(".{leaf}.part")
    } else {
        format!("{parent}/.{leaf}.part")
    })
}
pub(super) fn remove(root: &Root, name: &str, limits: Limits) -> Result<(), Error> {
    let (parent, leaf) = root.parent(name, false, &mut 0, limits)?;
    let path = anchor(&parent).join(leaf);
    let m = fs::symlink_metadata(&path).map_err(|_| Error::Changed)?;
    if !m.is_file() || m.nlink() != 1 {
        return Err(Error::Path);
    }
    fs::remove_file(path).map_err(|_| Error::Io)?;
    sync(&parent)?;
    root.guard()
}
struct Plan {
    extra: Vec<(String, Identity)>,
    cleanup: Vec<(String, Identity)>,
}
impl Plan {
    fn new() -> Self {
        Self {
            extra: Vec::new(),
            cleanup: Vec::new(),
        }
    }
    fn special(&mut self, name: &str, m: &Metadata, cleanup: bool) -> Result<(), Error> {
        if self.extra.len() >= 24 {
            return Err(Error::Limit);
        }
        self.extra.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.extra.push((name.to_owned(), Identity::of(m)));
        if cleanup {
            self.cleanup.try_reserve(1).map_err(|_| Error::Allocation)?;
            self.cleanup.push((name.to_owned(), Identity::of(m)));
        }
        Ok(())
    }
    // A two-link file is legal only as the exact pending/final inode pair from
    // exclusive hard-link publication. Unrelated links and conflicting pairs fail.
    fn pair(
        &mut self,
        root: &Root,
        name: &str,
        temporary: &str,
        cap: u64,
    ) -> Result<Option<Metadata>, Error> {
        let final_meta = meta(root, name)?;
        let temp_meta = meta(root, temporary)?;
        for m in final_meta.iter().chain(temp_meta.iter()) {
            if !m.is_file() || m.file_type().is_symlink() {
                return Err(Error::Path);
            }
        }
        match (&final_meta, &temp_meta) {
            (Some(a), Some(b)) => {
                if a.nlink() != 2 || Identity::of(a) != Identity::of(b) {
                    return Err(Error::Path);
                }
                self.special(name, a, false)?;
                self.special(temporary, b, true)?;
            }
            (Some(a), None) => {
                if a.nlink() != 1 {
                    return Err(Error::Path);
                }
            }
            (None, Some(b)) => {
                if b.nlink() != 1 || b.len() > cap {
                    return Err(Error::Path);
                }
                self.special(temporary, b, true)?;
            }
            _ => {}
        }
        Ok(final_meta)
    }
    fn apply(&self, root: &Root, limits: Limits) -> Result<(), Error> {
        for (name, id) in &self.cleanup {
            let (parent, leaf) = root.parent(name, false, &mut 0, limits)?;
            let path = anchor(&parent).join(leaf);
            if Identity::of(&fs::symlink_metadata(&path).map_err(|_| Error::Changed)?) != *id {
                return Err(Error::Changed);
            }
            fs::remove_file(path).map_err(|_| Error::Io)?;
            sync(&parent)?;
            root.guard()?;
        }
        Ok(())
    }
}
fn receipts(
    root: &Root,
    context: Context,
    limits: Limits,
    plan: &Plan,
) -> Result<Inventory, Error> {
    let fd = dir(&anchor(&root.held).join(".state/files"))?;
    let before = fd.metadata().map_err(|_| Error::Io)?;
    let mut inventory = Inventory::new(limits);
    for e in fs::read_dir(anchor(&fd)).map_err(|_| Error::Io)? {
        let leaf = e
            .map_err(|_| Error::Io)?
            .file_name()
            .into_string()
            .map_err(|_| Error::Path)?;
        let name = format!(".state/files/{leaf}");
        if leaf.starts_with('.') && plan.extra.iter().any(|(n, _)| *n == name) {
            continue;
        }
        if leaf.len() != 69 || !leaf.ends_with(".json") {
            return Err(Error::Inventory);
        }
        let links = if plan.extra.iter().any(|(n, _)| *n == name) {
            2
        } else {
            1
        };
        let b = root.small_links(&name, limits, links)?;
        let a = Artifact::parse(
            std::str::from_utf8(&b).map_err(|_| Error::Schema)?,
            context,
            limits,
        )?;
        if receipt(&a) != name {
            return Err(Error::Inventory);
        }
        inventory.push(a)?;
    }
    if Identity::of(&before) != Identity::of(&fd.metadata().map_err(|_| Error::Changed)?) {
        return Err(Error::Changed);
    }
    inventory.sort()?;
    Ok(inventory)
}
struct ReplayEntry {
    hash: [u8; 32],
    component: Component,
    seen: bool,
}
/// Bounded digest index, charged separately from subsequent fact inventories.
/// All retained unsealed facts (including an outstanding intent) must be emitted
/// by the restarted producer. Repeated callbacks cannot satisfy another file.
pub(super) struct Replay {
    entries: Vec<ReplayEntry>,
    remaining: usize,
}
impl Replay {
    fn new(inventory: &Inventory, sealed: u8) -> Result<Option<Self>, Error> {
        let count = inventory
            .facts
            .iter()
            .filter(|a| sealed & a.component.bit() == 0)
            .count();
        if count == 0 {
            return Ok(None);
        }
        let bytes = count
            .checked_mul(std::mem::size_of::<ReplayEntry>())
            .ok_or(Error::Limit)?;
        if bytes > inventory.remaining() {
            return Err(Error::Limit);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if entries
            .capacity()
            .checked_mul(std::mem::size_of::<ReplayEntry>())
            .ok_or(Error::Limit)?
            > inventory.remaining()
        {
            return Err(Error::Limit);
        }
        for a in inventory
            .facts
            .iter()
            .filter(|a| sealed & a.component.bit() == 0)
        {
            entries.push(ReplayEntry {
                hash: digest(a.path.to_ascii_lowercase().as_bytes()),
                component: a.component,
                seen: false,
            });
        }
        entries.sort_unstable_by_key(|a| a.hash);
        if entries.windows(2).any(|a| a[0].hash == a[1].hash) {
            return Err(Error::Inventory);
        }
        Ok(Some(Self {
            entries,
            remaining: count,
        }))
    }
    pub(super) fn bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<ReplayEntry>()
    }
    pub(super) fn acknowledge(&mut self, path: &str) {
        let hash = digest(path.to_ascii_lowercase().as_bytes());
        if let Ok(i) = self.entries.binary_search_by_key(&hash, |a| a.hash)
            && !self.entries[i].seen
        {
            self.entries[i].seen = true;
            self.remaining -= 1;
        }
    }
    pub(super) fn complete(&self) -> bool {
        self.remaining == 0
    }
    pub(super) fn pending(&self, c: Component) -> bool {
        self.entries.iter().any(|a| a.component == c && !a.seen)
    }
}
pub(super) fn resume(
    output_path: &Path,
    expected: Context,
    protected: &[&Path],
    limits: Limits,
) -> Result<Run, Error> {
    expected.validate()?;
    limits.validate()?;
    let output = output(output_path, protected)?;
    let root = Root::open(&output)?;
    let lock = root.lock(true, limits)?;
    if !root.has(INCOMPLETE, limits)? {
        return Err(Error::Sealed);
    }
    if root.small(INCOMPLETE, limits)? != b"incomplete\n" {
        return Err(Error::Incomplete);
    }
    if context(&root, limits)? != expected {
        return Err(Error::Context);
    }
    if !root.has(PROTOCOL, limits)? {
        return Err(Error::StateVersion);
    }
    protocol(&root, expected, limits)?;
    root.named_file(".state/lock", &lock, limits)?;
    let mut plan = Plan::new();
    let tm = plan.pair(&root, TRANSACTION, &pending(TRANSACTION)?, LINE as u64)?;
    let tx = tm
        .as_ref()
        .map(|m| {
            root.small_links(TRANSACTION, limits, m.nlink())
                .and_then(|b| parse_transaction(&b, expected, limits))
        })
        .transpose()?;
    let (payload, receipt_meta) = if let Some(a) = &tx {
        let payload = plan.pair(&root, &a.path, &pending(&a.path)?, a.bytes)?;
        let name = receipt(a);
        let receipt_meta = plan.pair(&root, &name, &pending(&name)?, a.json().len() as u64)?;
        if let Some(m) = &payload {
            root.hash_links(a, limits, m.nlink())?;
        }
        if let Some(m) = &receipt_meta
            && (payload.is_none()
                || root.small_links(&name, limits, m.nlink())? != a.json().as_bytes())
        {
            return Err(Error::Inventory);
        }
        if payload.is_none() && meta(&root, &pending(&name)?)?.is_some() {
            return Err(Error::Inventory);
        }
        (payload, receipt_meta)
    } else {
        (None, None)
    };
    let mut inventory = receipts(&root, expected, limits, &plan)?;
    let receipt_count = inventory.facts.len();
    for a in &inventory.facts {
        if tx
            .as_ref()
            .is_some_and(|t| t.path.eq_ignore_ascii_case(&a.path) && t != a)
        {
            return Err(Error::Inventory);
        }
        let links = if tx.as_ref() == Some(a) {
            payload.as_ref().map_or(1, MetadataExt::nlink)
        } else {
            1
        };
        root.hash_links(a, limits, links)?;
    }
    if let Some(a) = &tx
        && receipt_meta.is_none()
    {
        inventory.push(a.clone())?;
        inventory.sort()?;
    }
    let mut mask = 0u8;
    for c in Component::ALL {
        let name = format!(".state/sealed/{}", c.name());
        let expected_seal = seal(expected, c, &inventory);
        let primary = plan.pair(&root, &name, &pending(&name)?, expected_seal.len() as u64)?;
        if let Some(m) = primary {
            if !expected.contains(c)
                || root.small_links(&name, limits, m.nlink())? != expected_seal.as_bytes()
            {
                return Err(Error::Inventory);
            }
            mask |= c.bit();
        }
        if !expected.contains(c) && meta(&root, &pending(&name)?)?.is_some() {
            return Err(Error::Component);
        }
    }
    if tx
        .as_ref()
        .is_some_and(|a| receipt_meta.is_none() && mask & a.component.bit() != 0)
    {
        return Err(Error::Inventory);
    }
    let partial_manifest = if root.has(MANIFEST, limits)? {
        Some(read_partial_sections_from_manifest(
            &root, MANIFEST, expected, limits,
        )?)
    } else if root.has(".state/package.part", limits)? {
        match read_partial_sections_from_manifest(&root, ".state/package.part", expected, limits) {
            Ok(sections) => Some(sections),
            Err(Error::Schema) => None,
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let partial_sections = partial_manifest.as_deref().unwrap_or_default();
    let manifest_expected =
        manifest_fact_with_partial_sections(expected, &inventory, partial_sections)?;
    if manifest_expected.bytes > limits.manifest_bytes {
        return Err(Error::Limit);
    }
    let final_manifest = plan.pair(
        &root,
        MANIFEST,
        ".state/package.part",
        manifest_expected.bytes,
    )?;
    if final_manifest.is_some() || meta(&root, ".state/package.part")?.is_some() {
        if mask != expected.components || tx.is_some() {
            return Err(Error::Incomplete);
        }
        if let Some(m) = &final_manifest {
            root.hash_links(
                &manifest_expected,
                Limits {
                    file_bytes: limits.manifest_bytes,
                    ..limits
                },
                m.nlink(),
            )?;
        }
    }
    // Journal itself is a known validated internal file, excluded from artifact counts.
    if let Some(m) = &tm
        && m.nlink() == 1
    {
        plan.special(TRANSACTION, m, false)?;
    }
    let payload_count =
        receipt_count + usize::from(tx.is_some() && receipt_meta.is_none() && payload.is_some());
    let dirs = scan(
        &root,
        &inventory,
        expected,
        limits,
        false,
        final_manifest.is_some(),
        &plan.extra,
        (receipt_count, payload_count),
    )?;
    if context(&root, limits)? != expected {
        return Err(Error::Context);
    }
    protocol(&root, expected, limits)?;
    root.named_file(".state/lock", &lock, limits)?;
    let replay = Replay::new(&inventory, mask)?;
    let inventory_limits = Limits {
        metadata_bytes: limits.metadata_bytes - replay.as_ref().map_or(0, Replay::bytes),
        ..limits
    };
    // No deletion/publication occurs until full preflight and namespace closure pass.
    drop(inventory);
    plan.apply(&root, limits)?;
    if let Some(a) = &tx {
        if root.small(TRANSACTION, limits)? != transaction(expected, a).as_bytes() {
            return Err(Error::Changed);
        }
        if payload.is_some() {
            root.hash(a, limits)?;
            if receipt_meta.is_none() {
                root.publish(&receipt(a), a.json().as_bytes(), &mut 0, limits)?;
            }
            remove(&root, TRANSACTION, limits)?;
        }
    }
    let inventory = collect(&root, expected, inventory_limits)?;
    let files = inventory.facts.len();
    let bytes = inventory.bytes;
    for a in &inventory.facts {
        root.hash(a, limits)?;
    }
    seals(&root, expected, &inventory, limits)?;
    if context(&root, limits)? != expected {
        return Err(Error::Context);
    }
    protocol(&root, expected, limits)?;
    root.named_file(".state/lock", &lock, limits)?;
    Ok(Run {
        root,
        _lock: lock,
        context: expected,
        limits,
        dirs,
        files,
        bytes,
        failed: false,
        replay,
        partial_encrypted_sections: partial_manifest,
    })
}

#[cfg(test)]
mod tests;
