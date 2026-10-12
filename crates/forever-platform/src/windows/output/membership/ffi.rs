// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original synchronous NtQueryDirectoryFile declarations.
// SAFETY: Called only with a borrowed private-root directory keeper created
// using FILE_SYNCHRONOUS_IO_NONALERT and SYNCHRONIZE|LIST_DIRECTORY. Windows
// completes this event/APC-free query synchronously. Storage stays aligned,
// initialized and live throughout the call; status/extent is checked before use.
use super::{BUFFER_BYTES, Error, completion};
use std::{
    ffi::c_void,
    fs::File,
    mem::{align_of, offset_of, size_of},
    os::windows::io::AsRawHandle,
    ptr::null_mut,
};
type Handle = *mut c_void;
#[repr(C)]
struct Io {
    status: usize,
    information: usize,
}
type ApcRoutine = Option<unsafe extern "system" fn(*mut c_void, *mut Io, u32)>;
#[repr(C)]
struct FileIdFull {
    next: u32,
    index: u32,
    times: [i64; 6],
    attributes: u32,
    name_length: u32,
    ea_size: u32,
    file_id: i64,
    name: [u16; 1],
}
#[repr(C, align(8))]
pub(super) struct Buffer {
    bytes: [u8; BUFFER_BYTES],
}
impl Buffer {
    pub(super) fn new() -> Self {
        Self {
            bytes: [0; BUFFER_BYTES],
        }
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
const _: () = {
    assert!(size_of::<Io>() == 16 && align_of::<Io>() == 8);
    assert!(offset_of!(Io, information) == 8);
    assert!(size_of::<FileIdFull>() == 88 && align_of::<FileIdFull>() == 8);
    assert!(offset_of!(FileIdFull, next) == 0);
    assert!(offset_of!(FileIdFull, index) == 4);
    assert!(offset_of!(FileIdFull, times) == 8);
    assert!(offset_of!(FileIdFull, attributes) == 56);
    assert!(offset_of!(FileIdFull, name_length) == 60);
    assert!(offset_of!(FileIdFull, ea_size) == 64);
    assert!(offset_of!(FileIdFull, file_id) == 72);
    assert!(offset_of!(FileIdFull, name) == 80);
    assert!(align_of::<Buffer>() == 8 && size_of::<Buffer>() == BUFFER_BYTES);
    assert!(BUFFER_BYTES >= 80 + 255 * 2 + 7);
};
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryDirectoryFile(
        file: Handle,
        event: Handle,
        apc: ApcRoutine,
        context: Handle,
        io: *mut Io,
        information: Handle,
        length: u32,
        class: i32,
        single: u8,
        name: Handle,
        restart: u8,
    ) -> i32;
}
pub(super) fn query(
    file: &File,
    buffer: &mut Buffer,
    restart: bool,
) -> Result<Option<usize>, Error> {
    buffer.bytes.fill(0);
    let mut io = Io {
        status: usize::MAX,
        information: usize::MAX,
    };
    // No event/APC, no captured name filter, single entry, explicit restart on
    // each directory's first query. Anomalous pending/overflow statuses fail.
    let status = unsafe {
        NtQueryDirectoryFile(
            file.as_raw_handle(),
            null_mut(),
            None,
            null_mut(),
            &mut io,
            buffer.bytes.as_mut_ptr().cast(),
            BUFFER_BYTES as u32,
            38,
            1,
            null_mut(),
            u8::from(restart),
        )
    };
    completion(status as u32, io.status as u32, io.information)
}
