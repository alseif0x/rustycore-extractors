// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original minimal Win32 declarations; no upstream imports.
// SAFETY: Every raw handle is borrowed from a live std::fs::File. Calls are
// synchronous and do not own/close handles. Output pointers target initialized
// repr(C) storage of the exact API size; UTF16 buffers live across each call.

use super::Identity;
use crate::Error;
use std::{ffi::c_void, fs::File, os::windows::io::AsRawHandle};

type Handle = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}
impl FileTime {
    fn value(self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileInfo {
    attributes: u32,
    creation: FileTime,
    access: FileTime,
    write: FileTime,
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
}
#[repr(C)]
#[derive(Default)]
struct BasicInfo {
    creation: i64,
    access: i64,
    write: i64,
    change: i64,
    attributes: u32,
}
const _: () = assert!(std::mem::size_of::<FileInfo>() == 52);
const _: () = assert!(std::mem::size_of::<BasicInfo>() == 40);
const _: () = assert!(std::mem::align_of::<BasicInfo>() == 8);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileType(handle: Handle) -> u32;
    fn GetFileInformationByHandle(handle: Handle, info: *mut FileInfo) -> i32;
    fn GetFileInformationByHandleEx(
        handle: Handle,
        class: i32,
        info: *mut c_void,
        size: u32,
    ) -> i32;
    fn GetDriveTypeW(root: *const u16) -> u32;
    fn GetVolumeInformationByHandleW(
        handle: Handle,
        name: *mut u16,
        name_len: u32,
        serial: *mut u32,
        max_component: *mut u32,
        flags: *mut u32,
        filesystem: *mut u16,
        filesystem_len: u32,
    ) -> i32;
}

pub(super) fn fixed_drive(root: &str) -> Result<(), Error> {
    let b = root.as_bytes();
    let wide = [u16::from(b[0]), u16::from(b[1]), u16::from(b[2]), 0];
    // SAFETY: Root was admitted as three ASCII drive-root bytes; terminated
    // buffer is valid for this synchronous non-owning call.
    if unsafe { GetDriveTypeW(wide.as_ptr()) } != 3 {
        return Err(Error::FileType);
    }
    Ok(())
}
pub(super) fn ntfs_volume(file: &File) -> Result<(), Error> {
    let mut filesystem = [0_u16; 32];
    // SAFETY: Borrowed handle and correctly sized mutable UTF16 output buffer;
    // optional unused outputs are NULL as permitted by the API.
    let ok = unsafe {
        GetVolumeInformationByHandleW(
            file.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    if ok == 0 || filesystem[..5] != [78, 84, 70, 83, 0] {
        return Err(Error::FileType);
    }
    Ok(())
}
pub(super) fn identity(file: &File) -> Result<Identity, Error> {
    query_identity(file, false)
}

/// Captures only a stable object identity/attribute projection. Ancillary
/// metadata in the returned value is not a consistent content snapshot and must
/// only be admitted and compared with Identity::same_object. Directory sibling
/// changes may alter that metadata between any of the three queries.
pub(super) fn object_identity(file: &File) -> Result<Identity, Error> {
    query_identity(file, true)
}

fn query_identity(file: &File, object_only: bool) -> Result<Identity, Error> {
    let handle = file.as_raw_handle();
    // SAFETY: File owns a live handle; GetFileType neither mutates nor retains it.
    if unsafe { GetFileType(handle) } != 1 {
        return Err(Error::FileType);
    }
    let mut before = FileInfo::default();
    let mut after = FileInfo::default();
    let mut basic = BasicInfo::default();
    // SAFETY: Exact repr(C) layouts asserted above. All initialized buffers are
    // writable and alive for the synchronous calls. FileBasicInfo class = 0.
    let ok = unsafe {
        GetFileInformationByHandle(handle, &mut before) != 0
            && GetFileInformationByHandleEx(
                handle,
                0,
                (&mut basic as *mut BasicInfo).cast(),
                std::mem::size_of::<BasicInfo>() as u32,
            ) != 0
            && GetFileInformationByHandle(handle, &mut after) != 0
    };
    if !ok {
        return Err(Error::Identity);
    }
    checked_snapshot(before, basic, after, object_only)
}

fn checked_snapshot(
    before: FileInfo,
    basic: BasicInfo,
    after: FileInfo,
    object_only: bool,
) -> Result<Identity, Error> {
    let convert = |value: FileInfo| Identity {
        volume: value.volume,
        file_index: (u64::from(value.index_high) << 32) | u64::from(value.index_low),
        links: value.links,
        attributes: value.attributes,
        size: (u64::from(value.size_high) << 32) | u64::from(value.size_low),
        creation_time: value.creation.value(),
        write_time: value.write.value(),
        change_time: basic.change,
    };
    let result = convert(before);
    // A zero link count is never an admitted object, even when differing
    // positive link counts are intentionally outside this comparison policy.
    if before.links == 0 || after.links == 0 {
        return Err(Error::Identity);
    }
    let matches = if object_only {
        result.same_object(convert(after))
    } else {
        result == convert(after)
            && result.creation_time == basic.creation as u64
            && result.write_time == basic.write as u64
    };
    if !matches || result.attributes != basic.attributes {
        return Err(Error::Changed);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> (FileInfo, BasicInfo, FileInfo) {
        let info = FileInfo {
            attributes: super::super::FILE_ATTRIBUTE_DIRECTORY,
            volume: 7,
            index_low: 11,
            links: 1,
            creation: FileTime { low: 13, high: 0 },
            write: FileTime { low: 17, high: 0 },
            ..FileInfo::default()
        };
        let basic = BasicInfo {
            attributes: info.attributes,
            creation: 13,
            write: 17,
            change: 19,
            ..BasicInfo::default()
        };
        (info, basic, info)
    }

    #[test]
    fn object_query_accepts_within_query_metadata_churn_but_full_query_rejects() {
        let (before, mut basic, mut after) = snapshot();
        basic.creation = 23;
        basic.write = 29;
        basic.change = 31;
        after.creation.low = 37;
        after.write.low = 41;
        after.size_low = 43;
        after.links = 2;
        let identity = checked_snapshot(before, basic, after, true).unwrap();
        assert_eq!(identity.volume, before.volume);
        assert_eq!(identity.file_index, u64::from(before.index_low));

        // Each full-only field is checked independently: one mismatch cannot
        // conceal an accidental weakening of a different snapshot comparison.
        for field in 0..7 {
            for object_only in [false, true] {
                let (before, mut basic, mut after) = snapshot();
                match field {
                    0 => after.links = 2,
                    1 => after.size_low = 43,
                    2 => after.size_high = 1,
                    3 => after.creation.low = 37,
                    4 => after.write.low = 41,
                    5 => basic.creation = 23,
                    _ => basic.write = 29,
                }
                let result = checked_snapshot(before, basic, after, object_only);
                if object_only {
                    assert!(result.is_ok());
                } else {
                    assert_eq!(result, Err(Error::Changed));
                }
            }
        }
    }

    #[test]
    fn object_and_full_queries_reject_identity_attribute_or_link_admission_changes() {
        for object_only in [false, true] {
            for field in 0..5 {
                let (before, mut basic, mut after) = snapshot();
                match field {
                    0 => after.volume ^= 1,
                    1 => after.index_low ^= 1,
                    2 => after.index_high ^= 1,
                    3 => after.attributes ^= super::super::FILE_ATTRIBUTE_REPARSE_POINT,
                    _ => basic.attributes ^= super::super::FILE_ATTRIBUTE_REPARSE_POINT,
                }
                assert_eq!(
                    checked_snapshot(before, basic, after, object_only),
                    Err(Error::Changed)
                );
            }
            for zero_before in [false, true] {
                let (mut before, basic, mut after) = snapshot();
                if zero_before {
                    before.links = 0;
                } else {
                    after.links = 0;
                }
                assert_eq!(
                    checked_snapshot(before, basic, after, object_only),
                    Err(Error::Identity)
                );
            }
        }
    }
}
