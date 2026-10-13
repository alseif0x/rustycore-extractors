// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original scoped loaded-module Win32 FFI and seal storage.

use super::{Error, MARKER, PAYLOAD_BYTES};
use crate::path::Limits;
use std::{ffi::c_void, path::PathBuf, ptr};

const fn initial_payload() -> [u8; PAYLOAD_BYTES] {
    let mut result = [0; PAYLOAD_BYTES];
    let mut i = 0;
    while i < MARKER.len() {
        result[i] = MARKER[i];
        i += 1;
    }
    result
}

// SAFETY: unique original section name; immutable initialized bytes have no
// references to writable memory. Final link output must be validated/sealed by
// the strict PE packager. This attribute alone does not assert PE permissions.
#[used]
#[unsafe(link_section = ".rceid")]
static LOADED_PAYLOAD: [u8; PAYLOAD_BYTES] = initial_payload();

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, output: *mut u16, size: u32) -> u32;
}

pub(super) struct Snapshot {
    pub(super) payload: [u8; PAYLOAD_BYTES],
    pub(super) payload_rva: u32,
}

pub(super) fn snapshot() -> Result<Snapshot, Error> {
    let address = ptr::addr_of!(LOADED_PAYLOAD);
    // SAFETY: live immutable initialized static, naturally aligned for [u8;64].
    // Volatile prevents folding the offline-patched digest to its zero initializer.
    // No process mutation of readonly code/data is admitted by this protocol.
    let payload = unsafe { ptr::read_volatile(address) };
    if payload[..32] != MARKER || payload[32..].iter().all(|byte| *byte == 0) {
        return Err(Error::Unsealed);
    }
    let module = main_module()?;
    let mut containing = ptr::null_mut();
    // SAFETY: FROM_ADDRESS treats this valid static pointer as an address, not a
    // string; initialized output has exact HMODULE size. UNCHANGED_REFCOUNT means
    // no acquired ownership and neither handle is freed or returned to callers.
    let found = unsafe { GetModuleHandleExW(4 | 2, address.cast(), &mut containing) };
    if found == 0 || containing != module {
        return Err(Error::Module);
    }
    let rva = (address as usize)
        .checked_sub(module as usize)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| value.checked_add(PAYLOAD_BYTES as u32).is_some())
        .ok_or(Error::Module)?;
    Ok(Snapshot {
        payload,
        payload_rva: rva,
    })
}

fn main_module() -> Result<*mut c_void, Error> {
    // SAFETY: NULL requests the main executable, without transferring ownership.
    let module = unsafe { GetModuleHandleW(ptr::null()) };
    if module.is_null() {
        Err(Error::Module)
    } else {
        Ok(module)
    }
}

pub(super) fn module_path(limits: Limits) -> Result<PathBuf, Error> {
    let module = main_module()?;
    let maximum = limits.max_path_units.checked_add(1).ok_or(Error::Limit)?;
    let mut capacity = maximum.min(260);
    loop {
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation)?;
        buffer.resize(capacity, 0u16);
        // SAFETY: initialized live output has `capacity` u16 slots; conversion is
        // bounded by validated path limits. Win32 does not retain the buffer.
        let count =
            unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), capacity as u32) } as usize;
        if count == 0 {
            return Err(Error::Module);
        }
        if count >= capacity {
            if capacity == maximum {
                return Err(Error::Limit);
            }
            capacity = capacity.saturating_mul(2).min(maximum);
            continue;
        }
        if buffer[count] != 0 || count > limits.max_path_units {
            return Err(Error::Path);
        }
        let mut text = String::new();
        text.try_reserve_exact(count.checked_mul(3).ok_or(Error::Limit)?)
            .map_err(|_| Error::Allocation)?;
        for value in char::decode_utf16(buffer[..count].iter().copied()) {
            text.push(value.map_err(|_| Error::Path)?);
        }
        // Validate the exact API spelling. Never normalize or strip a verbatim
        // prefix: GetModuleFileNameW is permitted to return unsupported formats.
        let admitted =
            crate::path::AbsolutePath::parse(&text, limits).map_err(super::platform_error)?;
        drop(admitted);
        let mut path = PathBuf::new();
        path.try_reserve_exact(text.len())
            .map_err(|_| Error::Allocation)?;
        path.push(text);
        return Ok(path);
    }
}
