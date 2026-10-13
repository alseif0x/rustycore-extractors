// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded unsigned Windows producer binding.
// Modified 2026-10-13: explicit closed observed MSVC v2 producer profile.
//! Pure bytes verification. The caller must supply a payload and its RVA read
//! independently from the running main module, and retain a protected file
//! lease through reading and verification. This is not publisher authentication.

use forever_casc::integrity;
use std::io::Read;

pub const MARKER: [u8; 32] = *b"RCE-FOREVER-PE-SEAL-V1\0\0\0\0\0\0\0\0\0\0";
pub const MARKER_V2: [u8; 32] = *b"RCE-FOREVER-PE-SEAL-V2\0\0\0\0\0\0\0\0\0\0";
pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
const MAX_WORK: usize = 65_536;
const NAME: [u8; 8] = *b".rceid\0\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    V1,
    MsvcV2,
}
impl Profile {
    pub const fn marker(self) -> [u8; 32] {
        match self {
            Self::V1 => MARKER,
            Self::MsvcV2 => MARKER_V2,
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Span {
    rva: u64,
    size: u64,
}
impl Span {
    fn overlaps(self, other: Self) -> bool {
        self.rva < other.rva + other.size && other.rva < self.rva + self.size
    }
}
#[derive(Clone, Copy)]
struct Fixup {
    target: u64,
    owner: Option<Span>,
    seen: bool,
}
const MAX_CONTROLS: usize = MAX_WORK + 32;
const MAX_TYPED_FIXUPS: usize = 9 + 6 + 4 + 1024;

fn push_bounded<T>(values: &mut Vec<T>, value: T, limit: usize) -> Result<(), Error> {
    require(values.len() < limit, "pe_v2_control_limit")?;
    if values.len() == values.capacity() {
        values
            .try_reserve_exact(256.min(limit - values.len()))
            .map_err(|_| Error("pe_v2_allocation"))?;
    }
    values.push(value);
    Ok(())
}
fn disjoint(spans: &mut [Span], code: &'static str) -> Result<(), Error> {
    spans.sort_unstable();
    require(
        spans.windows(2).all(|pair| !pair[0].overlaps(pair[1])),
        code,
    )
}
fn control_at(controls: &[Span], span: Span) -> Option<Span> {
    let index = controls.partition_point(|c| c.rva + c.size <= span.rva);
    controls
        .get(index)
        .copied()
        .filter(|control| control.overlaps(span))
}
fn cross_disjoint(a: &[Span], b: &[Span]) -> Result<(), Error> {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        require(!a[i].overlaps(b[j]), "pe_v2_write_alias")?;
        if a[i].rva + a[i].size <= b[j].rva {
            i += 1;
        } else {
            j += 1;
        }
    }
    Ok(())
}

impl Image<'_> {
    fn v2_read_only(&self, rva: u64, size: u64) -> Result<(), Error> {
        let s = self.mapped(rva, size, true)?;
        require(
            s.name == *b".rdata\0\0" && s.flags == 0x4000_0040,
            "pe_v2_read_only",
        )
    }
    fn v2_code(&self, rva: u64, size: u64) -> Result<(), Error> {
        let s = self.mapped(rva, size, true)?;
        require(
            s.name == *b".text\0\0\0" && s.flags == 0x6000_0020,
            "pe_v2_code",
        )
    }

    /// Closed observed MSVC shape, not a general load-config/loader model.
    /// 0x80038003 is an opaque observed signature, not an ABI version. The
    /// platform capture separately requires native AMD64. No bytes are masked
    /// beyond the existing digest slot. We cover declared relocations, imports,
    /// TLS-index and conservative cookie/guard-slot writes, not arbitrary code
    /// or every Windows/CRT write. Import names and TLS templates are not parsed.
    /// Vector lengths cap 65,536 relocations/import writes, 65,568 controls and
    /// 1,043 typed fixups (including at most 1,024 callbacks); no recursive stack.
    fn msvc_v2(
        &self,
        directories: &[(u64, u64); 16],
        base: u64,
        dll_flags: u64,
    ) -> Result<(), Error> {
        let va_rva = |va: u64| -> Result<u64, Error> {
            let rva = va.checked_sub(base).ok_or(Error("pe_v2_va"))?;
            require((1..=u64::from(u32::MAX)).contains(&rva), "pe_v2_va")?;
            Ok(rva)
        };
        require(dll_flags == 0x8160, "pe_v2_flags")?;
        require(
            base > 0 && base.checked_add(MAX_FILE_BYTES as u64).is_some(),
            "pe_v2_va",
        )?;
        let config = Span {
            rva: directories[10].0,
            size: directories[10].1,
        };
        require(config.rva != 0 && config.size == 320, "pe_v2_load_config")?;
        self.v2_read_only(config.rva, config.size)?;
        let p = self.offset(config.rva, config.size)?;
        let fields = [88, 112, 120, 256, 280, 288, 296, 304, 312];
        let mut mask = [false; 320];
        mask[..4].fill(true);
        mask[144..148].fill(true);
        for field in fields {
            mask[field..field + 8].fill(true);
        }
        require(
            integer(self.data, p, 4)? == 320
                && integer(self.data, p + 144, 4)? == 0x100
                && mask
                    .iter()
                    .enumerate()
                    .all(|(i, known)| *known || self.data[p + i] == 0),
            "pe_v2_load_config",
        )?;
        let mut pointers = [0; 9];
        let mut controls = Vec::new();
        for (index, &(rva, size)) in directories.iter().enumerate() {
            if rva != 0 && index != 12 {
                push_bounded(&mut controls, Span { rva, size }, MAX_CONTROLS)?;
            }
        }
        let mut expected = Vec::new();
        for (index, field) in fields.into_iter().enumerate() {
            pointers[index] = va_rva(integer(self.data, p + field, 8)?)?;
            push_bounded(
                &mut expected,
                Fixup {
                    target: config.rva + field as u64,
                    owner: Some(config),
                    seen: false,
                },
                MAX_TYPED_FIXUPS,
            )?;
        }
        let cookie = Span {
            rva: pointers[0],
            size: 8,
        };
        let s = self.mapped(cookie.rva, 8, true)?;
        require(
            s.name == *b".data\0\0\0"
                && s.flags == 0xc000_0040
                && cookie.rva.is_multiple_of(8)
                && integer(self.data, self.offset(cookie.rva, 8)?, 8)? == 0x2b99_2ddf_a232,
            "pe_v2_cookie",
        )?;
        let mut destinations = [cookie; 8];
        let mut next = 1;
        for (field, rva) in fields.into_iter().zip(pointers) {
            if field == 88 || field == 256 {
                continue;
            }
            self.v2_read_only(rva, 8)?;
            require(rva % 8 == 0, "pe_v2_slot")?;
            let initial = integer(self.data, self.offset(rva, 8)?, 8)?;
            if field == 304 {
                require(initial == 0, "pe_v2_slot")?;
            } else {
                self.v2_code(va_rva(initial)?, 1)?;
                push_bounded(
                    &mut expected,
                    Fixup {
                        target: rva,
                        owner: None,
                        seen: false,
                    },
                    MAX_TYPED_FIXUPS,
                )?;
            }
            destinations[next] = Span { rva, size: 8 };
            next += 1;
        }
        disjoint(&mut destinations, "pe_v2_write_alias")?;
        let volatile = pointers[3];
        self.v2_read_only(volatile, 88)?;
        let q = self.offset(volatile, 88)?;
        let header = [24, 0x8003_8003, volatile + 24, 40, volatile + 64, 24];
        require(volatile % 4 == 0, "pe_v2_volatile")?;
        for (i, value) in header.into_iter().enumerate() {
            require(integer(self.data, q + i * 4, 4)? == value, "pe_v2_volatile")?;
        }
        push_bounded(
            &mut controls,
            Span {
                rva: volatile,
                size: 88,
            },
            MAX_CONTROLS,
        )?;
        let mut ranges = [Span { rva: 0, size: 0 }; 3];
        for (i, span) in ranges.iter_mut().enumerate() {
            span.rva = integer(self.data, q + 64 + i * 8, 4)?;
            span.size = integer(self.data, q + 68 + i * 8, 4)?;
            self.v2_code(span.rva, span.size)?;
        }
        require(
            ranges.windows(2).all(|p| p[0].rva + p[0].size <= p[1].rva),
            "pe_v2_volatile",
        )?;
        let mut previous = 0;
        for i in 0..10 {
            let access = integer(self.data, q + 24 + i * 4, 4)?;
            self.v2_code(access, 1)?;
            require(
                access > previous
                    && ranges
                        .iter()
                        .any(|span| span.rva <= access && access < span.rva + span.size),
                "pe_v2_volatile",
            )?;
            previous = access;
        }

        let mut import_writes = Vec::new();
        let (imports, imports_size) = directories[1];
        if imports != 0 {
            require(imports_size <= 1024 * 1024, "pe_import_limit")?;
            let mut p = self.offset(imports, imports_size)?;
            let end = p + imports_size as usize;
            let mut work = 0;
            loop {
                require(p + 20 <= end, "pe_import_descriptor")?;
                if self.data[p..p + 20].iter().all(|b| *b == 0) {
                    break;
                }
                let target = integer(self.data, p + 16, 4)?;
                let lookup = match integer(self.data, p, 4)? {
                    0 => target,
                    value => value,
                };
                require(lookup != 0 && target != 0, "pe_import_descriptor")?;
                let mut count = 0;
                loop {
                    work += 1;
                    require(work <= MAX_WORK, "pe_import_limit")?;
                    let thunk = integer(self.data, self.offset(lookup + count * 8, 8)?, 8)?;
                    self.mapped(target + count * 8, 8, false)?;
                    self.not_seal(target + count * 8, 8)?;
                    push_bounded(
                        &mut import_writes,
                        Span {
                            rva: target + count * 8,
                            size: 8,
                        },
                        MAX_WORK,
                    )?;
                    count += 1;
                    if thunk == 0 {
                        break;
                    }
                }
                push_bounded(
                    &mut controls,
                    Span {
                        rva: lookup,
                        size: count * 8,
                    },
                    MAX_CONTROLS,
                )?;
                p += 20;
            }
        }
        let mut tls_write = None;
        let tls = Span {
            rva: directories[9].0,
            size: directories[9].1,
        };
        if tls.rva != 0 {
            require(tls.size == 40, "pe_tls_size")?;
            let p = self.offset(tls.rva, 40)?;
            let mut values = [0; 4];
            for (i, value) in values.iter_mut().enumerate() {
                *value = integer(self.data, p + i * 8, 8)?;
            }
            let [start, end, index_va, callbacks_va] = values;
            require(
                (start != 0) == (end != 0) && end >= start,
                "pe_v2_tls_template",
            )?;
            if start != 0 {
                let (start, end) = (va_rva(start)?, va_rva(end)?);
                if start != end {
                    self.offset(start, end - start)?;
                    self.not_seal(start, end - start)?;
                } else {
                    self.offset(start, 1)?;
                }
            }
            let index = va_rva(index_va)?;
            self.mapped(index, 4, false)?;
            self.not_seal(index, 4)?;
            tls_write = Some(Span {
                rva: index,
                size: 4,
            });
            for (i, value) in values.into_iter().enumerate() {
                if value != 0 {
                    push_bounded(
                        &mut expected,
                        Fixup {
                            target: tls.rva + i as u64 * 8,
                            owner: Some(tls),
                            seen: false,
                        },
                        MAX_TYPED_FIXUPS,
                    )?;
                }
            }
            if callbacks_va != 0 {
                let callbacks = va_rva(callbacks_va)?;
                let mut count = 0;
                loop {
                    require(count < 1024, "pe_tls_callbacks_limit")?;
                    let value = integer(self.data, self.offset(callbacks + count * 8, 8)?, 8)?;
                    if value == 0 {
                        break;
                    }
                    self.v2_code(va_rva(value)?, 1)?;
                    count += 1;
                }
                let array = Span {
                    rva: callbacks,
                    size: (count + 1) * 8,
                };
                push_bounded(&mut controls, array, MAX_CONTROLS)?;
                for i in 0..count {
                    push_bounded(
                        &mut expected,
                        Fixup {
                            target: callbacks + i * 8,
                            owner: Some(array),
                            seen: false,
                        },
                        MAX_TYPED_FIXUPS,
                    )?;
                }
            }
        }
        disjoint(&mut controls, "pe_v2_control_alias")?;
        for destination in destinations {
            self.not_seal(destination.rva, destination.size)?;
            require(
                control_at(&controls, destination).is_none()
                    && directories
                        .iter()
                        .all(|&(rva, size)| rva == 0 || !destination.overlaps(Span { rva, size })),
                "pe_loader_control_write",
            )?;
        }
        expected.sort_unstable_by_key(|fixup| fixup.target);
        require(
            expected.windows(2).all(|p| p[0].target + 8 <= p[1].target),
            "pe_v2_write_alias",
        )?;
        for fixup in &expected {
            require(
                control_at(
                    &controls,
                    Span {
                        rva: fixup.target,
                        size: 8,
                    },
                ) == fixup.owner,
                "pe_loader_control_write",
            )?;
        }
        let mut reloc_writes = Vec::new();
        let (reloc, reloc_size) = directories[5];
        if reloc != 0 {
            require(reloc_size <= 1024 * 1024, "pe_relocation_limit")?;
            let mut p = self.offset(reloc, reloc_size)?;
            let end = p + reloc_size as usize;
            let mut work = 0;
            while p < end {
                require(p + 8 <= end, "pe_relocation_block")?;
                let page = integer(self.data, p, 4)?;
                let length = integer(self.data, p + 4, 4)? as usize;
                require(
                    page % 4096 == 0
                        && length >= 8
                        && length.is_multiple_of(2)
                        && length <= end - p,
                    "pe_relocation_block",
                )?;
                for at in (p + 8..p + length).step_by(2) {
                    work += 1;
                    require(work <= MAX_WORK, "pe_relocation_limit")?;
                    let entry = integer(self.data, at, 2)?;
                    let kind = entry >> 12;
                    if kind == 0 {
                        continue;
                    }
                    require(kind == 10, "pe_relocation_kind")?;
                    let target = page + (entry & 4095);
                    self.mapped(target, 8, false)?;
                    self.not_seal(target, 8)?;
                    let span = Span {
                        rva: target,
                        size: 8,
                    };
                    let typed = expected.binary_search_by_key(&target, |f| f.target);
                    if let Ok(i) = typed {
                        require(!expected[i].seen, "pe_v2_fixup_count")?;
                        expected[i].seen = true;
                    } else {
                        require(
                            control_at(&controls, span).is_none(),
                            "pe_loader_control_write",
                        )?;
                    }
                    require(
                        destinations.iter().all(|&destination| {
                            !span.overlaps(destination)
                                || (typed.is_ok() && span == destination && destination != cookie)
                        }),
                        "pe_v2_write_alias",
                    )?;
                    push_bounded(&mut reloc_writes, span, MAX_WORK)?;
                }
                p += length;
            }
        }
        require(expected.iter().all(|f| f.seen), "pe_v2_fixup_count")?;
        disjoint(&mut reloc_writes, "pe_v2_write_alias")?;
        disjoint(&mut import_writes, "pe_v2_write_alias")?;
        for span in import_writes.iter().chain(tls_write.iter()).copied() {
            require(
                control_at(&controls, span).is_none(),
                "pe_loader_control_write",
            )?;
            require(
                destinations.iter().all(|d| !span.overlaps(*d)),
                "pe_v2_write_alias",
            )?;
        }
        let tls_writes = tls_write.as_slice();
        cross_disjoint(&reloc_writes, &import_writes)?;
        cross_disjoint(&reloc_writes, tls_writes)?;
        cross_disjoint(&import_writes, tls_writes)
    }
}

pub fn locate(data: &[u8]) -> Result<Layout, Error> {
    locate_with_profile(data, Profile::V1)
}

pub fn locate_with_profile(data: &[u8], profile: Profile) -> Result<Layout, Error> {
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
    require(
        data[seal.raw..seal.raw + 32] == profile.marker(),
        "pe_seal_marker",
    )?;
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
            rva == 0
                || (![4, 7, 8, 11, 13, 14, 15].contains(&index)
                    && (index != 10 || profile == Profile::MsvcV2)),
            "pe_unsupported_directory",
        )?;
        if rva != 0 {
            image.offset(rva, size)?;
            image.not_seal(rva, size)?;
        }
        *directory = (rva, size);
    }
    match profile {
        Profile::V1 => {
            image.relocations(directories[5].0, directories[5].1, directories[1])?;
            image.imports(directories[1].0, directories[1].1, directories[5])?;
            image.tls(directories[9].0, directories[9].1, base)?;
        }
        Profile::MsvcV2 => {
            image.msvc_v2(&directories, base, integer(data, optional + 70, 2)?)?;
        }
    }
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
    verify_pe_with_profile(data, loaded_payload, measured_payload_rva, Profile::V1)
}

pub fn verify_pe_with_profile(
    data: &[u8],
    loaded_payload: &[u8],
    measured_payload_rva: u32,
    profile: Profile,
) -> Result<[u8; 32], Error> {
    require(loaded_payload.len() == 64, "pe_loaded_payload")?;
    require(
        loaded_payload[..32] == profile.marker() && loaded_payload[32..].iter().any(|b| *b != 0),
        "pe_loaded_seal",
    )?;
    let layout = locate_with_profile(data, profile)?;
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
    #[cfg(all(feature = "windows-msvc-seal-v2", target_env = "msvc"))]
    let profile = Profile::MsvcV2;
    #[cfg(not(all(feature = "windows-msvc-seal-v2", target_env = "msvc")))]
    let profile = Profile::V1;
    let identity = verify_pe_with_profile(&bytes, &source.payload(), source.payload_rva(), profile);
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
