// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original bounded held/named product disk anchors.
use crate::Error;
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};
const FLAGS: i32 = 0x20000 | 0x800;
const DIRECTORY: i32 = 0x10000;
pub(super) const MAX_PATH: usize = 4096;
const MAX_DEPTH: usize = 32;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    len: u64,
    mt: i64,
    mn: i64,
    ct: i64,
    cn: i64,
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
            mt: m.mtime(),
            mn: m.mtime_nsec(),
            ct: m.ctime(),
            cn: m.ctime_nsec(),
        }
    }
    fn object(self, b: Self) -> bool {
        self.dev == b.dev && self.ino == b.ino
    }
    fn directory(self, b: Self) -> bool {
        self.object(b) && self.mode == b.mode && self.uid == b.uid && self.gid == b.gid
    }
}
struct Node {
    file: File,
    path: PathBuf,
    id: Identity,
    mount: u64,
    directory: bool,
}
pub(super) struct Chain {
    nodes: Vec<Node>,
}
pub(super) fn anchor(file: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
}
pub(super) fn validate(path: &Path) -> Result<(), Error> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().contains(&0)
        || path.as_os_str().len() > MAX_PATH
        || path.components().count() > MAX_DEPTH
        || path
            .as_os_str()
            .as_encoded_bytes()
            .split(|b| *b == b'/')
            .any(|b| b == b"." || b == b".." || b == b".git")
    {
        return Err("product_disk_path".into());
    }
    Ok(())
}
fn open(path: &Path, directory: bool) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .custom_flags(FLAGS | if directory { DIRECTORY } else { 0 })
        .open(path)
        .map_err(|_| "product_disk_path".into())
}
fn no_git(directory: &File) -> Result<(), Error> {
    match fs::symlink_metadata(anchor(directory).join(".git")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("product_disk_path".into()),
    }
}
pub(super) fn mount(file: &File) -> Result<u64, Error> {
    let mut bytes = [0u8; 257];
    let mut source = File::open(format!("/proc/self/fdinfo/{}", file.as_raw_fd()))
        .map_err(|_| Error::from("product_disk_changed"))?;
    let mut n = 0;
    while n < bytes.len() {
        let got = source
            .read(&mut bytes[n..])
            .map_err(|_| Error::from("product_disk_changed"))?;
        if got == 0 {
            break;
        }
        n += got;
    }
    if n == 0 || n > 256 || bytes[n - 1] != b'\n' || !bytes[..n].is_ascii() {
        return Err("product_disk_changed".into());
    }
    let mut found = None;
    for line in bytes[..n].split(|b| *b == b'\n').filter(|b| !b.is_empty()) {
        let colon = line
            .iter()
            .position(|b| *b == b':')
            .ok_or_else(|| Error::from("product_disk_changed"))?;
        if &line[..colon] != b"mnt_id" {
            continue;
        }
        if found.is_some() {
            return Err("product_disk_changed".into());
        }
        let value = std::str::from_utf8(&line[colon + 1..])
            .map_err(|_| Error::from("product_disk_changed"))?
            .trim_start_matches([' ', '\t']);
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err("product_disk_changed".into());
        }
        let value = value
            .parse::<u64>()
            .map_err(|_| Error::from("product_disk_changed"))?;
        if value == 0 {
            return Err("product_disk_changed".into());
        }
        found = Some(value);
    }
    found.ok_or_else(|| "product_disk_changed".into())
}
impl Chain {
    pub(super) fn open(path: &Path, directory: bool) -> Result<Self, Error> {
        validate(path)?;
        let mut nodes: Vec<Node> = Vec::new();
        nodes
            .try_reserve_exact(path.components().count())
            .map_err(|_| Error::from("product_disk_allocation"))?;
        let mut named = PathBuf::new();
        for (at, part) in path.components().enumerate() {
            let is_dir = at + 1 != path.components().count() || directory;
            let resolved = match part {
                Component::RootDir => PathBuf::from("/"),
                Component::Normal(leaf) => anchor(
                    &nodes
                        .last()
                        .ok_or_else(|| Error::from("product_disk_path"))?
                        .file,
                )
                .join(leaf),
                _ => return Err("product_disk_path".into()),
            };
            named.push(part.as_os_str());
            let file = open(&resolved, is_dir)?;
            let metadata = file
                .metadata()
                .map_err(|_| Error::from("product_disk_io"))?;
            if if is_dir {
                !metadata.is_dir()
            } else {
                !metadata.is_file() || metadata.nlink() != 1
            } {
                return Err("product_disk_path".into());
            }
            if is_dir {
                no_git(&file)?;
            }
            let id = Identity::of(&metadata);
            let mount = mount(&file)?;
            nodes.push(Node {
                file,
                path: named.clone(),
                id,
                mount,
                directory: is_dir,
            });
        }
        if nodes.is_empty() {
            return Err("product_disk_path".into());
        }
        let chain = Self { nodes };
        chain.check()?;
        Ok(chain)
    }
    pub(super) fn check(&self) -> Result<(), Error> {
        let mut parent: Option<File> = None;
        for node in &self.nodes {
            let path = match &parent {
                Some(parent) => anchor(parent).join(
                    node.path
                        .file_name()
                        .ok_or_else(|| Error::from("product_disk_changed"))?,
                ),
                None => PathBuf::from("/"),
            };
            let reopened =
                open(&path, node.directory).map_err(|_| Error::from("product_disk_changed"))?;
            let held = Identity::of(
                &node
                    .file
                    .metadata()
                    .map_err(|_| Error::from("product_disk_changed"))?,
            );
            let named = fs::symlink_metadata(&node.path)
                .map_err(|_| Error::from("product_disk_changed"))?;
            let seen = Identity::of(
                &reopened
                    .metadata()
                    .map_err(|_| Error::from("product_disk_changed"))?,
            );
            let names = Identity::of(&named);
            if named.file_type().is_symlink()
                || mount(&node.file)? != node.mount
                || mount(&reopened)? != node.mount
                || if node.directory {
                    !node.id.directory(held)
                        || !node.id.directory(names)
                        || !node.id.directory(seen)
                } else {
                    node.id != held || node.id != names || node.id != seen
                }
            {
                return Err("product_disk_changed".into());
            }
            if node.directory {
                no_git(&reopened).map_err(|_| Error::from("product_disk_changed"))?;
            }
            parent = Some(reopened);
        }
        Ok(())
    }
    pub(super) fn file(&self) -> &File {
        &self.nodes.last().expect("nonempty checked chain").file
    }
    pub(super) fn path(&self) -> &Path {
        &self.nodes.last().expect("nonempty checked chain").path
    }
    pub(super) fn device(&self) -> u64 {
        self.nodes.last().expect("nonempty checked chain").id.dev
    }
    pub(super) fn mount(&self) -> u64 {
        self.nodes.last().expect("nonempty checked chain").mount
    }
    pub(super) fn private(&self) -> bool {
        self.nodes.last().expect("nonempty checked chain").id.mode & 0o7777 == 0o700
    }
    pub(super) fn uid(&self) -> u32 {
        self.nodes.last().expect("nonempty").id.uid
    }
    pub(super) fn contains_leaf(&self, other: &Self) -> bool {
        let id = other.nodes.last().expect("nonempty").id;
        self.nodes.iter().any(|n| n.id.object(id))
    }
}
pub(super) struct Namespace {
    held: File,
    id: Identity,
}
impl Namespace {
    pub(super) fn open() -> Result<Self, Error> {
        let held =
            File::open("/proc/self/ns/mnt").map_err(|_| Error::from("product_disk_unsupported"))?;
        let id = Identity::of(
            &held
                .metadata()
                .map_err(|_| Error::from("product_disk_unsupported"))?,
        );
        Ok(Self { held, id })
    }
    pub(super) fn check(&self) -> Result<(), Error> {
        let held = Identity::of(
            &self
                .held
                .metadata()
                .map_err(|_| Error::from("product_disk_changed"))?,
        );
        let named = Identity::of(
            &fs::metadata("/proc/self/ns/mnt").map_err(|_| Error::from("product_disk_changed"))?,
        );
        if !self.id.object(held) || !self.id.object(named) {
            return Err("product_disk_changed".into());
        }
        Ok(())
    }
}

pub(super) fn effective_uid() -> Result<u32, Error> {
    let mut file =
        File::open("/proc/self/status").map_err(|_| Error::from("product_disk_unsupported"))?;
    let mut bytes = [0u8; 16 * 1024 + 1];
    let mut used = 0;
    while used < bytes.len() {
        let n = file
            .read(&mut bytes[used..])
            .map_err(|_| Error::from("product_disk_unsupported"))?;
        if n == 0 {
            break;
        }
        used += n;
    }
    if used == bytes.len() {
        return Err("product_disk_unsupported".into());
    }
    let text =
        std::str::from_utf8(&bytes[..used]).map_err(|_| Error::from("product_disk_unsupported"))?;
    let mut found = None;
    for line in text.lines().filter_map(|line| line.strip_prefix("Uid:")) {
        if found.is_some() {
            return Err("product_disk_unsupported".into());
        }
        let mut fields = line.split_ascii_whitespace();
        let mut values = [0u32; 4];
        for value in &mut values {
            *value = fields
                .next()
                .ok_or_else(|| Error::from("product_disk_unsupported"))?
                .parse()
                .map_err(|_| Error::from("product_disk_unsupported"))?;
        }
        if fields.next().is_some() {
            return Err("product_disk_unsupported".into());
        }
        found = Some(values[1]);
    }
    found.ok_or_else(|| "product_disk_unsupported".into())
}
