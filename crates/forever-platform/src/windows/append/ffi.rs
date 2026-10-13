// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: native AMD64 admission before append pathname I/O.
use super::Error;
use std::ffi::c_void;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn IsWow64Process2(
        process: *mut c_void,
        process_machine: *mut u16,
        native_machine: *mut u16,
    ) -> i32;
}
pub(super) fn architecture() -> Result<(), Error> {
    let mut process = 0xffff_u16;
    let mut native = 0xffff_u16;
    // SAFETY: GetCurrentProcess returns a borrowed pseudo-handle, never closed.
    // Both USHORT outputs are initialized, aligned and live for this sync call.
    let success = unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, &mut native) } != 0;
    super::architecture(success, process, native)
}

use super::Identity;
use std::{
    fs::File,
    mem::{align_of, offset_of, size_of},
    os::windows::io::AsRawHandle,
};
#[repr(C)]
#[derive(Default)]
struct Time {
    low: u32,
    high: u32,
}
impl Time {
    fn value(&self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}
#[repr(C)]
#[derive(Default)]
struct Info {
    attributes: u32,
    creation: Time,
    access: Time,
    write: Time,
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
}
const _: () = {
    assert!(size_of::<Time>() == 8 && align_of::<Time>() == 4);
    assert!(size_of::<Info>() == 52 && align_of::<Info>() == 4);
    assert!(
        offset_of!(Info, volume) == 28
            && offset_of!(Info, links) == 40
            && offset_of!(Info, index_high) == 44
    );
};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileType(file: *mut c_void) -> u32;
    #[allow(clashing_extern_declarations)]
    fn GetFileInformationByHandle(file: *mut c_void, info: *mut Info) -> i32;
}
fn convert(info: Info) -> Identity {
    Identity {
        volume: info.volume,
        file_index: (u64::from(info.index_high) << 32) | u64::from(info.index_low),
        links: info.links,
        attributes: info.attributes,
        size: (u64::from(info.size_high) << 32) | u64::from(info.size_low),
        creation_time: info.creation.value(),
        write_time: info.write.value(),
        change_time: 0,
    }
}
pub(super) fn identity(file: &File) -> Result<Identity, Error> {
    let mut before = Info::default();
    let mut after = Info::default();
    // SAFETY: the private File remains alive and unshared, outputs use the exact
    // independently asserted BY_HANDLE_FILE_INFORMATION ABI. Calls are sync,
    // do not retain pointers and do not acquire handle ownership.
    let success = unsafe {
        GetFileType(file.as_raw_handle()) == 1
            && GetFileInformationByHandle(file.as_raw_handle(), &mut before) != 0
            && GetFileInformationByHandle(file.as_raw_handle(), &mut after) != 0
    };
    if !success {
        return Err(Error::Changed);
    }
    // Only live-writer write/change times are excluded. Unlike the v1 directory
    // projection, positive linkcount or EOF churn inside this query rejects.
    super::mutable_snapshot(convert(before), convert(after))
}

#[cfg(test)]
#[link(name = "kernel32")]
unsafe extern "system" {
    #[allow(clashing_extern_declarations)]
    fn SetFileTime(
        file: *mut c_void,
        creation: *const Time,
        access: *const Time,
        write: *const Time,
    ) -> i32;
}
#[cfg(test)]
pub(super) fn test_creation(file: &File, value: u64) -> Result<(), Error> {
    let time = Time {
        low: value as u32,
        high: (value >> 32) as u32,
    };
    // SAFETY: test-only borrowed metadata-write handle and exact initialized
    // FILETIME; NULL preserves both other timestamps. No pointer is retained.
    if unsafe {
        SetFileTime(
            file.as_raw_handle(),
            &time,
            std::ptr::null(),
            std::ptr::null(),
        )
    } == 0
    {
        Err(Error::Io)
    } else {
        Ok(())
    }
}
