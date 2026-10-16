// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: original GNU/Linux x86_64 statx FFI boundary.
// ABI facts: linux/stat.h and glibc bits/types/struct_statx.h. Only the mask
// and conventional mount-ID field are interpreted; other storage is opaque.
use super::{Error, STATX_MNT_ID, response};
use std::ffi::{c_int, c_long, c_uint};

const AT_EMPTY_PATH: c_int = 0x1000;
// Linux x86_64 LP64 UAPI asm/unistd_64.h; no newer libc statx symbol needed.
const NR_STATX: c_long = 332;

#[repr(C)]
struct Statx {
    mask: u32,
    before_mount: [u8; 140],
    mount: u64,
    after_mount: [u8; 104],
}
const _: () = assert!(size_of::<Statx>() == 256 && align_of::<Statx>() == 8);
const _: () = assert!(std::mem::offset_of!(Statx, mask) == 0);
const _: () = assert!(std::mem::offset_of!(Statx, mount) == 0x90);
const _: () = assert!(std::mem::offset_of!(Statx, after_mount) == 0x98);
const _: () = assert!(size_of::<c_int>() == 4 && size_of::<c_uint>() == 4);
const _: () = assert!(size_of::<c_long>() == 8 && size_of::<*mut Statx>() == 8);

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
}

pub(super) fn id(fd: c_int) -> Result<u64, Error> {
    let mut buffer = Statx {
        mask: 0,
        before_mount: [0; 140],
        mount: 0,
        after_mount: [0; 104],
    };
    // SAFETY: enclosing cfg admits GNU/Linux x86_64 and the asserted 256-byte,
    // 8-aligned ABI. Empty C pathname lives through the call; buffer is fully
    // initialized, writable and live. fd is borrowed from a live File, with no
    // close/ownership transfer. AT_EMPTY_PATH addresses that description and
    // only STATX_MNT_ID is requested (not its distinct UNIQUE variant). The
    // admitted x86_64 LP64 syscall ABI takes fd/flags as int, mask as unsigned
    // int and the two pointers in its general-purpose argument registers.
    let status = unsafe {
        syscall(
            NR_STATX,
            fd,
            c"".as_ptr(),
            AT_EMPTY_PATH,
            STATX_MNT_ID as c_uint,
            &mut buffer as *mut Statx,
        )
    };
    // Capture errno immediately and only on libc's documented error return.
    let errno = if status == -1 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    } else {
        0
    };
    let status = c_int::try_from(status).map_err(|_| Error::Invalid)?;
    response(status, errno, buffer.mask, buffer.mount)
}
