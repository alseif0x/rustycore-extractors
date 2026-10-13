// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: explicit Windows rename-state recovery and exact replay.
use super::*;
struct ReplayEntry {
    hash: [u8; 32],
    component: Component,
    seen: bool,
}
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
        if count
            .checked_mul(std::mem::size_of::<ReplayEntry>())
            .ok_or(Error::Limit)?
            > inventory.remaining()
        {
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
        if entries.windows(2).any(|w| w[0].hash == w[1].hash) {
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
struct Plan {
    cleanup: Vec<(String, Identity)>,
    special: Vec<String>,
}
impl Plan {
    fn new() -> Self {
        Self {
            cleanup: Vec::new(),
            special: Vec::new(),
        }
    }
    fn special(&mut self, e: &Entry, cleanup: bool) -> Result<(), Error> {
        if self.special.len() >= 16 {
            return Err(Error::Limit);
        }
        self.special
            .try_reserve_exact(1)
            .map_err(|_| Error::Allocation)?;
        self.special.push(e.path.clone());
        if cleanup {
            self.cleanup
                .try_reserve_exact(1)
                .map_err(|_| Error::Allocation)?;
            self.cleanup.push((e.path.clone(), e.identity));
        }
        Ok(())
    }
    // Rename publication preserves link count one and cannot have simultaneous
    // final/pending names. Partial pending bytes are discarded only after every
    // retained fact and all observed membership have passed preflight.
    fn partial(
        &mut self,
        entries: &[Entry],
        temporary: &str,
        final_name: &str,
        cap: u64,
    ) -> Result<(), Error> {
        if let Some(e) = entry(entries, temporary) {
            if entry(entries, final_name).is_some() || e.kind != Kind::File || e.identity.size > cap
            {
                return Err(Error::Inventory);
            }
            self.special(e, true)?;
        }
        Ok(())
    }
    fn apply(&self, root: &mut Root) -> Result<(), Error> {
        for (name, id) in &self.cleanup {
            root.remove_same(name, *id)?;
        }
        Ok(())
    }
}
pub(super) fn resume(
    output: &Path,
    expected: Context,
    protected: &[&Path],
    limits: Limits,
    sources: &[&ReadFile],
) -> Result<Run, Error> {
    resume_inner(output, expected, protected, limits, sources, |_| Ok(()))
}
#[cfg(test)]
pub(super) fn resume_for_test(
    output: &Path,
    expected: Context,
    limits: Limits,
    after_apply: impl FnOnce(&mut Root) -> Result<(), Error>,
) -> Result<Run, Error> {
    resume_inner(output, expected, &[], limits, &[], after_apply)
}
fn resume_inner(
    output: &Path,
    expected: Context,
    protected: &[&Path],
    limits: Limits,
    sources: &[&ReadFile],
    after_apply: impl FnOnce(&mut Root) -> Result<(), Error>,
) -> Result<Run, Error> {
    expected.validate()?;
    limits.validate()?;
    native::protect_output(output, protected, sources)?;
    let mut root = Root::open(output, native_limits(limits))?;
    // The only admitted missing-lock state is the empty root immediately after
    // exclusive creation. Never recreate a lock in an initialized/completed
    // package; doing so would mutate a rejected writer-resume attempt.
    if root.file_identity(LOCK)?.is_none() && !observe(&root)?.is_empty() {
        return Err(Error::Inventory);
    }
    let lock = root.lock(true)?;
    let mut entries = observe(&root)?;
    if entry(&entries, INCOMPLETE).is_none() && entry(&entries, MANIFEST).is_some() {
        return Err(Error::Sealed);
    }
    if entry(&entries, INCOMPLETE).is_none()
        || entry(&entries, ".state/context.json").is_none()
        || entry(&entries, PROTOCOL).is_none()
    {
        return initialize(root, lock, entries, expected, limits, after_apply);
    }
    let marker = entry(&entries, INCOMPLETE).ok_or(Error::Incomplete)?;
    if small(&root, INCOMPLETE, Some(marker.identity))? != b"incomplete\n" {
        return Err(Error::Incomplete);
    }
    if entry(&entries, PROTOCOL).is_none() {
        return Err(Error::StateVersion);
    }
    state(&root, expected)?;
    let mut plan = Plan::new();
    plan.partial(&entries, INTENT_PART, TRANSACTION, LINE as u64)?;
    let tx = entry(&entries, TRANSACTION)
        .map(|e| {
            if e.kind != Kind::File {
                return Err(Error::Path);
            }
            parse_transaction(
                &small(&root, TRANSACTION, Some(e.identity))?,
                expected,
                limits,
            )
        })
        .transpose()?;
    let mut inventory = collect(
        &root,
        &entries,
        expected,
        inventory_limits(&root, limits, &entries, 0)?,
    )?;
    let receipt_count = inventory.facts.len();
    let mut payload_present = false;
    let mut receipt_present = false;
    if let Some(a) = &tx {
        plan.special(entry(&entries, TRANSACTION).ok_or(Error::Inventory)?, false)?;
        plan.partial(&entries, PAYLOAD_PART, &a.path, a.bytes)?;
        let name = receipt(a);
        plan.partial(&entries, RECEIPT_PART, &name, a.json().len() as u64)?;
        if let Some(e) = entry(&entries, &a.path) {
            if e.kind != Kind::File || e.identity.size != a.bytes {
                return Err(Error::Digest);
            }
            hash(&root, a, Some(e.identity))?;
            payload_present = true;
        }
        if let Some(e) = entry(&entries, &name) {
            if !payload_present || small(&root, &name, Some(e.identity))? != a.json().as_bytes() {
                return Err(Error::Inventory);
            }
            receipt_present = true;
        }
        if !payload_present && entry(&entries, RECEIPT_PART).is_some() {
            return Err(Error::Inventory);
        }
        if let Ok(i) = inventory
            .facts
            .binary_search_by(|b| model::folded(&b.path, &a.path))
        {
            if !receipt_present || inventory.facts[i] != *a {
                return Err(Error::Inventory);
            }
        } else {
            if receipt_present {
                return Err(Error::Inventory);
            }
            inventory.push(a.clone())?;
            inventory.sort()?;
        }
    }
    for a in &inventory.facts {
        if tx.as_ref() == Some(a) && !payload_present {
            continue;
        }
        hash(&root, a, entry(&entries, &a.path).map(|e| e.identity))?;
    }
    let mask = seals(&root, &entries, expected, &inventory)?;
    if tx.as_ref().is_some_and(|a| mask & a.component.bit() != 0) {
        return Err(Error::Inventory);
    }
    for c in Component::ALL {
        let temporary = format!(".state/sealed/.{}.part", c.name());
        if entry(&entries, &temporary).is_some() {
            if !expected.contains(c) || tx.is_some() {
                return Err(Error::Incomplete);
            }
            plan.partial(
                &entries,
                &temporary,
                &format!(".state/sealed/{}", c.name()),
                seal(expected, c, &inventory).len() as u64,
            )?;
        }
    }
    let mf = manifest_fact(expected, &inventory)?;
    if mf.bytes > limits.manifest_bytes {
        return Err(Error::Limit);
    }
    let manifest = entry(&entries, MANIFEST);
    let manifest_part = entry(&entries, MANIFEST_PART);
    if manifest.is_some() || manifest_part.is_some() {
        if mask != expected.components || tx.is_some() {
            return Err(Error::Incomplete);
        }
        plan.partial(&entries, MANIFEST_PART, MANIFEST, mf.bytes)?;
        if let Some(e) = manifest {
            hash(&root, &mf, Some(e.identity))?;
        }
    }
    let payload_count =
        receipt_count + usize::from(tx.is_some() && !receipt_present && payload_present);
    let special = plan.special.iter().map(String::as_str).collect::<Vec<_>>();
    closed(
        &entries,
        expected,
        &inventory,
        true,
        manifest.is_some(),
        &special,
        (receipt_count, payload_count),
    )?;
    let replay = Replay::new(&inventory, mask)?;
    root.verify_membership_charged(
        &entries,
        inventory
            .charged_bytes()
            .checked_add(replay.as_ref().map_or(0, Replay::bytes))
            .ok_or(Error::Limit)?,
    )?;
    state(&root, expected)?;
    // No filesystem mutation precedes this point. Retained pending identities
    // are checked again by remove_same; every publication remains no-replace.
    drop(special);
    drop(inventory);
    plan.apply(&mut root)?;
    entries.retain(|e| !plan.cleanup.iter().any(|(name, _)| *name == e.path));
    let mut intent = tx.clone();
    if let Some(a) = &tx
        && payload_present
    {
        hash(&root, a, entry(&entries, &a.path).map(|e| e.identity))?;
        let new_receipt = if !receipt_present {
            let path = receipt(a);
            let identity = publish_file(&mut root, &path, RECEIPT_PART, a.json().as_bytes())?;
            Some(Entry {
                path,
                identity,
                kind: Kind::File,
            })
        } else {
            None
        };
        let e = entry(&entries, TRANSACTION).ok_or(Error::Inventory)?;
        if small(&root, TRANSACTION, Some(e.identity))? != transaction(expected, a).as_bytes() {
            return Err(Error::Changed);
        }
        root.remove_same(TRANSACTION, e.identity)?;
        entries.retain(|e| e.path != TRANSACTION);
        // Removing the original intent always frees this Entry slot; adding
        // its one receipt cannot grow the retained observation allocation.
        if let Some(e) = new_receipt {
            entries.push(e);
            entries.sort_unstable_by(|a, b| model::folded(&a.path, &b.path));
        }
        intent = None;
    }
    after_apply(&mut root)?;
    // Compare against the original file identities plus only the identities of
    // known mutations. A fresh observation must never adopt new foreign facts
    // after Replay was constructed from the closed preflight inventory.
    root.verify_membership_charged(
        &entries,
        128 * 1024 + replay.as_ref().map_or(0, Replay::bytes),
    )?;
    let inventory = collect(
        &root,
        &entries,
        expected,
        inventory_limits(
            &root,
            limits,
            &entries,
            replay.as_ref().map_or(0, Replay::bytes),
        )?,
    )?;
    validate_facts(&root, &entries, &inventory)?;
    state(&root, expected)?;
    root.verify_membership_charged(
        &entries,
        inventory
            .charged_bytes()
            .checked_add(replay.as_ref().map_or(0, Replay::bytes))
            .ok_or(Error::Limit)?,
    )?;
    Ok(Run {
        root,
        _lock: lock,
        context: expected,
        limits,
        files: inventory.facts.len(),
        bytes: inventory.bytes,
        failed: false,
        replay,
        intent,
    })
}

// An interrupted create is a separate empty-inventory state, never an implicit
// migration or adoption of artifacts without receipts. Expected identities bind
// even controls that have not yet been written. Pending bytes are discarded
// only after complete namespace and all finalized controls have been checked.
fn initialize(
    mut root: Root,
    lock: Lock,
    mut entries: Vec<Entry>,
    expected: Context,
    limits: Limits,
    after_apply: impl FnOnce(&mut Root) -> Result<(), Error>,
) -> Result<Run, Error> {
    if limits.directories < 3 {
        return Err(Error::Limit);
    }
    let controls = [
        (
            INCOMPLETE,
            ".state/incomplete.part",
            "incomplete\n".to_owned(),
        ),
        (
            ".state/context.json",
            ".state/context.part",
            expected.json(),
        ),
        (PROTOCOL, ".state/protocol.part", protocol_json(expected)),
    ];
    let mut cleanup = Vec::new();
    for e in &entries {
        if e.kind == Kind::Directory {
            if !BASE_DIRS.contains(&e.path.as_str()) {
                return Err(Error::Inventory);
            }
            continue;
        }
        if e.path == LOCK && e.identity.size == 0 {
            continue;
        }
        let Some((name, temporary, bytes)) = controls
            .iter()
            .find(|(name, temporary, _)| e.path == *name || e.path == *temporary)
        else {
            return Err(Error::Inventory);
        };
        if e.path == *temporary {
            if entry(&entries, name).is_some() || e.identity.size > bytes.len() as u64 {
                return Err(Error::Inventory);
            }
            cleanup.push((e.path.clone(), e.identity));
        } else {
            let actual = small(&root, name, Some(e.identity))?;
            if *name == PROTOCOL {
                let v = parse(std::str::from_utf8(&actual).map_err(|_| Error::Schema)?)?;
                if number(&v, "state_version")? != 3
                    || string(&v, "filesystem_profile")? != FILESYSTEM_PROFILE
                {
                    return Err(Error::StateVersion);
                }
            }
            if actual != bytes.as_bytes() {
                return Err(if *name == INCOMPLETE {
                    Error::Incomplete
                } else {
                    Error::Context
                });
            }
        }
    }
    // At most three directories and three finalized controls are added. Admit
    // the expected membership allocation before any recovery mutation.
    entries
        .try_reserve_exact(6)
        .map_err(|_| Error::Allocation)?;
    root.verify_membership_charged(&entries, 128 * 1024)?;
    for (name, id) in &cleanup {
        root.remove_same(name, *id)?;
    }
    entries.retain(|e| !cleanup.iter().any(|(name, _)| *name == e.path));
    root.create_dirs(".state/files")?;
    root.create_dirs(".state/sealed")?;
    for (name, temporary, bytes) in controls {
        if entry(&entries, name).is_none() {
            let identity = publish_file(&mut root, name, temporary, bytes.as_bytes())?;
            entries.push(Entry {
                path: name.to_owned(),
                identity,
                kind: Kind::File,
            });
            entries.sort_unstable_by(|a, b| model::folded(&a.path, &b.path));
        }
    }
    after_apply(&mut root)?;
    // Only the explicitly created base directories need identities captured
    // after creation. Preserve every original file/control identity and every
    // returned publication identity; never adopt newly observed control or
    // manifest files during bootstrap.
    let observed = observe_charged(&root, 128 * 1024 + observed_bytes(&entries)?)?;
    for e in &entries {
        let actual = entry(&observed, &e.path).ok_or(Error::Changed)?;
        if actual.kind != e.kind
            || if e.kind == Kind::Directory {
                !actual.identity.same_object(e.identity)
            } else {
                actual.identity != e.identity
            }
        {
            return Err(Error::Changed);
        }
    }
    for e in observed {
        if entry(&entries, &e.path).is_none() {
            if e.kind != Kind::Directory || !BASE_DIRS.contains(&e.path.as_str()) {
                return Err(Error::Inventory);
            }
            entries.push(e);
            entries.sort_unstable_by(|a, b| model::folded(&a.path, &b.path));
        }
    }
    root.verify_membership_charged(&entries, 128 * 1024)?;
    state(&root, expected)?;
    Ok(Run {
        root,
        _lock: lock,
        context: expected,
        limits,
        files: 0,
        bytes: 0,
        failed: false,
        replay: None,
        intent: None,
    })
}
