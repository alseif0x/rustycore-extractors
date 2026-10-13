// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded unsigned Windows producer binding.
//! Pure bytes verification. The caller must supply a payload and its RVA read
//! independently from the running main module, and retain a protected file
//! lease through reading and verification. This is not publisher authentication.

use forever_casc::integrity;
use std::io::Read;

pub const MARKER: [u8; 32] = *b"RCE-FOREVER-PE-SEAL-V1\0\0\0\0\0\0\0\0\0\0";
pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
const MAX_WORK: usize = 65_536;
const NAME: [u8; 8] = *b".rceid\0\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(&'static str);
impl Error {
    pub const fn code(self) -> &'static str {
        self.0
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}

fn require(value: bool, code: &'static str) -> Result<(), Error> {
    if value { Ok(()) } else { Err(Error(code)) }
}
fn integer(data: &[u8], at: usize, width: usize) -> Result<u64, Error> {
    let end = at.checked_add(width).ok_or(Error("pe_bounds"))?;
    let bytes = data.get(at..end).ok_or(Error("pe_bounds"))?;
    Ok(bytes
        .iter()
        .enumerate()
        .fold(0, |v, (i, b)| v | (u64::from(*b) << (i * 8))))
}
fn aligned(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

#[derive(Clone, Copy, Default)]
struct Section {
    name: [u8; 8],
    rva: u64,
    virtual_size: u64,
    raw: usize,
    size: u64,
    flags: u64,
}

/// Disk digest offset and actual loaded payload start. They are different
/// address spaces; neither is interchangeable with the measured module RVA.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub digest_offset: usize,
    pub payload_rva: u32,
}

struct Image<'a> {
    data: &'a [u8],
    sections: &'a [Section],
    seal: Section,
}
impl Image<'_> {
    fn mapped(&self, rva: u64, size: u64, raw: bool) -> Result<Section, Error> {
        require(rva > 0 && size > 0, "pe_rva")?;
        let end = rva.checked_add(size).ok_or(Error("pe_rva_bounds"))?;
        let mut found = None;
        for &section in self.sections {
            let len = if raw {
                section.virtual_size.min(section.size)
            } else {
                section.virtual_size
            };
            if section.rva <= rva && end <= section.rva + len {
                require(found.is_none(), "pe_rva_bounds")?;
                found = Some(section);
            }
        }
        found.ok_or(Error("pe_rva_bounds"))
    }
    fn offset(&self, rva: u64, size: u64) -> Result<usize, Error> {
        let s = self.mapped(rva, size, true)?;
        Ok(s.raw + (rva - s.rva) as usize)
    }
    fn not_seal(&self, rva: u64, size: u64) -> Result<(), Error> {
        let end = rva
            .checked_add(size)
            .ok_or(Error("pe_loader_writes_seal"))?;
        require(
            end <= self.seal.rva || rva >= self.seal.rva + self.seal.virtual_size,
            "pe_loader_writes_seal",
        )
    }
    fn not_control(&self, rva: u64, size: u64, control: (u64, u64)) -> Result<(), Error> {
        if control.0 == 0 {
            return Ok(());
        }
        let end = rva
            .checked_add(size)
            .ok_or(Error("pe_loader_control_write"))?;
        require(
            end <= control.0 || rva >= control.0 + control.1,
            "pe_loader_control_write",
        )
    }
    fn relocations(&self, rva: u64, size: u64, imports: (u64, u64)) -> Result<(), Error> {
        if rva == 0 {
            return Ok(());
        }
        require(size <= 1024 * 1024, "pe_relocation_limit")?;
        let mut p = self.offset(rva, size)?;
        let end = p + size as usize;
        let mut work = 0;
        while p < end {
            require(p + 8 <= end, "pe_relocation_block")?;
            let page = integer(self.data, p, 4)?;
            let length = integer(self.data, p + 4, 4)? as usize;
            require(
                page % 4096 == 0 && length >= 8 && length.is_multiple_of(2) && length <= end - p,
                "pe_relocation_block",
            )?;
            for at in (p + 8..p + length).step_by(2) {
                work += 1;
                require(work <= MAX_WORK, "pe_relocation_limit")?;
                let entry = integer(self.data, at, 2)?;
                let kind = entry >> 12;
                if kind != 0 {
                    require(kind == 10, "pe_relocation_kind")?;
                    let target = page + (entry & 4095);
                    self.mapped(target, 8, false)?;
                    self.not_seal(target, 8)?;
                    self.not_control(target, 8, (rva, size))?;
                    self.not_control(target, 8, imports)?;
                }
            }
            p += length;
        }
        Ok(())
    }
    fn imports(&self, rva: u64, size: u64, relocations: (u64, u64)) -> Result<(), Error> {
        if rva == 0 {
            return Ok(());
        }
        require(size <= 1024 * 1024, "pe_import_limit")?;
        let mut p = self.offset(rva, size)?;
        let end = p + size as usize;
        let mut work = 0;
        loop {
            require(p + 20 <= end, "pe_import_descriptor")?;
            if self.data[p..p + 20].iter().all(|b| *b == 0) {
                break;
            }
            let target = integer(self.data, p + 16, 4)?;
            let lookup = match integer(self.data, p, 4)? {
                0 => target,
                other => other,
            };
            require(lookup != 0 && target != 0, "pe_import_descriptor")?;
            let mut index = 0_u64;
            loop {
                work += 1;
                require(work <= MAX_WORK, "pe_import_limit")?;
                let at = self.offset(lookup + index * 8, 8)?;
                let thunk = integer(self.data, at, 8)?;
                self.mapped(target + index * 8, 8, false)?;
                self.not_seal(target + index * 8, 8)?;
                self.not_control(target + index * 8, 8, relocations)?;
                self.not_control(target + index * 8, 8, (rva, size))?;
                if thunk == 0 {
                    break;
                }
                index += 1;
            }
            p += 20;
        }
        Ok(())
    }
    fn tls(&self, rva: u64, size: u64, base: u64) -> Result<(), Error> {
        if rva == 0 {
            return Ok(());
        }
        require(size == 40, "pe_tls_size")?;
        let p = self.offset(rva, 40)?;
        let index = integer(self.data, p + 16, 8)?
            .checked_sub(base)
            .ok_or(Error("pe_tls_index"))?;
        self.mapped(index, 4, false)?;
        self.not_seal(index, 4)?;
        let callbacks = integer(self.data, p + 24, 8)?;
        if callbacks != 0 {
            let callbacks = callbacks
                .checked_sub(base)
                .ok_or(Error("pe_tls_callbacks"))?;
            for index in 0..1024_u64 {
                let at = callbacks
                    .checked_add(index * 8)
                    .ok_or(Error("pe_tls_callbacks"))?;
                let callback = integer(self.data, self.offset(at, 8)?, 8)?;
                if callback == 0 {
                    return Ok(());
                }
                let code = callback
                    .checked_sub(base)
                    .ok_or(Error("pe_tls_callbacks"))?;
                require(
                    self.mapped(code, 1, false)?.flags & 0x2000_0000 != 0,
                    "pe_tls_callback_code",
                )?;
            }
            return Err(Error("pe_tls_callbacks_limit"));
        }
        Ok(())
    }
}

pub fn locate(data: &[u8]) -> Result<Layout, Error> {
    require((64..=MAX_FILE_BYTES).contains(&data.len()), "pe_limit")?;
    require(&data[..2] == b"MZ", "pe_dos")?;
    let pe = integer(data, 0x3c, 4)? as usize;
    require(
        pe >= 64 && pe <= data.len().saturating_sub(4) && data.get(pe..pe + 4) == Some(b"PE\0\0"),
        "pe_signature",
    )?;
    require(integer(data, pe + 4, 2)? == 0x8664, "pe_machine")?;
    let count = integer(data, pe + 6, 2)? as usize;
    require((1..=96).contains(&count), "pe_sections")?;
    require(integer(data, pe + 20, 2)? == 240, "pe_optional")?;
    let flags = integer(data, pe + 22, 2)?;
    require(
        flags & 2 != 0 && flags & (0x100 | 0x1000 | 0x2000) == 0,
        "pe_executable",
    )?;
    let optional = pe + 24;
    require(integer(data, optional, 2)? == 0x20b, "pe_magic")?;
    require(integer(data, optional + 108, 4)? == 16, "pe_directories")?;
    require(integer(data, optional + 68, 2)? == 3, "pe_console_only")?;
    require(integer(data, optional + 64, 4)? == 0, "pe_checksum_nonzero")?;
    let base = integer(data, optional + 24, 8)?;
    let sa = integer(data, optional + 32, 4)?;
    let fa = integer(data, optional + 36, 4)?;
    require(
        (4096..=65536).contains(&sa) && sa.is_power_of_two(),
        "pe_alignment",
    )?;
    require(
        (512..=sa).contains(&fa) && fa.is_power_of_two(),
        "pe_alignment",
    )?;
    let image_size = integer(data, optional + 56, 4)?;
    let headers = integer(data, optional + 60, 4)?;
    let table = optional + 240;
    require(
        table + count * 40 <= headers as usize && headers <= data.len() as u64 && headers % fa == 0,
        "pe_headers",
    )?;
    require(
        image_size <= MAX_FILE_BYTES as u64 && image_size % sa == 0,
        "pe_image_size",
    )?;
    let mut sections = [Section::default(); 96];
    let mut raw_end = headers;
    let mut virtual_end = aligned(headers, sa);
    for (index, section) in sections[..count].iter_mut().enumerate() {
        let p = table + index * 40;
        section.name.copy_from_slice(&data[p..p + 8]);
        section.virtual_size = integer(data, p + 8, 4)?;
        section.rva = integer(data, p + 12, 4)?;
        section.size = integer(data, p + 16, 4)?;
        section.raw = integer(data, p + 20, 4)? as usize;
        section.flags = integer(data, p + 36, 4)?;
        require(
            data[p + 24..p + 36].iter().all(|b| *b == 0),
            "pe_coff_relocations",
        )?;
        require(
            section.virtual_size > 0 && section.rva >= virtual_end && section.rva % sa == 0,
            "pe_virtual_overlap",
        )?;
        virtual_end = aligned(section.rva + section.virtual_size.max(section.size), sa);
        require(virtual_end <= image_size, "pe_virtual_bounds")?;
        if section.size != 0 {
            require(
                section.size % fa == 0
                    && (section.raw as u64).is_multiple_of(fa)
                    && section.raw as u64 >= raw_end,
                "pe_raw_overlap",
            )?;
            require(
                section.raw as u64 + section.size <= data.len() as u64,
                "pe_raw_bounds",
            )?;
            raw_end = section.raw as u64 + section.size;
        } else {
            require(section.raw == 0, "pe_raw_zero")?;
        }
    }
    let sections = &sections[..count];
    let mut seals = sections.iter().copied().filter(|s| s.name == NAME);
    let seal = seals.next().ok_or(Error("pe_seal_count"))?;
    require(seals.next().is_none(), "pe_seal_count")?;
    require(
        seal.virtual_size == 64 && seal.size >= 64 && seal.raw != 0,
        "pe_seal_initialized",
    )?;
    require(seal.flags == 0x4000_0040, "pe_seal_flags")?;
    require(data[seal.raw..seal.raw + 32] == MARKER, "pe_seal_marker")?;
    require(
        data[seal.raw + 64..seal.raw + seal.size as usize]
            .iter()
            .all(|b| *b == 0),
        "pe_seal_padding",
    )?;
    let image = Image {
        data,
        sections,
        seal,
    };
    let mut directories = [(0, 0); 16];
    for (index, directory) in directories.iter_mut().enumerate() {
        let p = optional + 112 + index * 8;
        let rva = integer(data, p, 4)?;
        let size = integer(data, p + 4, 4)?;
        require((rva != 0) == (size != 0), "pe_directory_pair")?;
        require(
            rva == 0 || ![4, 7, 8, 10, 11, 13, 14, 15].contains(&index),
            "pe_unsupported_directory",
        )?;
        if rva != 0 {
            image.offset(rva, size)?;
            image.not_seal(rva, size)?;
        }
        *directory = (rva, size);
    }
    image.relocations(directories[5].0, directories[5].1, directories[1])?;
    image.imports(directories[1].0, directories[1].1, directories[5])?;
    image.tls(directories[9].0, directories[9].1, base)?;
    Ok(Layout {
        digest_offset: seal.raw + 32,
        payload_rva: seal.rva as u32,
    })
}

/// Complete-file hash with exactly one 32-byte digest field replaced by zeros.
/// The layout must come from `locate` for these same immutable bytes.
fn canonical_digest(data: &[u8], layout: Layout) -> Result<[u8; 32], Error> {
    let zero = [0; 32];
    let input = data[..layout.digest_offset]
        .chain(zero.as_slice())
        .chain(&data[layout.digest_offset + 32..]);
    integrity::reader(input, data.len() as u64).map_err(|_| Error("pe_digest_read"))
}

pub fn verify_pe(
    data: &[u8],
    loaded_payload: &[u8],
    measured_payload_rva: u32,
) -> Result<[u8; 32], Error> {
    require(loaded_payload.len() == 64, "pe_loaded_payload")?;
    require(
        loaded_payload[..32] == MARKER && loaded_payload[32..].iter().any(|b| *b != 0),
        "pe_loaded_seal",
    )?;
    let layout = locate(data)?;
    require(measured_payload_rva == layout.payload_rva, "pe_loaded_rva")?;
    let stored = &data[layout.digest_offset..layout.digest_offset + 32];
    require(stored == &loaded_payload[32..], "pe_loaded_mismatch")?;
    require(
        canonical_digest(data, layout)?.as_slice() == stored,
        "pe_digest_mismatch",
    )?;
    Ok(integrity::digest(data))
}

/// Bind the actual main module's volatile payload and measured RVA to the
/// complete guarded candidate file. There is no unsealed/pathname fallback.
#[cfg(windows)]
pub fn loaded_identity() -> Result<[u8; 32], Error> {
    loaded_identity_evidence().map(|identity| identity.file_sha256())
}

#[cfg(windows)]
pub struct VerifiedProducer {
    file_sha256: [u8; 32],
    payload_rva: u32,
}
#[cfg(windows)]
impl VerifiedProducer {
    pub const fn file_sha256(&self) -> [u8; 32] {
        self.file_sha256
    }
    pub const fn payload_rva(&self) -> u32 {
        self.payload_rva
    }
}

/// Same complete binding, retaining the RVA actually measured from the loaded
/// module for explicit QA. This value is never taken from the candidate header.
#[cfg(windows)]
pub fn loaded_identity_evidence() -> Result<VerifiedProducer, Error> {
    use forever_platform::{path::Limits, windows::producer::LoadedProducer};
    let source = LoadedProducer::capture(Limits::default()).map_err(|e| Error(e.code()))?;
    let bytes = source
        .read_all(MAX_FILE_BYTES)
        .map_err(|e| Error(e.code()))?;
    let identity = verify_pe(&bytes, &source.payload(), source.payload_rva());
    // Always recheck the loaded bytes/RVA and retained file lease, including
    // failed PE verification, before returning a result to the caller.
    source.verify_unchanged().map_err(|e| Error(e.code()))?;
    identity.map(|file_sha256| VerifiedProducer {
        file_sha256,
        payload_rva: source.payload_rva(),
    })
}

#[cfg(test)]
mod tests;
