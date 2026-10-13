// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded, offline, read-only CASC store adapter.
//! Local WoW archive backend for the admitted Forever builds, without native IO.
//!
//! This backend supports the reviewed local v7 index layouts, ENCODING CKey
//! projection, explicit WoW ROOT layouts and complete unencrypted N/Z BLTE.
//! [`LocalStore::open_with_keys`] separately admits supplied-key Salsa20 E/N/Z;
//! the default still rejects E. F, loose files, remote files and unresolved sizes
//! do not silently fall back.
//! The native backend and CLI defaults are unchanged. Build/CDN identifiers and
//! tags are retained as metadata; no CDN request, key discovery or listfile is
//! performed. Files are opened read-only. Diagnostics contain no client paths.
//!
//! All index rows and complete EKeys survive in owned arrays with sorted ordinal
//! indexes. ROOT is resolved once by central CKey presence; missing archive bytes
//! never select a later ROOT row. Alternative encodings of that SAME selected
//! CKey may be tried within a caller limit, and success requires exact size and
//! complete content MD5. Prelude/index checks still authenticate only an EKey's
//! nine-byte prefix, not its remaining bytes. No read returns an incomplete Vec.
//!
//! Resident and scratch payload reservations are bounded separately and jointly
//! before each phase. These are logical allocation budgets, not process RSS:
//! requested Vec/String/Path storage and conservative parser conversion overlap
//! are counted; allocator bookkeeping, fixed stack frames, standard-library IO
//! internals and caller-retained prior read results are outside that accounting.
//! Checked exact reservations avoid geometric growth of metadata vectors. Metadata conversion releases decoded buffers; no struct contains
//! references into its own allocation. Defaults admit the observed 160.6 MB
//! ENCODING, but exceeding any bound is an explicit policy error, not truncation.
//!
//! Logical-name reads are a separate opt-in capability. They use a conservative
//! stable ROOT-name index, never OS paths, textual FDIDs or fallback routes.
//! Opening a store and the existing acquisition profile do not enable names.

mod build_info;
pub mod framing;
mod io;
mod owned;
pub mod routes;

use crate::{Locale, build_config, encoding, local_index, root};
use owned::{Central, OwnedRoot};
use std::{
    cell::Cell,
    mem::size_of,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedBuild {
    pub product: &'static str,
    pub version: &'static str,
    pub build: u32,
    pub build_key: [u8; 16],
    pub cdn_key: [u8; 16],
    pub tags: String,
    pub config_build_uid: Option<String>,
    pub config_build_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_resident_bytes: usize,
    pub max_scratch_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_build_info_bytes: u64,
    pub max_build_info_rows: usize,
    pub max_path_bytes: usize,
    pub config: build_config::Limits,
    pub max_index_files: usize,
    pub max_directory_entries: usize,
    pub index: local_index::Limits,
    pub encoding: encoding::Limits,
    pub root: root::Limits,
    pub max_archive_span_bytes: u64,
    pub max_decoded_file_bytes: u64,
    pub max_candidate_attempts: usize,
    pub max_blte_frames: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_resident_bytes: 768 * 1024 * 1024,
            max_scratch_bytes: 512 * 1024 * 1024,
            max_memory_bytes: 1024 * 1024 * 1024,
            max_build_info_bytes: 256 * 1024,
            max_build_info_rows: 4096,
            max_path_bytes: 4096,
            config: build_config::Limits {
                max_bytes: 256 * 1024,
                max_lines: 8192,
                max_keys: 4096,
            },
            max_index_files: 4096,
            max_directory_entries: 65536,
            index: local_index::Limits {
                max_bytes: 64 * 1024 * 1024,
                max_entries: 2_000_000,
                max_pages: 131072,
            },
            encoding: encoding::Limits {
                max_bytes: 256 * 1024 * 1024,
                max_pages: 65536,
                max_page_bytes: 64 * 1024,
                max_espec_bytes: 16 * 1024 * 1024,
                max_tail_bytes: 128 * 1024 * 1024,
                max_entries: 4_000_000,
                max_encoded_keys: 8_000_000,
            },
            root: root::Limits {
                max_bytes: 256 * 1024 * 1024,
                max_header_bytes: 4096,
                max_blocks: 65536,
                max_entries: 4_000_000,
            },
            max_archive_span_bytes: 256 * 1024 * 1024,
            max_decoded_file_bytes: 64 * 1024 * 1024,
            max_candidate_attempts: 64,
            max_blte_frames: 65536,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    pub locale: Locale,
    pub index_layouts: [local_index::Layout; 16],
    pub root_layout: root::Layout,
    pub limits: Limits,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CandidateFailures {
    pub missing: u32,
    pub incomplete: u32,
    pub encrypted: u32,
    pub corrupt: u32,
    pub unsupported: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Input,
    InputChanged,
    BuildMetadata,
    UnsupportedClient,
    AmbiguousClient,
    Limit,
    IndexSet,
    UnknownSize,
    MissingFile,
    Integrity,
    NoLocalEncoding(CandidateFailures),
    Config(build_config::Error),
    Index(local_index::Error),
    Encoding(encoding::Error),
    Root(root::Error),
    NamesDisabled,
    NamesAlreadyEnabled,
    Name(root::name::Error),
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Input => "local_store_input_io",
            Self::InputChanged => "local_store_input_changed",
            Self::BuildMetadata => "invalid_build_metadata",
            Self::UnsupportedClient => "unsupported_client",
            Self::AmbiguousClient => "ambiguous_active_client",
            Self::Limit => "local_store_policy_limit",
            Self::IndexSet => "local_store_incomplete_index_set",
            Self::UnknownSize => "local_store_unknown_content_size",
            Self::MissingFile => "file_unavailable",
            Self::Integrity => "local_store_content_integrity",
            Self::NoLocalEncoding(_) => "local_store_no_verified_encoding",
            Self::Config(error) => error.code(),
            Self::Index(error) => error.code(),
            Self::Encoding(error) => error.code(),
            Self::Root(error) => error.code(),
            Self::NamesDisabled => "local_store_names_disabled",
            Self::NamesAlreadyEnabled => "local_store_names_already_enabled",
            Self::Name(error) => error.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Debug)]
pub struct VerifiedFile {
    pub file_data_id: u32,
    pub content_key: [u8; 16],
    pub bytes: Vec<u8>,
    pub candidate_attempts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub index_entries: usize,
    pub encoding_entries: usize,
    pub encoded_keys: usize,
    pub root_entries: usize,
    pub resident_bytes: usize,
}

/// Separate from the FDID acquisition profile; no CLI/provenance opt-in implied.
pub const NAME_CAPABILITY: &str = "linux-ascii-stable-root-names-v1";

struct EnabledNames {
    index: root::name::NameIndex,
    requested: root::name::Limits,
    effective: root::name::Limits,
}

pub struct LocalStore {
    build: AdmittedBuild,
    content_identity: [u8; 32],
    data_dir: PathBuf,
    locale: Locale,
    limits: Limits,
    indexes: Vec<local_index::Entry>,
    index_order: Vec<u32>,
    central: Central,
    root: OwnedRoot,
    guards: Vec<io::Guard>,
    stats: Stats,
    names: Option<EnabledNames>,
    keys: Option<crate::keys::Keyring>,
}

impl LocalStore {
    /// Versioned logical collection identity, not physical availability or a
    /// complete archive snapshot. Build/config and central content keys bind
    /// ROOT/ENCODING; each acquired file still requires its full content MD5.
    /// Paths, TACT encryption keys and archive bytes are not returned.
    pub const fn content_identity(&self) -> [u8; 32] {
        self.content_identity
    }
    pub fn build(&self) -> &AdmittedBuild {
        &self.build
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }

    pub fn open(client: &Path, options: OpenOptions) -> Result<Self, Error> {
        Self::open_inner(client, options, None)
    }

    /// Explicit opt-in Salsa20/128-bit keys. Keys are consumed and kept private;
    /// no file is discovered or read for them. They affect physical availability,
    /// not logical collection identity. Every returned file still passes its
    /// mandatory complete CKey MD5. Actual keyring heap capacity counts against
    /// resident limits, and encrypted-frame scratch is reserved conservatively.
    /// Existing acquisition profiles and product-v1 CLI do not call this API.
    pub fn open_with_keys(
        client: &Path,
        options: OpenOptions,
        keys: crate::keys::Keyring,
    ) -> Result<Self, Error> {
        Self::open_inner(client, options, Some(keys))
    }

    fn open_inner(
        client: &Path,
        options: OpenOptions,
        keys: Option<crate::keys::Keyring>,
    ) -> Result<Self, Error> {
        let limits = options.limits;
        if add(client.as_os_str().len(), 48)? > limits.max_path_bytes {
            return Err(Error::Limit);
        }
        let mut budget = Budget::new(limits);
        if let Some(keys) = &keys {
            budget.retain(keys.resident_bytes())?;
        }
        // Conservative storage for the resident data path plus the bounded
        // transient paths used throughout opening and reading.
        budget.retain(limits.max_path_bytes.checked_mul(4).ok_or(Error::Limit)?)?;
        // Windows acquisition holds only one complete ancestor lease at a
        // time. Reserve its bounded path/prefix/handle metadata and transient
        // Win32 path conversion separately from the caller's PathBuf storage.
        // This is a conservative logical admission allowance, not an RSS cap.
        #[cfg(windows)]
        budget.retain(io::windows_lease_reservation())?;
        let mut guards = Vec::new();
        let build_guard = io::Guard::capture(&client.join(".build.info"))?;
        let build_bytes = io::whole(
            &client.join(".build.info"),
            limits.max_build_info_bytes,
            &budget,
        )?;
        budget.phase(add(build_bytes.len(), build_bytes.len())?)?;
        let mut build = build_info::parse(&build_bytes, limits.max_build_info_rows)?;
        budget.retain(build.tags.capacity())?;
        budget.retain(build_bytes.capacity())?;
        let build_guard = build_guard.with_bytes(build_bytes);
        budget.phase(0)?;
        build_guard.check()?;
        io::push_guard(&mut guards, build_guard, &mut budget)?;
        let data_dir = client.join("Data").join("data");
        io::push_guard(&mut guards, io::directory_guard(&data_dir)?, &mut budget)?;
        let config_path = io::config_path(client, build.build_key);
        let config_guard = io::Guard::capture(&config_path)?;
        let config_bytes = io::whole(&config_path, limits.config.max_bytes, &budget)?;
        budget.phase(add(
            config_bytes.len(),
            array_bytes::<build_config::AuxiliaryReference>(
                limits.config.max_keys.checked_mul(2).ok_or(Error::Limit)?,
            )?,
        )?)?;
        let config = build_config::parse(&config_bytes, build.build_key, limits.config)
            .map_err(Error::Config)?;
        let reference_count = add(
            config
                .auxiliary
                .iter()
                .filter(|r| r.content_key.is_some())
                .count(),
            2,
        )?;
        budget.retain(array_bytes::<build_config::ContentReference>(
            reference_count,
        )?)?;
        let mut references = owned::config_references(&config)?;
        build.config_build_uid = owned_string(config.build_uid, &mut budget)?;
        build.config_build_name = owned_string(config.build_name, &mut budget)?;
        let encoding_reference = config.encoding;
        let root_reference = config.root;
        let mut identity = crate::sha256::Sha256::new();
        identity.update(b"forever-logical-collection-v1\0");
        identity.update(&build.build.to_le_bytes());
        identity.update(&build.build_key);
        identity.update(&build.cdn_key);
        identity.update(options.locale.name().as_bytes());
        identity.update(&encoding_reference.content_key);
        identity.update(&root_reference.content_key);
        let content_identity = identity.finish();
        drop(config);
        drop(config_bytes);
        budget.phase(0)?;
        config_guard.check()?;
        io::push_guard(&mut guards, config_guard, &mut budget)?;
        let indexes = io::load_indexes(
            &data_dir,
            options.index_layouts,
            limits,
            &mut budget,
            &mut guards,
        )?;
        let index_order = owned::sorted_order(indexes.len(), &mut budget, |a, b| {
            indexes[a].key_prefix.cmp(&indexes[b].key_prefix)
        })?;
        let (encoding_bytes, _) = io::Reader {
            dir: &data_dir,
            indexes: &indexes,
            order: &index_order,
            limits,
            budget: &budget,
            keys: keys.as_ref(),
        }
        .read_reference(
            encoding_reference.content_key,
            encoding_reference.content_size,
            encoding_reference.encoded_key.into_iter(),
            limits.encoding.max_bytes,
        )?;
        let parser_rows = limits.encoding.max_entries.min(encoding_bytes.len() / 38);
        budget.phase(add(
            encoding_bytes.len(),
            array_bytes::<encoding::Entry<'_>>(parser_rows)?,
        )?)?;
        let routes = encoding::parse_ckey_routes(
            &encoding_bytes,
            encoding_reference.content_key,
            limits.encoding,
        )
        .map_err(Error::Encoding)?;
        let encoding_entries = routes.entries.len();
        let encoded_keys = routes
            .entries
            .iter()
            .try_fold(0_usize, |n, row| add(n, row.encoded_keys.len()))?;
        let central = Central::from_routes(routes, &references, &mut budget)?;
        drop(encoding_bytes);
        budget.phase(0)?;
        budget.release(array_bytes::<build_config::ContentReference>(
            reference_count,
        )?);
        references.clear();
        drop(references);
        let (root_bytes, _) = io::Reader {
            dir: &data_dir,
            indexes: &indexes,
            order: &index_order,
            limits,
            budget: &budget,
            keys: keys.as_ref(),
        }
        .read_reference(
            root_reference.content_key,
            central.content_size(root_reference.content_key),
            central.encoded_keys(root_reference.content_key),
            limits.root.max_bytes,
        )?;
        let parser_rows = limits.root.max_entries.min(root_bytes.len() / 20);
        let parser_blocks = limits.root.max_blocks.min(root_bytes.len() / 12);
        budget.phase(add(
            root_bytes.len(),
            add(
                array_bytes::<root::Entry>(parser_rows)?,
                array_bytes::<root::Block>(parser_blocks)?,
            )?,
        )?)?;
        let parsed = root::parse_root(
            &root_bytes,
            root_reference.content_key,
            options.root_layout,
            limits.root,
        )
        .map_err(Error::Root)?;
        let owned_root = OwnedRoot::from_root(parsed, &mut budget)?;
        drop(root_bytes);
        budget.phase(0)?;
        io::check_guards(&guards)?;
        let stats = Stats {
            index_entries: indexes.len(),
            encoding_entries,
            encoded_keys,
            root_entries: owned_root.entries.len(),
            resident_bytes: budget.resident,
        };
        Ok(Self {
            build,
            content_identity,
            data_dir,
            locale: options.locale,
            limits,
            indexes,
            index_order,
            central,
            root: owned_root,
            guards,
            stats,
            names: None,
            keys,
        })
    }

    pub fn names_enabled(&self) -> bool {
        self.names.is_some()
    }

    pub fn names_capability(&self) -> Option<&'static str> {
        self.names.as_ref().map(|_| NAME_CAPABILITY)
    }

    pub fn names_stats(&self) -> Option<root::name::Stats> {
        self.names.as_ref().map(|names| names.index.stats())
    }

    /// Actual index bounds, including a possibly reduced memory ceiling imposed
    /// by the store's resident, scratch and combined budgets.
    pub fn names_limits(&self) -> Option<root::name::Limits> {
        self.names.as_ref().map(|names| names.effective)
    }

    /// Construct an owned name index lazily, after open released decoded ROOT.
    ///
    /// Invalid options fail before any guard I/O. Failed construction exposes no
    /// partial cache and does not change FDID selection or resident accounting.
    /// Repeating the exact requested options is idempotent (with fresh guards);
    /// changing them after success is an explicit error, never a hidden rebuild.
    ///
    /// Before index reservations, its peak-memory ceiling is reduced to the
    /// minimum of the requested bound, store scratch allowance, free combined
    /// allowance and free resident allowance. Charging the whole construction
    /// peak against free resident space is conservative: an index whose final
    /// allocation fits but whose construction peak does not may be rejected.
    /// Only its actual retained capacity is charged after successful completion.
    pub fn enable_names(
        &mut self,
        requested: root::name::Limits,
    ) -> Result<root::name::Stats, Error> {
        requested.validate().map_err(Error::Name)?;
        if let Some(names) = &self.names {
            if names.requested != requested {
                return Err(Error::NamesAlreadyEnabled);
            }
            io::check_guards(&self.guards)?;
            let stats = names.index.stats();
            io::check_guards(&self.guards)?;
            return Ok(stats);
        }
        let available_resident = self
            .limits
            .max_resident_bytes
            .checked_sub(self.stats.resident_bytes)
            .ok_or(Error::Limit)?;
        let available_combined = self
            .limits
            .max_memory_bytes
            .checked_sub(self.stats.resident_bytes)
            .ok_or(Error::Limit)?;
        let effective = root::name::Limits {
            max_memory_bytes: requested
                .max_memory_bytes
                .min(self.limits.max_scratch_bytes)
                .min(available_resident)
                .min(available_combined),
            ..requested
        };
        let mut budget = Budget {
            limits: self.limits,
            resident: self.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        // NameIndex checks every reserve against this effective bound and its
        // actual resulting capacities. Reserve the whole permitted phase before
        // construction, rather than discovering its peak after allocation.
        budget.phase(effective.max_memory_bytes)?;
        io::check_guards(&self.guards)?;
        let result = root::name::NameIndex::build(
            self.root.blocks(),
            &self.root.entries,
            self.locale,
            |key| self.central.contains(key),
            effective,
        );
        // Mutation wins over a construction error and remains the unwrapped
        // InputChanged variant recognized by the existing Session poison gate.
        io::check_guards(&self.guards)?;
        let index = result.map_err(Error::Name)?;
        budget.phase(index.peak_bytes())?;
        budget.phase(0)?;
        budget.retain(index.resident_bytes())?;
        let stats = index.stats();
        self.names = Some(EnabledNames {
            index,
            requested,
            effective,
        });
        self.stats.resident_bytes = budget.resident;
        Ok(stats)
    }

    pub fn read_name(&mut self, name: &[u8]) -> Result<VerifiedFile, Error> {
        self.read_name_with_limit(name, self.limits.max_decoded_file_bytes)
    }

    /// Read a complete file selected by the enabled stable ROOT-name index.
    /// Hash normalization is lexical and does not access an OS path. FDID zero
    /// and distinct FDID aliases are retained; the selected node's first CKey is
    /// read directly, never re-resolved through the FDID API. Failure of local
    /// bytes cannot select a later ROOT CKey, another locale/backend or network.
    pub fn read_name_with_limit(
        &mut self,
        name: &[u8],
        max_decoded_bytes: u64,
    ) -> Result<VerifiedFile, Error> {
        self.read_name_with_external_budget(
            name,
            max_decoded_bytes,
            0,
            self.limits.max_memory_bytes,
        )
    }

    // Shared in-memory consumers retain earlier verified inputs and outputs.
    // Include those capacities in every archive/decode phase before reserving.
    pub(crate) fn read_name_with_external_budget(
        &mut self,
        name: &[u8],
        max_decoded_bytes: u64,
        external_live_bytes: usize,
        combined_cap: usize,
    ) -> Result<VerifiedFile, Error> {
        let names = self.names.as_ref().ok_or(Error::NamesDisabled)?;
        io::check_guards(&self.guards)?;
        let selected = names.index.resolve(name);
        io::check_guards(&self.guards)?;
        let selected = selected.map_err(Error::Name)?.ok_or(Error::MissingFile)?;
        self.read_selected_ckey(
            selected.file_data_id,
            selected.content_key,
            max_decoded_bytes,
            external_live_bytes,
            combined_cap,
        )
    }

    /// Read one complete file using the store's configured per-file ceiling.
    pub fn read_file_data_id(&mut self, file_data_id: u32) -> Result<VerifiedFile, Error> {
        self.read_file_data_id_with_limit(file_data_id, self.limits.max_decoded_file_bytes)
    }

    /// Tighten the decoded-byte ceiling for this read. A request cannot enlarge
    /// the store policy. The returned Vec is caller-owned; keeping several prior
    /// results alive is outside this store's resident/scratch accounting.
    pub fn read_file_data_id_with_limit(
        &mut self,
        file_data_id: u32,
        max_decoded_bytes: u64,
    ) -> Result<VerifiedFile, Error> {
        self.read_file_data_id_with_external_budget(
            file_data_id,
            max_decoded_bytes,
            0,
            self.limits.max_memory_bytes,
        )
    }

    pub(crate) fn read_file_data_id_with_external_budget(
        &mut self,
        file_data_id: u32,
        max_decoded_bytes: u64,
        external_live_bytes: usize,
        combined_cap: usize,
    ) -> Result<VerifiedFile, Error> {
        if file_data_id == 0 || file_data_id == u32::MAX {
            return Err(Error::MissingFile);
        }
        io::check_guards(&self.guards)?;
        let entry = self
            .root
            .resolve(file_data_id, self.locale, |key| self.central.contains(key))
            .ok_or(Error::MissingFile)?;
        self.read_selected_ckey(
            file_data_id,
            entry.content_key,
            max_decoded_bytes,
            external_live_bytes,
            combined_cap,
        )
    }

    /// Recheck the admitted input snapshot after conversion and before handing
    /// an in-memory result to a consumer. Does not acquire or decode new data.
    /// Recheck the captured metadata guards; not a full archive rehash/snapshot.
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        io::check_guards(&self.guards)
    }

    // Both public routes checked metadata guards before resolving their owned
    // tables. Share all archive I/O, same-CKey alternatives, complete MD5, size
    // caps and the post-read guard (including failed reads) here.
    fn read_selected_ckey(
        &self,
        file_data_id: u32,
        key: [u8; 16],
        max_decoded_bytes: u64,
        external_live_bytes: usize,
        combined_cap: usize,
    ) -> Result<VerifiedFile, Error> {
        let budget = Budget {
            limits: Limits {
                max_memory_bytes: self.limits.max_memory_bytes.min(combined_cap),
                ..self.limits
            },
            resident: add(self.stats.resident_bytes, external_live_bytes)?,
            scratch: Cell::new(0),
        };
        budget.phase(0)?;
        let result = io::Reader {
            dir: &self.data_dir,
            indexes: &self.indexes,
            order: &self.index_order,
            limits: self.limits,
            budget: &budget,
            keys: self.keys.as_ref(),
        }
        .read_reference(
            key,
            self.central.content_size(key),
            self.central.encoded_keys(key),
            max_decoded_bytes.min(self.limits.max_decoded_file_bytes),
        );
        io::check_guards(&self.guards)?;
        let (bytes, candidate_attempts) = result?;
        Ok(VerifiedFile {
            file_data_id,
            content_key: key,
            bytes,
            candidate_attempts,
        })
    }
}

pub(super) struct Budget {
    limits: Limits,
    resident: usize,
    scratch: Cell<usize>,
}
impl Budget {
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            resident: 0,
            scratch: Cell::new(0),
        }
    }
    fn phase(&self, scratch: usize) -> Result<(), Error> {
        if scratch > self.limits.max_scratch_bytes
            || add(self.resident, scratch)? > self.limits.max_memory_bytes
        {
            Err(Error::Limit)
        } else {
            self.scratch.set(scratch);
            Ok(())
        }
    }
    fn retain(&mut self, bytes: usize) -> Result<(), Error> {
        let resident = add(self.resident, bytes)?;
        if resident > self.limits.max_resident_bytes
            || add(resident, self.scratch.get())? > self.limits.max_memory_bytes
        {
            return Err(Error::Limit);
        }
        self.resident = resident;
        Ok(())
    }
    fn release(&mut self, bytes: usize) {
        self.resident -= bytes;
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn array_bytes<T>(count: usize) -> Result<usize, Error> {
    count.checked_mul(size_of::<T>()).ok_or(Error::Limit)
}
fn reserve<T>(vec: &mut Vec<T>, count: usize) -> Result<(), Error> {
    vec.try_reserve_exact(count).map_err(|_| Error::Limit)
}
fn owned_string(value: Option<&str>, budget: &mut Budget) -> Result<Option<String>, Error> {
    value
        .map(|value| {
            budget.retain(value.len())?;
            let mut output = String::new();
            output
                .try_reserve_exact(value.len())
                .map_err(|_| Error::Limit)?;
            output.push_str(value);
            Ok(output)
        })
        .transpose()
}

#[cfg(test)]
pub(crate) mod tests;
