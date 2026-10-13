// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: versioned resident Windows NTFS artifact package.
//! State v3 uses exclusive same-volume rename, never Linux's hardlink protocol.
//! Observed identity/membership checks are not an atomic filesystem snapshot or
//! a crash durability guarantee. Completion proves artifact integrity only.
use super::*;
use forever_platform::windows::{Identity, ReadFile, package as native};
use model::{Inventory, footer, header, manifest_fact, receipt, seal};
use native::{Entry, Kind, Lock, Root};
use std::io::{Seek, SeekFrom};
mod recover;
mod verify;
pub use verify::{verify, verify_with};
#[cfg(test)]
mod tests;

pub const FILESYSTEM_PROFILE: &str = "forever-windows-product-ntfs-v1";
const PROTOCOL: &str = ".state/protocol.json";
const TRANSACTION: &str = ".state/transaction.json";
const INTENT_PART: &str = ".state/intent.part";
const PAYLOAD_PART: &str = ".state/payload.part";
const RECEIPT_PART: &str = ".state/receipt.part";
const MANIFEST_PART: &str = ".state/package.part";
const LOCK: &str = ".forever-package-lock";
const BASE_DIRS: [&str; 3] = [".state", ".state/files", ".state/sealed"];

impl From<native::Error> for Error {
    fn from(error: native::Error) -> Self {
        match error {
            native::Error::Limit => Self::Limit,
            native::Error::Allocation => Self::Allocation,
            native::Error::Exists => Self::Exists,
            native::Error::Busy => Self::Busy,
            native::Error::Changed | native::Error::Identity => Self::Changed,
            native::Error::Unsupported => Self::Unsupported,
            native::Error::Path
            | native::Error::Reparse
            | native::Error::HardLinks
            | native::Error::Privacy
            | native::Error::Alias => Self::Path,
            native::Error::Membership => Self::Inventory,
            _ => Self::Io,
        }
    }
}
fn native_limits(limits: Limits) -> native::Limits {
    native::Limits {
        max_entries: limits
            .files
            .saturating_mul(2)
            .saturating_add(limits.directories)
            .saturating_add(32),
        max_directories: limits.directories,
        max_file_bytes: limits
            .file_bytes
            .max(limits.manifest_bytes)
            .max(LINE as u64),
        // Payloads retain the common 1 TiB gate; this also admits state overhead.
        max_total_bytes: limits.total_bytes.saturating_add(512 * 1024 * 1024),
        max_metadata_bytes: limits.metadata_bytes,
        ..native::Limits::default()
    }
}
fn protocol_json(context: Context) -> String {
    format!(
        "{{\"state_version\":3,\"filesystem_profile\":\"{FILESYSTEM_PROFILE}\",\"context_sha256\":\"{}\"}}",
        hex(&digest(context.json().as_bytes()))
    )
}
fn transaction(context: Context, a: &Artifact) -> String {
    format!(
        "{{\"transaction_version\":3,\"filesystem_profile\":\"{FILESYSTEM_PROFILE}\",\"context_sha256\":\"{}\",\"artifact\":{}}}",
        hex(&digest(context.json().as_bytes())),
        a.json()
    )
}
fn parse_transaction(bytes: &[u8], context: Context, limits: Limits) -> Result<Artifact, Error> {
    let s = std::str::from_utf8(bytes).map_err(|_| Error::Schema)?;
    let v = parse(s)?;
    if number(&v, "transaction_version")? != 3
        || string(&v, "filesystem_profile")? != FILESYSTEM_PROFILE
    {
        return Err(Error::StateVersion);
    }
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
fn context(root: &Root) -> Result<Context, Error> {
    Context::parse(
        std::str::from_utf8(&small(root, ".state/context.json", None)?)
            .map_err(|_| Error::Schema)?,
    )
}
fn state(root: &Root, expected: Context) -> Result<(), Error> {
    root.guard()?;
    if context(root)? != expected {
        return Err(Error::Context);
    }
    let bytes = small(root, PROTOCOL, None)?;
    let s = std::str::from_utf8(&bytes).map_err(|_| Error::Schema)?;
    let v = parse(s)?;
    if number(&v, "state_version")? != 3 || string(&v, "filesystem_profile")? != FILESYSTEM_PROFILE
    {
        return Err(Error::StateVersion);
    }
    if s != protocol_json(expected) {
        return Err(Error::Context);
    }
    Ok(())
}
fn small(root: &Root, path: &str, id: Option<Identity>) -> Result<Vec<u8>, Error> {
    root.with_read_file(path, id, |r| {
        let mut bytes = Vec::new();
        Read::by_ref(r)
            .take((LINE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Io)?;
        if bytes.len() > LINE {
            return Err(Error::Limit);
        }
        Ok(bytes)
    })
    .map(|(_, b)| b)
}
fn hash(root: &Root, a: &Artifact, id: Option<Identity>) -> Result<(), Error> {
    let (actual, ()) = root.with_read_file(&a.path, id, |r| {
        let hash = crate::integrity::reader(r, a.bytes).map_err(|_| Error::Digest)?;
        if hash != a.sha256 {
            return Err(Error::Digest);
        }
        Ok(())
    })?;
    if actual.size != a.bytes {
        return Err(Error::Digest);
    }
    Ok(())
}
fn publish(root: &mut Root, name: &str, temporary: &str, bytes: &[u8]) -> Result<(), Error> {
    publish_file(root, name, temporary, bytes).map(|_| ())
}
fn publish_file(
    root: &mut Root,
    name: &str,
    temporary: &str,
    bytes: &[u8],
) -> Result<Identity, Error> {
    if let Some((parent, _)) = name.rsplit_once('/') {
        root.create_dirs(parent)?;
    }
    let (id, ()) = root.with_new_file(temporary, |f| f.write_all(bytes).map_err(|_| Error::Io))?;
    Ok(root.rename_no_replace(temporary, name, id)?)
}

fn publish_manifest(
    root: &mut Root,
    context: Context,
    inventory: &Inventory,
    limits: Limits,
) -> Result<Identity, Error> {
    let fact = manifest_fact(context, inventory)?;
    if fact.bytes > limits.manifest_bytes {
        return Err(Error::Limit);
    }
    let (id, ()) = root.with_new_file(MANIFEST_PART, |f| {
        f.write_all(header(context).as_bytes())
            .map_err(|_| Error::Io)?;
        for (i, a) in inventory.facts.iter().enumerate() {
            writeln!(
                f,
                "{}{}",
                a.json(),
                if i + 1 == inventory.facts.len() {
                    ""
                } else {
                    ","
                }
            )
            .map_err(|_| Error::Io)?;
        }
        f.write_all(footer(inventory.summary(context)).as_bytes())
            .map_err(|_| Error::Io)
    })?;
    Ok(root.rename_no_replace(MANIFEST_PART, MANIFEST, id)?)
}
fn entry<'a>(entries: &'a [Entry], path: &str) -> Option<&'a Entry> {
    entries
        .binary_search_by(|e| model::folded(&e.path, path))
        .ok()
        .and_then(|i| entries.get(i))
        .filter(|e| e.path == path)
}
fn observe(root: &Root) -> Result<Vec<Entry>, Error> {
    observe_charged(root, 0)
}
fn observe_charged(root: &Root, retained: usize) -> Result<Vec<Entry>, Error> {
    let mut entries = root.observe_charged(retained)?;
    entries.sort_unstable_by(|a, b| model::folded(&a.path, &b.path));
    if entries
        .windows(2)
        .any(|w| !model::ordered(&w[0].path, &w[1].path))
    {
        return Err(Error::Inventory);
    }
    Ok(entries)
}
fn observed_bytes(entries: &Vec<Entry>) -> Result<usize, Error> {
    entries
        .capacity()
        .checked_mul(std::mem::size_of::<Entry>())
        .and_then(|n| {
            entries
                .iter()
                .try_fold(n, |n, e| n.checked_add(e.path.capacity()))
        })
        .ok_or(Error::Limit)
}
fn inventory_limits(
    root: &Root,
    limits: Limits,
    entries: &Vec<Entry>,
    replay: usize,
) -> Result<Limits, Error> {
    let charge = observed_bytes(entries)?
        .checked_add(root.retained_metadata_bytes()?)
        .ok_or(Error::Limit)?
        .checked_add(replay)
        .ok_or(Error::Limit)?;
    let metadata_bytes = limits
        .metadata_bytes
        .checked_sub(charge)
        .ok_or(Error::Limit)?;
    if metadata_bytes < 128 * 1024 {
        return Err(Error::Limit);
    }
    Ok(Limits {
        metadata_bytes,
        ..limits
    })
}
fn collect(
    root: &Root,
    entries: &[Entry],
    context: Context,
    limits: Limits,
) -> Result<Inventory, Error> {
    let mut inventory = Inventory::new(limits);
    for e in entries
        .iter()
        .filter(|e| e.path.starts_with(".state/files/"))
    {
        if e.kind != Kind::File {
            return Err(Error::Inventory);
        }
        let leaf = e
            .path
            .strip_prefix(".state/files/")
            .ok_or(Error::Inventory)?;
        if leaf.len() != 69 || !leaf.ends_with(".json") {
            return Err(Error::Inventory);
        }
        let b = small(root, &e.path, Some(e.identity))?;
        let a = Artifact::parse(
            std::str::from_utf8(&b).map_err(|_| Error::Schema)?,
            context,
            limits,
        )?;
        if receipt(&a) != e.path {
            return Err(Error::Inventory);
        }
        inventory.push(a)?;
    }
    inventory.sort()?;
    Ok(inventory)
}
fn seals(
    root: &Root,
    entries: &[Entry],
    context: Context,
    inventory: &Inventory,
) -> Result<u8, Error> {
    let mut mask = 0;
    for c in Component::ALL {
        let name = format!(".state/sealed/{}", c.name());
        if let Some(e) = entry(entries, &name) {
            if e.kind != Kind::File || !context.contains(c) {
                return Err(Error::Component);
            }
            if small(root, &name, Some(e.identity))? != seal(context, c, inventory).as_bytes() {
                return Err(Error::Inventory);
            }
            mask |= c.bit();
        }
    }
    Ok(mask)
}
fn directory_allowed(name: &str, context: Context, inventory: &Inventory) -> bool {
    if BASE_DIRS.contains(&name) {
        return true;
    }
    if Component::ALL
        .into_iter()
        .any(|c| context.contains(c) && name == if c == Component::Db2 { "dbc" } else { c.name() })
    {
        return true;
    }
    let prefix = format!("{name}/");
    let i = inventory
        .facts
        .partition_point(|a| model::folded(&a.path, &prefix) == std::cmp::Ordering::Less);
    inventory
        .facts
        .get(i)
        .is_some_and(|a| a.path.starts_with(&prefix))
}
// Complete namespace closure. `special` is a bounded, fully classified recovery
// set; ordinary receipts/payloads must exactly match the canonical inventory.
fn closed(
    entries: &[Entry],
    context: Context,
    inventory: &Inventory,
    incomplete: bool,
    manifest: bool,
    special: &[&str],
    counts: (usize, usize),
) -> Result<(), Error> {
    let (mut receipts, mut payloads) = (0, 0);
    for e in entries {
        let name = e.path.as_str();
        if special.contains(&name) {
            continue;
        }
        if e.kind == Kind::Directory {
            if !directory_allowed(name, context, inventory) {
                return Err(Error::Inventory);
            }
            continue;
        }
        let allowed = match name {
            INCOMPLETE => incomplete,
            MANIFEST => manifest,
            LOCK => e.identity.size == 0,
            ".state/context.json" | PROTOCOL => true,
            _ if name.starts_with(".state/sealed/") => Component::parse(
                name.strip_prefix(".state/sealed/")
                    .ok_or(Error::Inventory)?,
            )
            .is_ok_and(|c| context.contains(c)),
            _ if name.starts_with(".state/files/") => {
                receipts += 1;
                let leaf = &name[13..];
                leaf.strip_suffix(".json").is_some_and(|s| unhex(s).is_ok())
            }
            _ => inventory
                .facts
                .binary_search_by(|a| model::folded(&a.path, name))
                .is_ok_and(|i| {
                    let same = inventory.facts[i].path == name;
                    if same {
                        payloads += 1;
                    }
                    same
                }),
        };
        if !allowed {
            return Err(Error::Inventory);
        }
    }
    for name in BASE_DIRS {
        if !entry(entries, name).is_some_and(|e| e.kind == Kind::Directory) {
            return Err(Error::Inventory);
        }
    }
    if !entry(entries, LOCK).is_some_and(|e| e.kind == Kind::File && e.identity.size == 0) {
        return Err(Error::Inventory);
    }
    if (receipts, payloads) != counts {
        return Err(Error::Inventory);
    }
    Ok(())
}
fn validate_facts(root: &Root, entries: &[Entry], inventory: &Inventory) -> Result<(), Error> {
    for a in &inventory.facts {
        let e = entry(entries, &a.path).ok_or(Error::Inventory)?;
        if e.kind != Kind::File || e.identity.size != a.bytes {
            return Err(Error::Digest);
        }
        hash(root, a, Some(e.identity))?;
    }
    Ok(())
}
/// Windows state v3 writer. Errors poison this writer and retain incomplete
/// state. Retained handles are bounded independently from artifact count.
pub struct Run {
    root: Root,
    _lock: Lock,
    context: Context,
    limits: Limits,
    files: usize,
    bytes: u64,
    failed: bool,
    replay: Option<recover::Replay>,
    intent: Option<Artifact>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    Written,
    AlreadyPresent,
}
impl Run {
    pub fn create(
        output: &Path,
        context: Context,
        protected: &[&Path],
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::create_with_sources(output, context, protected, limits, &[])
    }
    pub fn create_with_sources(
        output: &Path,
        context: Context,
        protected: &[&Path],
        limits: Limits,
        sources: &[&ReadFile],
    ) -> Result<Self, Error> {
        context.validate()?;
        limits.validate()?;
        if limits.directories < 3 {
            return Err(Error::Limit);
        }
        native::protect_output(output, protected, sources)?;
        let mut root = Root::create(output, native_limits(limits))?;
        let lock = root.lock(true)?;
        root.create_dirs(".state/files")?;
        root.create_dirs(".state/sealed")?;
        publish(
            &mut root,
            INCOMPLETE,
            ".state/incomplete.part",
            b"incomplete\n",
        )?;
        publish(
            &mut root,
            ".state/context.json",
            ".state/context.part",
            context.json().as_bytes(),
        )?;
        publish(
            &mut root,
            PROTOCOL,
            ".state/protocol.part",
            protocol_json(context).as_bytes(),
        )?;
        Ok(Self {
            root,
            _lock: lock,
            context,
            limits,
            files: 0,
            bytes: 0,
            failed: false,
            replay: None,
            intent: None,
        })
    }
    pub fn resume(
        output: &Path,
        expected: Context,
        protected: &[&Path],
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::resume_with_sources(output, expected, protected, limits, &[])
    }
    pub fn resume_with_sources(
        output: &Path,
        expected: Context,
        protected: &[&Path],
        limits: Limits,
        sources: &[&ReadFile],
    ) -> Result<Self, Error> {
        recover::resume(output, expected, protected, limits, sources)
    }
    fn state(&self) -> Result<(), Error> {
        state(&self.root, self.context)?;
        if small(&self.root, INCOMPLETE, None)? != b"incomplete\n" {
            return Err(Error::Incomplete);
        }
        Ok(())
    }
    fn replay_bytes(&self) -> usize {
        self.replay.as_ref().map_or(0, recover::Replay::bytes)
    }
    fn acknowledge(&mut self, path: &str) {
        if let Some(r) = &mut self.replay {
            r.acknowledge(path);
            if r.complete() {
                self.replay = None;
            }
        }
    }
    pub fn completed_components(&self) -> Result<u8, Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        self.state()?;
        let entries = observe_charged(&self.root, self.replay_bytes())?;
        let limits = inventory_limits(&self.root, self.limits, &entries, self.replay_bytes())?;
        let mut inventory = collect(&self.root, &entries, self.context, limits)?;
        if let Some(a) = &self.intent {
            inventory.push(a.clone())?;
            inventory.sort()?;
        }
        let mask = seals(&self.root, &entries, self.context, &inventory)?;
        for a in &inventory.facts {
            if self.intent.as_ref() == Some(a) {
                continue;
            }
            let e = entry(&entries, &a.path).ok_or(Error::Inventory)?;
            hash(&self.root, a, Some(e.identity))?;
        }
        let special = if self.intent.is_some() {
            &[TRANSACTION][..]
        } else {
            &[][..]
        };
        let count = inventory.facts.len() - usize::from(self.intent.is_some());
        let manifest = entry(&entries, MANIFEST).is_some();
        if manifest {
            hash(
                &self.root,
                &manifest_fact(self.context, &inventory)?,
                entry(&entries, MANIFEST).map(|e| e.identity),
            )?;
        }
        closed(
            &entries,
            self.context,
            &inventory,
            true,
            manifest,
            special,
            (count, count),
        )?;
        self.root.verify_membership_charged(
            &entries,
            inventory
                .charged_bytes()
                .checked_add(self.replay_bytes())
                .ok_or(Error::Limit)?,
        )?;
        Ok(mask)
    }
    pub fn put(&mut self, component: Component, path: &str, bytes: &[u8]) -> Result<Put, Error> {
        let result = self.put_inner(component, path, bytes);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn put_inner(&mut self, component: Component, path: &str, bytes: &[u8]) -> Result<Put, Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        self.state()?;
        let a = Artifact {
            component,
            path: path.to_owned(),
            bytes: bytes.len() as u64,
            sha256: digest(bytes),
        };
        a.validate(self.context, self.limits)?;
        let name = receipt(&a);
        // Exact existing receipt lookup avoids an inventory/namespace rescan for
        // every emitted file. Native path traversal rejects spelling aliases.
        if let Some(id) = self.root.file_identity(&name)? {
            if small(&self.root, &name, Some(id))? != a.json().as_bytes() {
                return Err(Error::Inventory);
            }
            hash(&self.root, &a, None)?;
            self.acknowledge(path);
            return Ok(Put::AlreadyPresent);
        }
        if self
            .root
            .file_identity(&format!(".state/sealed/{}", component.name()))?
            .is_some()
        {
            return Err(Error::Sealed);
        }
        if self.files >= self.limits.files
            || self.bytes.checked_add(a.bytes).ok_or(Error::Limit)? > self.limits.total_bytes
        {
            return Err(Error::Limit);
        }
        if self.intent.is_none() && self.root.file_identity(path)?.is_some() {
            return Err(Error::Exists);
        }
        if let Some(intent) = &self.intent {
            if intent != &a
                || small(&self.root, TRANSACTION, None)? != transaction(self.context, &a).as_bytes()
            {
                return Err(Error::Inventory);
            }
        } else {
            publish(
                &mut self.root,
                TRANSACTION,
                INTENT_PART,
                transaction(self.context, &a).as_bytes(),
            )?;
        }
        publish(&mut self.root, path, PAYLOAD_PART, bytes)?;
        publish(&mut self.root, &name, RECEIPT_PART, a.json().as_bytes())?;
        let (id, ()) = self.root.with_read_file(TRANSACTION, None, |r| {
            let mut b = Vec::new();
            r.read_to_end(&mut b).map_err(|_| Error::Io)?;
            if b != transaction(self.context, &a).as_bytes() {
                return Err(Error::Changed);
            }
            Ok(())
        })?;
        self.root.remove_same(TRANSACTION, id)?;
        self.intent = None;
        self.files += 1;
        self.bytes += a.bytes;
        self.acknowledge(path);
        Ok(Put::Written)
    }
    pub fn finish_component(&mut self, component: Component) -> Result<(), Error> {
        let result = self.finish_component_inner(component);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn finish_component_inner(&mut self, component: Component) -> Result<(), Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        if !self.context.contains(component) {
            return Err(Error::Component);
        }
        if self.intent.is_some() || self.replay.as_ref().is_some_and(|r| r.pending(component)) {
            return Err(Error::Incomplete);
        }
        self.state()?;
        let entries = observe_charged(&self.root, self.replay_bytes())?;
        let inventory = collect(
            &self.root,
            &entries,
            self.context,
            inventory_limits(&self.root, self.limits, &entries, self.replay_bytes())?,
        )?;
        validate_facts(&self.root, &entries, &inventory)?;
        let mask = seals(&self.root, &entries, self.context, &inventory)?;
        let manifest = entry(&entries, MANIFEST).is_some();
        if manifest && mask != self.context.components {
            return Err(Error::Incomplete);
        }
        if manifest {
            hash(
                &self.root,
                &manifest_fact(self.context, &inventory)?,
                entry(&entries, MANIFEST).map(|e| e.identity),
            )?;
        }
        closed(
            &entries,
            self.context,
            &inventory,
            true,
            manifest,
            &[],
            (inventory.facts.len(), inventory.facts.len()),
        )?;
        self.root.verify_membership_charged(
            &entries,
            inventory
                .charged_bytes()
                .checked_add(self.replay_bytes())
                .ok_or(Error::Limit)?,
        )?;
        self.state()?;
        if mask & component.bit() != 0 {
            return Ok(());
        }
        publish(
            &mut self.root,
            &format!(".state/sealed/{}", component.name()),
            &format!(".state/sealed/.{}.part", component.name()),
            seal(self.context, component, &inventory).as_bytes(),
        )
    }
    pub fn finish(self) -> Result<Summary, Error> {
        self.finish_inner(|_| Ok(()))
    }
    #[cfg(test)]
    fn finish_for_test(
        self,
        after_manifest: impl FnOnce(&mut Root) -> Result<(), Error>,
    ) -> Result<Summary, Error> {
        self.finish_inner(after_manifest)
    }
    fn finish_inner(
        mut self,
        after_manifest: impl FnOnce(&mut Root) -> Result<(), Error>,
    ) -> Result<Summary, Error> {
        if self.failed
            || self.intent.is_some()
            || self.replay.as_ref().is_some_and(|r| !r.complete())
        {
            return Err(Error::Incomplete);
        }
        self.state()?;
        let mut entries = observe_charged(&self.root, self.replay_bytes())?;
        let inventory = collect(
            &self.root,
            &entries,
            self.context,
            inventory_limits(&self.root, self.limits, &entries, self.replay_bytes())?,
        )?;
        if seals(&self.root, &entries, self.context, &inventory)? != self.context.components {
            return Err(Error::Incomplete);
        }
        validate_facts(&self.root, &entries, &inventory)?;
        let manifest = entry(&entries, MANIFEST).is_some();
        let mf = manifest_fact(self.context, &inventory)?;
        if mf.bytes > self.limits.manifest_bytes {
            return Err(Error::Limit);
        }
        if manifest {
            hash(
                &self.root,
                &mf,
                entry(&entries, MANIFEST).map(|e| e.identity),
            )?;
        }
        closed(
            &entries,
            self.context,
            &inventory,
            true,
            manifest,
            &[],
            (inventory.facts.len(), inventory.facts.len()),
        )?;
        self.root.verify_membership_charged(
            &entries,
            inventory
                .charged_bytes()
                .checked_add(self.replay_bytes())
                .ok_or(Error::Limit)?,
        )?;
        self.state()?;
        if !manifest {
            // Admit the exact extra expected binding before publication. Every
            // older file keeps its captured identity across the mutation.
            entries
                .try_reserve_exact(1)
                .map_err(|_| Error::Allocation)?;
            let mut path = String::new();
            path.try_reserve_exact(MANIFEST.len())
                .map_err(|_| Error::Allocation)?;
            path.push_str(MANIFEST);
            let budget = inventory_limits(
                &self.root,
                self.limits,
                &entries,
                path.capacity()
                    .checked_add(self.replay_bytes())
                    .ok_or(Error::Limit)?,
            )?;
            if inventory.charged_bytes() > budget.metadata_bytes {
                return Err(Error::Limit);
            }
            let identity = publish_manifest(&mut self.root, self.context, &inventory, self.limits)?;
            entries.push(Entry {
                path,
                identity,
                kind: Kind::File,
            });
            entries.sort_unstable_by(|a, b| model::folded(&a.path, &b.path));
        }
        let marker = entry(&entries, INCOMPLETE)
            .ok_or(Error::Incomplete)?
            .identity;
        after_manifest(&mut self.root)?;
        closed(
            &entries,
            self.context,
            &inventory,
            true,
            true,
            &[],
            (inventory.facts.len(), inventory.facts.len()),
        )?;
        hash(
            &self.root,
            &mf,
            entry(&entries, MANIFEST).map(|e| e.identity),
        )?;
        self.root.verify_membership_charged(
            &entries,
            inventory
                .charged_bytes()
                .checked_add(self.replay_bytes())
                .ok_or(Error::Limit)?,
        )?;
        if small(&self.root, INCOMPLETE, Some(marker))? != b"incomplete\n" {
            return Err(Error::Incomplete);
        }
        self.root.remove_same(INCOMPLETE, marker)?;
        self.root.guard()?;
        Ok(inventory.summary(self.context))
    }
}
