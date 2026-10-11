// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic WMO comparison adapter.
//! Linux QA only: wmo_raw <fixture> <new-output>.
//! Fixture ABI matches tests/forever/native_wmo_conversion.cpp: RCEWM001,
//! eight LE u32 values (rootFlags, rootID, declaredRootGroups, groupNameBytes,
//! inputGroups, typeRows, materialRows, outputCap), then names, type rows
//! {u32 id,u8 material}, material rows {u32 id,u32 flags}, and group records
//! {u32 FDID,u32 byteLength,complete bytes}. Exact EOF, at most 64 MiB.
//!
//! Synthetic root metadata is framed as MOHD64/MOGN/GFID. Input groups form
//! the nonzero detailed inventory; undeclared slots and extra LODs are zero.
//! This extra inventory consistency gate is not an original WMORoot parser
//! claim. No client acquisition, CASC, network or filesystem-name lookup.

#[cfg(target_os = "linux")]
mod qa {
    use std::fs::{self, Metadata, OpenOptions};
    use std::io::{Read, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Path;

    use forever_vmap::wmo::liquid::LiquidLookup;
    use forever_vmap::wmo::raw::{self, ConversionLimits, MAX_RAW_BYTES, RawWmo};
    use forever_vmap::wmo::{Group, GroupInput, Limits, MAX_GROUPS, MAX_INPUT_BYTES, Root};

    const O_NONBLOCK: i32 = 0x800;
    const O_NOFOLLOW: i32 = 0x20000;
    const MAX_NAMES: usize = 65536;
    const MAX_INPUT_GROUPS: usize = 65536;
    const MAX_LOOKUP_ROWS: usize = 10000;

    fn same(first: &Metadata, second: &Metadata) -> bool {
        first.dev() == second.dev()
            && first.ino() == second.ino()
            && first.mode() == second.mode()
            && first.len() == second.len()
            && first.mtime() == second.mtime()
            && first.mtime_nsec() == second.mtime_nsec()
            && first.ctime() == second.ctime()
            && first.ctime_nsec() == second.ctime_nsec()
    }

    fn allocate<T>(count: usize) -> Result<Vec<T>, &'static str> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| "qa_allocation")?;
        Ok(values)
    }

    fn read_complete(path: &Path) -> Result<Vec<u8>, &'static str> {
        let named = fs::symlink_metadata(path).map_err(|_| "qa_input_metadata")?;
        if !named.is_file() || named.file_type().is_symlink() {
            return Err("qa_input_not_file");
        }
        if named.len() > MAX_INPUT_BYTES as u64 {
            return Err("qa_input_limit");
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_input_open")?;
        let before = file.metadata().map_err(|_| "qa_input_metadata")?;
        if !same(&named, &before) {
            return Err("qa_input_changed");
        }
        let length = usize::try_from(before.len()).map_err(|_| "qa_input_limit")?;
        let mut bytes = allocate(length)?;
        bytes.resize(length, 0);
        let read = file.read_exact(&mut bytes);
        let after = file.metadata().map_err(|_| "qa_input_metadata")?;
        let named_after = fs::symlink_metadata(path).map_err(|_| "qa_input_changed")?;
        if !same(&before, &after) || !same(&before, &named_after) {
            return Err("qa_input_changed");
        }
        read.map_err(|_| "qa_input_read")?;
        let mut tail = [0u8; 1];
        if file.read(&mut tail).map_err(|_| "qa_input_read")? != 0 {
            return Err("qa_input_changed");
        }
        let after_tail = file.metadata().map_err(|_| "qa_input_metadata")?;
        let named_tail = fs::symlink_metadata(path).map_err(|_| "qa_input_changed")?;
        if !same(&before, &after_tail) || !same(&before, &named_tail) {
            return Err("qa_input_changed");
        }
        Ok(bytes)
    }

    struct Cursor<'a> {
        bytes: &'a [u8],
        at: usize,
    }
    impl<'a> Cursor<'a> {
        fn take(&mut self, size: usize) -> Result<&'a [u8], &'static str> {
            let end = self.at.checked_add(size).ok_or("qa_input_extent")?;
            let bytes = self.bytes.get(self.at..end).ok_or("qa_input_extent")?;
            self.at = end;
            Ok(bytes)
        }
        fn word(&mut self) -> Result<u32, &'static str> {
            Ok(u32::from_le_bytes(
                self.take(4)?.try_into().map_err(|_| "qa_input_extent")?,
            ))
        }
    }

    struct TypeRow {
        id: u32,
        ordinal: u32,
        material: u8,
    }
    struct MaterialRow {
        id: u32,
        ordinal: u32,
        flags: u32,
    }
    struct Lookup {
        types: Vec<TypeRow>,
        materials: Vec<MaterialRow>,
    }
    impl LiquidLookup for Lookup {
        fn material_id(&self, id: u32) -> Option<u8> {
            let at = self
                .types
                .partition_point(|row| row.id <= id)
                .checked_sub(1)?;
            let row = self.types.get(at)?;
            (row.id == id).then_some(row.material)
        }
        fn material_flags(&self, id: u8) -> Option<u32> {
            let id = u32::from(id);
            let at = self
                .materials
                .partition_point(|row| row.id <= id)
                .checked_sub(1)?;
            let row = self.materials.get(at)?;
            (row.id == id).then_some(row.flags)
        }
    }
    struct InputGroup<'a> {
        file_data_id: u32,
        bytes: &'a [u8],
    }
    struct Fixture<'a> {
        root_flags: u16,
        root_id: u32,
        declared_groups: usize,
        names: &'a [u8],
        groups: Vec<InputGroup<'a>>,
        lookup: Lookup,
        output_cap: usize,
    }

    fn fixture(bytes: &[u8]) -> Result<Fixture<'_>, &'static str> {
        let mut input = Cursor { bytes, at: 0 };
        if input.take(8)? != b"RCEWM001" {
            return Err("qa_input_magic");
        }
        let root_flags = u16::try_from(input.word()?).map_err(|_| "qa_root_flags")?;
        let root_id = input.word()?;
        let declared_groups = input.word()? as usize;
        let name_bytes = input.word()? as usize;
        let group_count = input.word()? as usize;
        let type_count = input.word()? as usize;
        let material_count = input.word()? as usize;
        let output_cap = input.word()? as usize;
        if declared_groups > MAX_GROUPS
            || name_bytes > MAX_NAMES
            || group_count > MAX_INPUT_GROUPS
            || group_count > declared_groups
            || type_count > MAX_LOOKUP_ROWS
            || material_count > MAX_LOOKUP_ROWS
            || output_cap > MAX_RAW_BYTES
        {
            return Err("qa_counts");
        }
        let names = input.take(name_bytes)?;
        let mut types = allocate(type_count)?;
        let mut materials = allocate(material_count)?;
        let mut groups = allocate(group_count)?;
        for ordinal in 0..type_count {
            let id = input.word()?;
            let material = input.take(1)?[0];
            types.push(TypeRow {
                id,
                ordinal: ordinal as u32,
                material,
            });
        }
        for ordinal in 0..material_count {
            materials.push(MaterialRow {
                id: input.word()?,
                ordinal: ordinal as u32,
                flags: input.word()?,
            });
        }
        // Sorting includes physical ordinal; the final equal-key row implements
        // native map assignment's last-row-wins rule without insert defaults.
        types.sort_unstable_by_key(|row| (row.id, row.ordinal));
        materials.sort_unstable_by_key(|row| (row.id, row.ordinal));
        for _ in 0..group_count {
            let file_data_id = input.word()?;
            if file_data_id == 0 {
                return Err("qa_group_id");
            }
            let size = input.word()? as usize;
            groups.push(InputGroup {
                file_data_id,
                bytes: input.take(size)?,
            });
        }
        if input.at != bytes.len() {
            return Err("qa_input_tail");
        }
        Ok(Fixture {
            root_flags,
            root_id,
            declared_groups,
            names,
            groups,
            lookup: Lookup { types, materials },
            output_cap,
        })
    }

    fn frame_header(bytes: &mut Vec<u8>, tag: &[u8; 4], size: usize) -> Result<(), &'static str> {
        bytes.extend(tag.iter().rev().copied());
        bytes.extend_from_slice(
            &u32::try_from(size)
                .map_err(|_| "qa_root_extent")?
                .to_le_bytes(),
        );
        Ok(())
    }

    fn framed_root(fixture: &Fixture<'_>) -> Result<Vec<u8>, &'static str> {
        let lods = if fixture.root_flags & 0x10 != 0 { 3 } else { 1 };
        let gfid_size = fixture
            .declared_groups
            .checked_mul(lods)
            .and_then(|v| v.checked_mul(4))
            .ok_or("qa_root_extent")?;
        let size = 88usize
            .checked_add(fixture.names.len())
            .and_then(|v| v.checked_add(gfid_size))
            .ok_or("qa_root_extent")?;
        if size > MAX_INPUT_BYTES {
            return Err("qa_input_limit");
        }
        let mut bytes = allocate(size)?;
        frame_header(&mut bytes, b"MOHD", 64)?;
        let mut header = [0u8; 64];
        header[4..8].copy_from_slice(&(fixture.declared_groups as u32).to_le_bytes());
        header[32..36].copy_from_slice(&fixture.root_id.to_le_bytes());
        header[60..62].copy_from_slice(&fixture.root_flags.to_le_bytes());
        // numLod remains zero: the source's flag0x10 default denotes 3 LODs.
        bytes.extend_from_slice(&header);
        frame_header(&mut bytes, b"MOGN", fixture.names.len())?;
        bytes.extend_from_slice(fixture.names);
        frame_header(&mut bytes, b"GFID", gfid_size)?;
        for group in &fixture.groups {
            bytes.extend_from_slice(&group.file_data_id.to_le_bytes());
        }
        bytes.resize(size, 0); // Unused detailed slots and extra LODs.
        Ok(bytes)
    }

    fn require_absent(path: &Path) -> Result<(), &'static str> {
        match fs::symlink_metadata(path) {
            Ok(_) => Err("qa_output_exists"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("qa_output_metadata"),
        }
    }
    fn write_new(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_output_create")?;
        // Complete conversion and verification precede creation. An I/O error
        // can retain this new partial file for inspection, never report success.
        // Do not automatically remove a path another process could replace.
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "qa_output_write")
    }

    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() != 3 {
            return Err("qa_usage");
        }
        let destination = Path::new(&args[2]);
        require_absent(destination)?;
        let bytes = read_complete(Path::new(&args[1]))?;
        let fixture = fixture(&bytes)?;
        let root_bytes = framed_root(&fixture)?;
        let root = Root::parse(&root_bytes, Limits::default()).map_err(|e| e.code())?;
        let mut parsed = allocate(fixture.groups.len())?;
        for group in &fixture.groups {
            parsed.push(Group::parse(group.bytes, Limits::default()).map_err(|e| e.code())?);
        }
        let mut inputs = allocate(parsed.len())?;
        for (input, group) in fixture.groups.iter().zip(&parsed) {
            inputs.push(GroupInput {
                file_data_id: input.file_data_id,
                group,
            });
        }
        let limits = ConversionLimits {
            max_raw_bytes: fixture.output_cap,
            ..ConversionLimits::default()
        };
        let Some(converted) =
            raw::convert(&root, &inputs, &fixture.lookup, limits).map_err(|e| e.code())?
        else {
            println!(
                "{{\"status\":\"no_collision\",\"bytes\":0,\"triangles\":0,\"input_groups\":{}}}",
                inputs.len()
            );
            return Ok(());
        };
        let verified = RawWmo::parse(converted.bytes(), limits).map_err(|e| e.code())?;
        if converted.group_count() != verified.group_count()
            || converted.triangle_count() != verified.triangle_count()
        {
            return Err("qa_output_counts");
        }
        write_new(destination, verified.bytes())?;
        println!(
            "{{\"status\":\"complete\",\"bytes\":{},\"groups\":{},\"triangles\":{},\"input_groups\":{}}}",
            verified.bytes().len(),
            verified.group_count(),
            verified.triangle_count(),
            inputs.len()
        );
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(code) = qa::run() {
        println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
        std::process::exit(4);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("{{\"status\":\"error\",\"code\":\"qa_platform_unsupported\"}}");
    std::process::exit(4);
}
