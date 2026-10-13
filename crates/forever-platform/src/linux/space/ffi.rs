// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original GNU/Linux LP64 fstatvfs ABI boundary.
// ABI reviewed against local x86_64-linux-gnu sys/statvfs.h,
// bits/statvfs.h and bits/typesizes.h. Only ABI declarations are reproduced.
use super::Error;
use std::ffi::c_int;

#[repr(C)]
struct Statvfs {
    block_size: u64,
    fragment_size: u64,
    blocks: u64,
    free_blocks: u64,
    available_blocks: u64,
    files: u64,
    free_files: u64,
    available_files: u64,
    filesystem_id: u64,
    flags: u64,
    max_name: u64,
    // Opaque ABI storage: older GNU headers use six spare ints; newer headers
    // use one filesystem-type uint and five spare ints in the same 24 bytes.
    tail: [u32; 6],
}
const _: () = assert!(size_of::<Statvfs>() == 112 && align_of::<Statvfs>() == 8);
const _: () = assert!(std::mem::offset_of!(Statvfs, fragment_size) == 8);
const _: () = assert!(std::mem::offset_of!(Statvfs, available_blocks) == 32);
const _: () = assert!(std::mem::offset_of!(Statvfs, files) == 40);
const _: () = assert!(std::mem::offset_of!(Statvfs, available_files) == 56);
const _: () = assert!(std::mem::offset_of!(Statvfs, tail) == 88);

unsafe extern "C" {
    fn fstatvfs(fd: c_int, output: *mut Statvfs) -> c_int;
}

pub(super) fn query(fd: c_int) -> Result<(u64, u64, u64, u64), Error> {
    let mut output = Statvfs {
        block_size: 0,
        fragment_size: 0,
        blocks: 0,
        free_blocks: 0,
        available_blocks: 0,
        files: 0,
        free_files: 0,
        available_files: 0,
        filesystem_id: 0,
        flags: 0,
        max_name: 0,
        tail: [0; 6],
    };
    // SAFETY: the exact GNU x86_64 LP64 ABI is cfg-gated in the parent. Output
    // is initialized, writable, correctly aligned and lives through the call.
    // Production borrows the descriptor from a live File; invalid integers also
    // fail safely through libc. libc neither closes the descriptor nor changes
    // its flags or ownership. No output is accepted after syscall error.
    if unsafe { fstatvfs(fd, &mut output) } != 0 {
        return Err(Error::Unavailable);
    }
    Ok((
        output.fragment_size,
        output.available_blocks,
        output.files,
        output.available_files,
    ))
}
