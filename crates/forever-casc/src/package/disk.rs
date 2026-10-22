// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: exclusive Linux package staging and offline integrity QA.
#![cfg(target_os = "linux")]
use super::*;
pub(super) use model::receipt;
use model::{
    Inventory, PartialEncryptedSection, footer, header_with_partial_sections,
    manifest_fact_with_partial_sections, parse_header_partial_sections, seal,
    validate_partial_encrypted_sections,
};
use std::{
    fs::{DirBuilder, File, Metadata, OpenOptions},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Component as Part, PathBuf},
};
#[cfg(test)]
mod admission_tests;
#[cfg(test)]
mod partial_manifest_tests;
mod recover;
const NOFOLLOW: i32 = 0x20000;
const DIRECTORY: i32 = 0x10000;
const NONBLOCK: i32 = 0x800;
#[derive(PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mns: i64,
    ctime: i64,
    cns: i64,
}
impl Identity {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            links: m.nlink(),
            len: m.len(),
            mtime: m.mtime(),
            mns: m.mtime_nsec(),
            ctime: m.ctime(),
            cns: m.ctime_nsec(),
        }
    }
}
fn anchor(f: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", f.as_raw_fd()))
}
fn dir(path: &Path) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .custom_flags(NOFOLLOW | DIRECTORY | NONBLOCK)
        .open(path)
        .map_err(|_| Error::Path)
}
fn sync(f: &File) -> Result<(), Error> {
    f.sync_all().map_err(|_| Error::Io)
}
fn directory(path: &Path) -> Result<(), Error> {
    DirBuilder::new().mode(0o700).create(path).map_err(|e| {
        if e.kind() == io::ErrorKind::AlreadyExists {
            Error::Exists
        } else {
            Error::Io
        }
    })
}
fn safe(path: &Path) -> Result<(), Error> {
    if !path.is_absolute()
        || path.as_os_str().len() > 4096
        || path
            .as_os_str()
            .as_encoded_bytes()
            .split(|&b| b == b'/')
            .any(|s| s == b"." || s == b"..")
    {
        return Err(Error::Path);
    }
    for p in path.ancestors() {
        let m = fs::symlink_metadata(p).map_err(|_| Error::Path)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(Error::Path);
        }
    }
    Ok(())
}
struct Root {
    held: File,
    named: PathBuf,
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}
impl Root {
    fn open(path: &Path) -> Result<Self, Error> {
        safe(path)?;
        let named = fs::canonicalize(path).map_err(|_| Error::Path)?;
        let held = dir(&named)?;
        let m = held.metadata().map_err(|_| Error::Io)?;
        let out = Self {
            held,
            named,
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
        };
        out.guard()?;
        Ok(out)
    }
    fn guard(&self) -> Result<(), Error> {
        safe(&self.named).map_err(|_| Error::Changed)?;
        let a = self.held.metadata().map_err(|_| Error::Changed)?;
        let b = fs::symlink_metadata(&self.named).map_err(|_| Error::Changed)?;
        for m in [a, b] {
            if m.dev() != self.dev
                || m.ino() != self.ino
                || m.mode() != self.mode
                || m.uid() != self.uid
                || m.gid() != self.gid
            {
                return Err(Error::Changed);
            }
        }
        Ok(())
    }
    fn parent<'a>(
        &self,
        name: &'a str,
        create: bool,
        dirs: &mut usize,
        limits: Limits,
    ) -> Result<(File, &'a str), Error> {
        self.guard()?;
        let mut parts = name.split('/').peekable();
        let mut fd = self.held.try_clone().map_err(|_| Error::Io)?;
        let mut depth = 0;
        loop {
            let p = parts.next().ok_or(Error::Path)?;
            if p.is_empty() || p == "." || p == ".." {
                return Err(Error::Path);
            }
            if parts.peek().is_none() {
                return Ok((fd, p));
            }
            depth += 1;
            if depth > 16 {
                return Err(Error::Limit);
            }
            let path = anchor(&fd).join(p);
            if create
                && fs::symlink_metadata(&path).is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
            {
                if *dirs >= limits.directories {
                    return Err(Error::Limit);
                }
                directory(&path)?;
                *dirs += 1;
                sync(&fd)?;
            }
            fd = dir(&path)?;
        }
    }
    fn open_file(&self, name: &str, limits: Limits) -> Result<File, Error> {
        let (p, leaf) = self.parent(name, false, &mut 0, limits)?;
        OpenOptions::new()
            .read(true)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(anchor(&p).join(leaf))
            .map_err(|_| Error::Io)
    }
    fn named_file(&self, name: &str, held: &File, limits: Limits) -> Result<(), Error> {
        let current = self.open_file(name, limits)?;
        if Identity::of(&current.metadata().map_err(|_| Error::Changed)?)
            != Identity::of(&held.metadata().map_err(|_| Error::Changed)?)
        {
            return Err(Error::Changed);
        }
        self.guard()
    }
    fn small(&self, name: &str, limits: Limits) -> Result<Vec<u8>, Error> {
        self.small_links(name, limits, 1)
    }
    fn small_links(&self, name: &str, limits: Limits, links: u64) -> Result<Vec<u8>, Error> {
        let mut f = self.open_file(name, limits)?;
        self.named_file(name, &f, limits)?;
        let m = f.metadata().map_err(|_| Error::Io)?;
        if !m.is_file() || m.nlink() != links {
            return Err(Error::Path);
        }
        if m.len() > LINE as u64 {
            return Err(Error::Limit);
        }
        let mut out = Vec::new();
        Read::by_ref(&mut f)
            .take((LINE + 1) as u64)
            .read_to_end(&mut out)
            .map_err(|_| Error::Io)?;
        if out.len() > LINE {
            return Err(Error::Limit);
        }
        if Identity::of(&m) != Identity::of(&f.metadata().map_err(|_| Error::Changed)?) {
            return Err(Error::Changed);
        }
        self.named_file(name, &f, limits)?;
        Ok(out)
    }
    fn has(&self, name: &str, limits: Limits) -> Result<bool, Error> {
        let (p, leaf) = self.parent(name, false, &mut 0, limits)?;
        match fs::symlink_metadata(anchor(&p).join(leaf)) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(_) => Err(Error::Io),
        }
    }
    fn publish(
        &self,
        name: &str,
        bytes: &[u8],
        dirs: &mut usize,
        limits: Limits,
    ) -> Result<(), Error> {
        let (p, leaf) = self.parent(name, true, dirs, limits)?;
        let path = anchor(&p).join(leaf);
        if path.exists() {
            return Err(Error::Exists);
        }
        let pending = anchor(&p).join(format!(".{leaf}.part"));
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(&pending)
            .map_err(|_| Error::Exists)?;
        f.write_all(bytes).map_err(|_| Error::Io)?;
        sync(&f)?;
        fs::hard_link(&pending, &path).map_err(|_| Error::Exists)?;
        fs::remove_file(pending).map_err(|_| Error::Io)?;
        sync(&p)?;
        self.guard()
    }
    fn hash(&self, a: &Artifact, limits: Limits) -> Result<(), Error> {
        self.hash_links(a, limits, 1)
    }
    fn hash_links(&self, a: &Artifact, limits: Limits, links: u64) -> Result<(), Error> {
        let mut f = self.open_file(&a.path, limits)?;
        self.named_file(&a.path, &f, limits)?;
        let m = f.metadata().map_err(|_| Error::Io)?;
        if !m.is_file() || m.nlink() != links {
            return Err(Error::Path);
        }
        if m.len() != a.bytes {
            return Err(Error::Digest);
        }
        if m.len() > limits.file_bytes {
            return Err(Error::Limit);
        }
        let mut h = sha256::Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0; 65536];
        loop {
            let n = f.read(&mut buffer).map_err(|_| Error::Io)?;
            if n == 0 {
                break;
            }
            count = count.checked_add(n as u64).ok_or(Error::Limit)?;
            if count > a.bytes {
                return Err(Error::Changed);
            }
            h.update(&buffer[..n]);
        }
        if count != a.bytes || h.finish() != a.sha256 {
            return Err(Error::Digest);
        }
        if Identity::of(&m) != Identity::of(&f.metadata().map_err(|_| Error::Changed)?) {
            return Err(Error::Changed);
        }
        self.named_file(&a.path, &f, limits)
    }
    fn lock(&self, write: bool, limits: Limits) -> Result<PackageLock, Error> {
        let (p, leaf) = self.parent(".state/lock", false, &mut 0, limits)?;
        let f = OpenOptions::new()
            .read(true)
            .write(write)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(anchor(&p).join(leaf))
            .map_err(|_| Error::Io)?;
        let m = f.metadata().map_err(|_| Error::Io)?;
        if !m.is_file() || m.len() != 0 || m.nlink() != 1 {
            return Err(Error::Path);
        }
        let r = if write {
            f.try_lock()
        } else {
            f.try_lock_shared()
        };
        r.map_err(|_| Error::Busy)?;
        Ok(PackageLock(f))
    }
}
fn context(root: &Root, limits: Limits) -> Result<Context, Error> {
    let b = root.small(".state/context.json", limits)?;
    Context::parse(std::str::from_utf8(&b).map_err(|_| Error::Schema)?)
}
fn collect(root: &Root, context: Context, limits: Limits) -> Result<Inventory, Error> {
    let path = anchor(&root.held).join(".state/files");
    let fd = dir(&path)?;
    let before = fd.metadata().map_err(|_| Error::Io)?;
    let mut inventory = Inventory::new(limits);
    for e in fs::read_dir(anchor(&fd)).map_err(|_| Error::Io)? {
        let e = e.map_err(|_| Error::Io)?;
        let name = e.file_name().into_string().map_err(|_| Error::Path)?;
        if !name.ends_with(".json") || name.len() != 69 {
            return Err(Error::Inventory);
        }
        let b = root.small(&format!(".state/files/{name}"), limits)?;
        let a = Artifact::parse(
            std::str::from_utf8(&b).map_err(|_| Error::Schema)?,
            context,
            limits,
        )?;
        if receipt(&a) != format!(".state/files/{name}") {
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
fn check_seals(
    root: &Root,
    context: Context,
    inventory: &Inventory,
    limits: Limits,
) -> Result<(), Error> {
    let mask = seals(root, context, inventory, limits)?;
    if mask != context.components {
        return Err(Error::Incomplete);
    }
    Ok(())
}
fn seals(
    root: &Root,
    context: Context,
    inventory: &Inventory,
    limits: Limits,
) -> Result<u8, Error> {
    let fd = dir(&anchor(&root.held).join(".state/sealed"))?;
    let mut mask = 0;
    for e in fs::read_dir(anchor(&fd)).map_err(|_| Error::Io)? {
        let name = e
            .map_err(|_| Error::Io)?
            .file_name()
            .into_string()
            .map_err(|_| Error::Path)?;
        let c = Component::parse(&name)?;
        if !context.contains(c) || mask & c.bit() != 0 {
            return Err(Error::Component);
        }
        mask |= c.bit();
        let bytes = root.small(&format!(".state/sealed/{name}"), limits)?;
        if bytes != seal(context, c, inventory).as_bytes() {
            return Err(Error::Inventory);
        }
    }
    Ok(mask)
}
fn manifest(
    root: &Root,
    context: Context,
    inventory: &Inventory,
    partial_sections: &[PartialEncryptedSection],
    dirs: &mut usize,
    limits: Limits,
) -> Result<(), Error> {
    if root.has(MANIFEST, limits)? {
        return root.hash(
            &manifest_fact_with_partial_sections(context, inventory, partial_sections)?,
            Limits {
                file_bytes: limits.manifest_bytes,
                ..limits
            },
        );
    }
    let pending = ".state/package.part";
    let (p, leaf) = root.parent(pending, false, dirs, limits)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(NOFOLLOW)
        .open(anchor(&p).join(leaf))
        .map_err(|_| Error::Exists)?;
    let mut written = 0u64;
    let mut emit = |s: &str| -> Result<(), Error> {
        written = written.checked_add(s.len() as u64).ok_or(Error::Limit)?;
        if written > limits.manifest_bytes {
            return Err(Error::Limit);
        }
        f.write_all(s.as_bytes()).map_err(|_| Error::Io)
    };
    emit(&header_with_partial_sections(context, partial_sections)?)?;
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
    sync(&f)?;
    fs::hard_link(anchor(&p).join(leaf), anchor(&root.held).join(MANIFEST))
        .map_err(|_| Error::Exists)?;
    fs::remove_file(anchor(&p).join(leaf)).map_err(|_| Error::Io)?;
    sync(&root.held)
}
// Bind the OS lock to this owner, even when a child briefly retains a duplicate
// open description before exec closes its CLOEXEC descriptors. Closing only our
// descriptor would leave that shared-description lock held by the child.
struct PackageLock(File);
impl std::ops::Deref for PackageLock {
    type Target = File;
    fn deref(&self) -> &File {
        &self.0
    }
}
impl Drop for PackageLock {
    fn drop(&mut self) {
        // Closing follows regardless; errors cannot be returned from Drop.
        // A failed unlock retains the previous close-only fallback behavior.
        let _ = self.0.unlock();
    }
}
/// New private output only. Drop/error leaves the incomplete marker. Completion
/// must be called only after each selected producer returned its complete token.
/// State v2 supports explicit same-context recovery; completed output stays immutable.
pub struct Run {
    root: Root,
    _lock: PackageLock,
    context: Context,
    limits: Limits,
    dirs: usize,
    files: usize,
    bytes: u64,
    failed: bool,
    replay: Option<recover::Replay>,
    partial_encrypted_sections: Option<Vec<PartialEncryptedSection>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    Written,
    AlreadyPresent,
}
impl Run {
    fn inventory_limits(&self) -> Limits {
        Limits {
            metadata_bytes: self.limits.metadata_bytes
                - self.replay.as_ref().map_or(0, recover::Replay::bytes),
            ..self.limits
        }
    }
    fn acknowledge(&mut self, path: &str) {
        if let Some(replay) = &mut self.replay {
            replay.acknowledge(path);
            if replay.complete() {
                self.replay = None;
            }
        }
    }
    fn state(&self) -> Result<(), Error> {
        if context(&self.root, self.limits)? != self.context {
            return Err(Error::Context);
        }
        recover::protocol(&self.root, self.context, self.limits)?;
        self.root
            .named_file(".state/lock", &self._lock, self.limits)
    }
    pub fn create(
        output: &Path,
        context: Context,
        protected: &[&Path],
        limits: Limits,
    ) -> Result<Self, Error> {
        context.validate()?;
        limits.validate()?;
        let output = recover::output(output, protected)?;
        let parent = output.parent().ok_or(Error::Path)?;
        if limits.directories < 3 {
            return Err(Error::Limit);
        }
        directory(&output)?;
        sync(&dir(parent)?)?;
        let root = Root::open(&output)?;
        let mut dirs = 0;
        root.publish(INCOMPLETE, b"incomplete\n", &mut dirs, limits)?;
        root.publish(".state/lock", b"", &mut dirs, limits)?;
        let lock = root.lock(true, limits)?;
        root.publish(
            ".state/context.json",
            context.json().as_bytes(),
            &mut dirs,
            limits,
        )?;
        for name in ["files", "sealed"] {
            directory(&anchor(&root.held).join(".state").join(name))?;
            dirs += 1;
        }
        sync(&dir(&anchor(&root.held).join(".state"))?)?;
        root.publish(
            recover::PROTOCOL,
            recover::protocol_json(context).as_bytes(),
            &mut dirs,
            limits,
        )?;
        Ok(Self {
            root,
            _lock: lock,
            context,
            limits,
            dirs,
            files: 0,
            bytes: 0,
            failed: false,
            replay: None,
            partial_encrypted_sections: None,
        })
    }
    /// All retained bytes are checked before returning a writer. The caller must
    /// supply the exact expected source/plan/producer context and replay unfinished
    /// producers to their complete result. No implicit adoption or v1 migration.
    pub fn resume(
        output: &Path,
        expected: Context,
        protected: &[&Path],
        limits: Limits,
    ) -> Result<Self, Error> {
        recover::resume(output, expected, protected, limits)
    }
    /// Verified producer seals may skip already finished selected components.
    pub fn completed_components(&self) -> Result<u8, Error> {
        self.state()?;
        let inventory = collect(&self.root, self.context, self.inventory_limits())?;
        seals(&self.root, self.context, &inventory, self.limits)
    }
    /// Attach the redacted E-frame declarations to the final manifest. This is
    /// a product annotation only; it never authenticates or decodes plaintext.
    pub fn set_partial_encrypted_sections(
        &mut self,
        sections: &[PartialEncryptedSection],
    ) -> Result<(), Error> {
        let result = (|| {
            if self.failed {
                return Err(Error::Incomplete);
            }
            self.state()?;
            if self.context.build != 70450
                || self.context.components != (Component::Db2.bit() | Component::Terrain.bit())
            {
                return Err(Error::Component);
            }
            validate_partial_encrypted_sections(sections)?;
            if let Some(previous) = &self.partial_encrypted_sections {
                if previous != sections {
                    return Err(Error::Inventory);
                }
                return Ok(());
            }
            let mut retained = Vec::new();
            retained
                .try_reserve_exact(sections.len())
                .map_err(|_| Error::Allocation)?;
            for section in sections {
                let mut lookup = String::new();
                lookup
                    .try_reserve_exact(section.lookup.len())
                    .map_err(|_| Error::Allocation)?;
                lookup.push_str(&section.lookup);
                retained.push(PartialEncryptedSection {
                    lookup,
                    frame_ordinal: section.frame_ordinal,
                    decoded_size: section.decoded_size,
                });
            }
            self.partial_encrypted_sections = Some(retained);
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Compare a caller-supplied identity with an already published, sealed
    /// artifact. This never publishes, acknowledges replay or changes counters.
    /// Reads use the retained root and exact receipt, size, digest and EOF checks;
    /// successful comparison is guarded observation, not an atomic snapshot.
    /// Any failure permanently poisons this writer, as with `put`.
    pub fn verify_sealed_artifact(&mut self, expected: &Artifact) -> Result<(), Error> {
        let result = (|| {
            if self.failed {
                return Err(Error::Incomplete);
            }
            self.state()?;
            expected.validate(self.context, self.limits)?;
            if self.completed_components()? & expected.component.bit() == 0 {
                return Err(Error::Incomplete);
            }
            let seal_name = format!(".state/sealed/{}", expected.component.name());
            let sealed = self.root.small(&seal_name, self.limits)?;
            let receipt_name = receipt(expected);
            let canonical = expected.json();
            if self.root.small(&receipt_name, self.limits)? != canonical.as_bytes() {
                return Err(Error::Inventory);
            }
            self.root.hash(expected, self.limits)?;
            if self.root.small(&receipt_name, self.limits)? != canonical.as_bytes()
                || self.root.small(&seal_name, self.limits)? != sealed
                || self.completed_components()? & expected.component.bit() == 0
            {
                return Err(Error::Inventory);
            }
            self.state()
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn put(&mut self, component: Component, path: &str, bytes: &[u8]) -> Result<Put, Error> {
        let result = (|| {
            if self.failed {
                return Err(Error::Incomplete);
            }
            self.state()?;
            if bytes.len() as u64 > self.limits.file_bytes {
                return Err(Error::Limit);
            }
            let a = Artifact {
                component,
                path: path.to_owned(),
                bytes: bytes.len() as u64,
                sha256: digest(bytes),
            };
            a.validate(self.context, self.limits)?;
            let receipt = receipt(&a);
            if self.root.has(&receipt, self.limits)? {
                if self.root.small(&receipt, self.limits)? != a.json().as_bytes() {
                    return Err(Error::Inventory);
                }
                self.root.hash(&a, self.limits)?;
                self.acknowledge(path);
                return Ok(Put::AlreadyPresent);
            }
            if self
                .root
                .has(&format!(".state/sealed/{}", component.name()), self.limits)?
            {
                return Err(Error::Sealed);
            }
            if self.files >= self.limits.files
                || self.bytes.checked_add(a.bytes).ok_or(Error::Limit)? > self.limits.total_bytes
            {
                return Err(Error::Limit);
            }
            if self.root.has(recover::TRANSACTION, self.limits)? {
                if self.root.small(recover::TRANSACTION, self.limits)?
                    != recover::transaction(self.context, &a).as_bytes()
                {
                    return Err(Error::Inventory);
                }
            } else {
                self.root.publish(
                    recover::TRANSACTION,
                    recover::transaction(self.context, &a).as_bytes(),
                    &mut self.dirs,
                    self.limits,
                )?;
            }
            self.root
                .publish(path, bytes, &mut self.dirs, self.limits)?;
            self.root
                .publish(&receipt, a.json().as_bytes(), &mut self.dirs, self.limits)?;
            recover::remove(&self.root, recover::TRANSACTION, self.limits)?;
            self.files += 1;
            self.bytes += a.bytes;
            self.acknowledge(path);
            Ok(Put::Written)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn finish_component(&mut self, component: Component) -> Result<(), Error> {
        let result = (|| {
            if self.failed {
                return Err(Error::Incomplete);
            }
            self.state()?;
            if !self.context.contains(component) {
                return Err(Error::Component);
            }
            if self.replay.as_ref().is_some_and(|r| r.pending(component)) {
                return Err(Error::Incomplete);
            }
            if self.root.has(recover::TRANSACTION, self.limits)? {
                return Err(Error::Incomplete);
            }
            let inventory = collect(&self.root, self.context, self.inventory_limits())?;
            for a in inventory.facts.iter().filter(|a| a.component == component) {
                self.root.hash(a, self.limits)?;
            }
            let name = format!(".state/sealed/{}", component.name());
            if self.root.has(&name, self.limits)? {
                if self.root.small(&name, self.limits)?
                    != seal(self.context, component, &inventory).as_bytes()
                {
                    return Err(Error::Inventory);
                }
                return Ok(());
            }
            self.root.publish(
                &name,
                seal(self.context, component, &inventory).as_bytes(),
                &mut self.dirs,
                self.limits,
            )
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn finish(mut self) -> Result<Summary, Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        self.state()?;
        if self.replay.as_ref().is_some_and(|r| !r.complete()) {
            return Err(Error::Incomplete);
        }
        if self.root.has(recover::TRANSACTION, self.limits)? {
            return Err(Error::Incomplete);
        }
        let inventory = collect(&self.root, self.context, self.inventory_limits())?;
        let partial_sections = self
            .partial_encrypted_sections
            .as_deref()
            .unwrap_or_default();
        check_seals(&self.root, self.context, &inventory, self.limits)?;
        for a in &inventory.facts {
            self.root.hash(a, self.limits)?;
        }
        // Namespace closure before manifest publication: no ignored payloads.
        let published = self.root.has(MANIFEST, self.limits)?;
        if published {
            self.root.hash(
                &manifest_fact_with_partial_sections(self.context, &inventory, partial_sections)?,
                Limits {
                    file_bytes: self.limits.manifest_bytes,
                    ..self.limits
                },
            )?;
        }
        scan(
            &self.root,
            &inventory,
            self.context,
            self.limits,
            false,
            published,
            &[],
            (inventory.facts.len(), inventory.facts.len()),
        )?;
        self.state()?;
        manifest(
            &self.root,
            self.context,
            &inventory,
            partial_sections,
            &mut self.dirs,
            self.limits,
        )?;
        fs::remove_file(anchor(&self.root.held).join(INCOMPLETE)).map_err(|_| Error::Io)?;
        sync(&self.root.held)?;
        self.root.guard()?;
        Ok(inventory.summary(self.context))
    }
}
fn tree(
    root: &Root,
    inventory: &Inventory,
    context: Context,
    limits: Limits,
    complete: bool,
) -> Result<(), Error> {
    scan(
        root,
        inventory,
        context,
        limits,
        complete,
        complete,
        &[],
        (inventory.facts.len(), inventory.facts.len()),
    )
    .map(|_| ())
}
#[allow(clippy::too_many_arguments)]
fn scan(
    root: &Root,
    inventory: &Inventory,
    context: Context,
    limits: Limits,
    complete: bool,
    manifest: bool,
    extra: &[(String, Identity)],
    counts: (usize, usize),
) -> Result<usize, Error> {
    struct Walker<'a> {
        root: &'a Root,
        inventory: &'a Inventory,
        context: Context,
        limits: Limits,
        complete: bool,
        manifest: bool,
        extra: &'a [(String, Identity)],
        dirs: usize,
        files: usize,
        receipts: usize,
        payloads: usize,
    }
    impl Walker<'_> {
        fn payload(&self, name: &str) -> bool {
            self.inventory
                .facts
                .binary_search_by(|a| model::folded(&a.path, name))
                .is_ok_and(|i| self.inventory.facts[i].path == name)
        }
        fn directory(&self, name: &str) -> bool {
            if [".state", ".state/files", ".state/sealed"].contains(&name) {
                return true;
            }
            if Component::ALL.into_iter().any(|c| {
                self.context.contains(c)
                    && name == if c == Component::Db2 { "dbc" } else { c.name() }
            }) {
                return true;
            }
            let prefix = format!("{name}/");
            let i = self
                .inventory
                .facts
                .partition_point(|a| model::folded(&a.path, &prefix) == std::cmp::Ordering::Less);
            self.inventory
                .facts
                .get(i)
                .is_some_and(|a| a.path.starts_with(&prefix))
        }
        fn walk(&mut self, fd: &File, prefix: &str, depth: usize) -> Result<(), Error> {
            if depth > 16 {
                return Err(Error::Limit);
            }
            let before = fd.metadata().map_err(|_| Error::Io)?;
            for e in fs::read_dir(anchor(fd)).map_err(|_| Error::Io)? {
                let e = e.map_err(|_| Error::Io)?;
                let leaf = e.file_name().into_string().map_err(|_| Error::Path)?;
                let name = if prefix.is_empty() {
                    leaf.clone()
                } else {
                    format!("{prefix}/{leaf}")
                };
                let m = fs::symlink_metadata(e.path()).map_err(|_| Error::Changed)?;
                if self
                    .extra
                    .iter()
                    .any(|(n, id)| n == &name && *id == Identity::of(&m))
                {
                    if !m.is_file() {
                        return Err(Error::Path);
                    }
                    continue;
                }
                if m.file_type().is_symlink() {
                    return Err(Error::Path);
                }
                if m.is_dir() {
                    self.dirs += 1;
                    if self.dirs > self.limits.directories {
                        return Err(Error::Limit);
                    }
                    if !self.directory(&name) {
                        return Err(Error::Inventory);
                    }
                    let next = dir(&e.path())?;
                    if Identity::of(&m)
                        != Identity::of(&next.metadata().map_err(|_| Error::Changed)?)
                    {
                        return Err(Error::Changed);
                    }
                    self.walk(&next, &name, depth + 1)?;
                    if Identity::of(&m)
                        != Identity::of(
                            &fs::symlink_metadata(e.path()).map_err(|_| Error::Changed)?,
                        )
                    {
                        return Err(Error::Changed);
                    }
                } else if m.is_file() && m.nlink() == 1 {
                    self.files += 1;
                    if self.files > self.limits.files.saturating_mul(2) + 16 {
                        return Err(Error::Limit);
                    }
                    let allowed = if name == MANIFEST {
                        self.manifest
                    } else if name == INCOMPLETE {
                        !self.complete
                    } else if name == ".state/lock" {
                        m.len() == 0
                    } else if name == ".state/context.json" || name == recover::PROTOCOL {
                        true
                    } else if prefix == ".state/sealed" {
                        Component::parse(&leaf).is_ok_and(|c| self.context.contains(c))
                    } else if prefix == ".state/files" {
                        // Every expected receipt has already been read and matched.
                        // Exact cardinality closes extras without an O(files^2) scan.
                        self.receipts += 1;
                        leaf.strip_suffix(".json").is_some_and(|v| unhex(v).is_ok())
                    } else {
                        let present = self.payload(&name);
                        if present {
                            self.payloads += 1;
                        }
                        present
                    };
                    if !allowed {
                        return Err(Error::Inventory);
                    }
                } else {
                    return Err(Error::Path);
                }
            }
            if Identity::of(&before) != Identity::of(&fd.metadata().map_err(|_| Error::Changed)?) {
                return Err(Error::Changed);
            }
            self.root.guard()
        }
    }
    let mut w = Walker {
        root,
        inventory,
        context,
        limits,
        complete,
        manifest,
        extra,
        dirs: 0,
        files: 0,
        receipts: 0,
        payloads: 0,
    };
    w.walk(&root.held, "", 0)?;
    let special_receipts = extra
        .iter()
        .filter(|(n, _)| {
            n.starts_with(".state/files/") && !n.rsplit('/').next().unwrap_or("").starts_with('.')
        })
        .count();
    let special_payloads = extra
        .iter()
        .filter(|(n, _)| {
            inventory
                .facts
                .binary_search_by(|a| model::folded(&a.path, n))
                .is_ok_and(|i| inventory.facts[i].path == *n)
        })
        .count();
    if w.receipts + special_receipts != counts.0 || w.payloads + special_payloads != counts.1 {
        return Err(Error::Inventory);
    }
    Ok(w.dirs)
}
const MAX_MANIFEST_HEADER_BYTES: usize = 4096;
fn manifest_header_line<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    Read::take(reader, (MAX_MANIFEST_HEADER_BYTES + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() > MAX_MANIFEST_HEADER_BYTES {
        return Err(Error::Limit);
    }
    if bytes.last() != Some(&b'\n') {
        return Err(Error::Schema);
    }
    Ok(bytes)
}

/// Offline integrity only. No client/CASC reads; refuses active writers and all
/// incomplete markers, malformed manifests, missing/extra/changed payloads.
pub fn verify(output: &Path, expected: Option<Context>, limits: Limits) -> Result<Summary, Error> {
    verify_inner(output, expected, limits, None, None, None)
}

/// Inspect read-only rooted artifacts while holding the shared package lock.
/// Callbacks run only after integrity/namespace preflight, but their results
/// remain provisional until the whole call returns. Caller allocations and
/// semantic checks are outside the package inventory's logical memory budget.
pub fn verify_with(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    mut inspect: impl FnMut(&Artifact, &mut File) -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_inner(output, expected, limits, None, Some(&mut inspect), None)
}
/// Admit the complete canonical artifact metadata before any artifact payload
/// hashing, reading or inspection. Manifest, receipt and seal metadata are
/// validated first. The admission callback runs once over the whole inventory,
/// allowing order-independent declared-length and cumulative component limits.
/// As with `verify_with`, all callback results remain provisional until success;
/// caller allocations and semantic limits are separate from package accounting.
pub fn verify_with_preflight(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    mut admit: impl FnMut(&[Artifact]) -> Result<(), Error>,
    mut inspect: impl FnMut(&Artifact, &mut File) -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_inner(
        output,
        expected,
        limits,
        Some(&mut admit),
        Some(&mut inspect),
        None,
    )
}
/// As `verify_with_preflight`, additionally exposing validated deterministic
/// partial-section declarations to a product-specific offline verifier.
pub fn verify_with_preflight_and_partial_sections(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    mut partial_sections: impl FnMut(&[PartialEncryptedSection]) -> Result<(), Error>,
    mut admit: impl FnMut(&[Artifact]) -> Result<(), Error>,
    mut inspect: impl FnMut(&Artifact, &mut File) -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_inner(
        output,
        expected,
        limits,
        Some(&mut admit),
        Some(&mut inspect),
        Some(&mut partial_sections),
    )
}
type Admitter<'a> = &'a mut dyn FnMut(&[Artifact]) -> Result<(), Error>;
type Inspector<'a> = &'a mut dyn FnMut(&Artifact, &mut File) -> Result<(), Error>;
type PartialVisitor<'a> = &'a mut dyn FnMut(&[PartialEncryptedSection]) -> Result<(), Error>;
fn verify_inner(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    admit: Option<Admitter<'_>>,
    inspect: Option<Inspector<'_>>,
    partial_visitor: Option<PartialVisitor<'_>>,
) -> Result<Summary, Error> {
    limits.validate()?;
    let root = Root::open(output)?;
    let _lock = root.lock(false, limits)?;
    let before = root.held.metadata().map_err(|_| Error::Io)?;
    if root.has(INCOMPLETE, limits)? {
        return Err(Error::Incomplete);
    }
    let context = context(&root, limits)?;
    if root.has(recover::PROTOCOL, limits)? {
        recover::protocol(&root, context, limits)?;
    }
    if root.has(recover::TRANSACTION, limits)? {
        return Err(Error::Incomplete);
    }
    if expected.is_some_and(|v| v != context) {
        return Err(Error::Context);
    }
    let f = root.open_file(MANIFEST, limits)?;
    let meta = f.metadata().map_err(|_| Error::Io)?;
    if !meta.is_file() || meta.nlink() != 1 {
        return Err(Error::Path);
    }
    if meta.len() > limits.manifest_bytes {
        return Err(Error::Limit);
    }
    let mut reader = io::BufReader::new(f);
    let header_line = manifest_header_line(&mut reader)?;
    let header_text = std::str::from_utf8(header_line.strip_suffix(b"\n").ok_or(Error::Schema)?)
        .map_err(|_| Error::Schema)?;
    let partial_sections = parse_header_partial_sections(header_text, context)?;
    let mut inventory = Inventory::new(limits);
    loop {
        let b = line(&mut reader)?;
        if b.starts_with(b"]") {
            let summary = inventory.summary(context);
            if b != footer(summary).as_bytes() {
                return Err(Error::Inventory);
            }
            let mut eof = [0];
            if reader.read(&mut eof).map_err(|_| Error::Io)? != 0 {
                return Err(Error::Schema);
            }
            break;
        }
        let s = std::str::from_utf8(&b).map_err(|_| Error::Schema)?;
        let s = s.strip_suffix('\n').ok_or(Error::Schema)?;
        let json = s.strip_suffix(',').unwrap_or(s);
        let a = Artifact::parse(json, context, limits)?;
        let previous = inventory.facts.last();
        if previous.is_some_and(|p| !model::ordered(&p.path, &a.path)) {
            return Err(Error::Inventory);
        }
        inventory.push(a)?;
        // Reconstruct the exact canonical array punctuation from the next line
        // after parsing below; accepted text is checked against its own renderer.
    }
    let mut canonical = sha256::Sha256::new();
    canonical.update(&header_line);
    for (i, a) in inventory.facts.iter().enumerate() {
        canonical.update(
            format!(
                "{}{}\n",
                a.json(),
                if i + 1 == inventory.facts.len() {
                    ""
                } else {
                    ","
                }
            )
            .as_bytes(),
        );
    }
    canonical.update(footer(inventory.summary(context)).as_bytes());
    let expected_manifest = Artifact {
        component: Component::Db2,
        path: MANIFEST.to_owned(),
        bytes: meta.len(),
        sha256: canonical.finish(),
    };
    root.hash(
        &expected_manifest,
        Limits {
            file_bytes: limits.manifest_bytes,
            ..limits
        },
    )?;
    for a in &inventory.facts {
        if root.small(&receipt(a), limits)? != a.json().as_bytes() {
            return Err(Error::Inventory);
        }
    }
    check_seals(&root, context, &inventory, limits)?;
    if let Some(admit) = admit {
        admit(&inventory.facts)?;
    }
    if let Some(visitor) = partial_visitor {
        visitor(&partial_sections)?;
    }
    for a in &inventory.facts {
        root.hash(a, limits)?;
    }
    tree(&root, &inventory, context, limits, true)?;
    if let Some(inspect) = inspect {
        for artifact in &inventory.facts {
            let mut file = root.open_file(&artifact.path, limits)?;
            let before = Identity::of(&file.metadata().map_err(|_| Error::Io)?);
            // Hash the same read-only descriptor handed to the callback. A reopened
            // pathname alone would not bind callback bytes against replacement.
            let actual =
                crate::integrity::reader(&mut file, artifact.bytes).map_err(|_| Error::Io)?;
            if actual != artifact.sha256 {
                return Err(Error::Digest);
            }
            use std::io::{Seek, SeekFrom};
            file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
            inspect(artifact, &mut file)?;
            if before != Identity::of(&file.metadata().map_err(|_| Error::Changed)?) {
                return Err(Error::Changed);
            }
            root.hash(artifact, limits)?;
        }
        // Callback work can take much longer than ordinary inventory checks.
        // Close the namespace and state facts again before reporting success.
        for a in &inventory.facts {
            if root.small(&receipt(a), limits)? != a.json().as_bytes() {
                return Err(Error::Inventory);
            }
        }
        check_seals(&root, context, &inventory, limits)?;
        root.hash(
            &expected_manifest,
            Limits {
                file_bytes: limits.manifest_bytes,
                ..limits
            },
        )?;
        tree(&root, &inventory, context, limits, true)?;
    }
    if self::context(&root, limits)? != context {
        return Err(Error::Context);
    }
    root.named_file(".state/lock", &_lock, limits)?;
    if Identity::of(&before) != Identity::of(&root.held.metadata().map_err(|_| Error::Changed)?) {
        return Err(Error::Changed);
    }
    Ok(inventory.summary(context))
}
