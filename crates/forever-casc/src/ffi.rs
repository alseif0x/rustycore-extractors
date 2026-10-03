// SPDX-License-Identifier: GPL-3.0-or-later
//! Only unsafe boundary: single-threaded owned native storage+file handle.
use crate::{Error, Locale, admit};
use std::{
    ffi::{CString, c_char, c_void},
    marker::PhantomData,
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

unsafe extern "C" {
    fn rce_casc_abi() -> u32;
    fn rce_casc_open(
        path: *const c_char,
        locale: u32,
        id: u32,
        handle: *mut *mut c_void,
        size: *mut u64,
    ) -> u32;
    fn rce_casc_read(handle: *mut c_void, buffer: *mut u8, length: u32, read: *mut u32) -> u32;
    fn rce_casc_close(handle: *mut c_void);
}

fn status(code: u32) -> Result<(), Error> {
    match code {
        0 => Ok(()),
        2 => Err(Error::UnsupportedClient),
        4 => Err(Error::Encrypted),
        5 => Err(Error::MissingFile),
        7 => Err(Error::Incomplete),
        _ => Err(Error::Native),
    }
}

pub struct CascFile {
    handle: NonNull<c_void>,
    size: u64,
    // CascLib handles/error state are not promoted to concurrent ownership.
    _single_thread: PhantomData<Rc<()>>,
}
impl CascFile {
    pub fn open(client: &Path, locale: Locale, id: u32) -> Result<Self, Error> {
        if id == 0 || id == u32::MAX {
            return Err(Error::InvalidId);
        }
        admit(client)?;
        let path = CString::new(client.as_os_str().as_bytes()).map_err(|_| Error::Input)?;
        let mut handle = std::ptr::null_mut();
        let mut size = 0;
        // SAFETY: ABI version is checked; path is NUL-terminated and alive;
        // writable out-parameters are valid. Native code checks target and
        // returns ownership of exactly one storage+file object only on success.
        unsafe {
            if rce_casc_abi() != 1 {
                return Err(Error::Native);
            }
            status(rce_casc_open(
                path.as_ptr(),
                locale.mask(),
                id,
                &mut handle,
                &mut size,
            ))?;
        }
        Ok(Self {
            handle: NonNull::new(handle).ok_or(Error::Native)?,
            size,
            _single_thread: PhantomData,
        })
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    /// Complete single read. A missing key/incomplete block is an error, never
    /// zeros or successful partial bytes. The caller supplies a size budget.
    pub fn read_all(self, max_bytes: u32) -> Result<Vec<u8>, Error> {
        if self.size > u64::from(max_bytes) {
            return Err(Error::Limit);
        }
        let length = self.size as u32;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length as usize)
            .map_err(|_| Error::Limit)?;
        bytes.resize(length as usize, 0);
        let mut read = 0;
        // SAFETY: unique owned valid handle, buffer has exactly length initialized
        // bytes, valid out-parameter. Native callback cannot access Rust state.
        unsafe {
            status(rce_casc_read(
                self.handle.as_ptr(),
                bytes.as_mut_ptr(),
                length,
                &mut read,
            ))?;
        }
        if read != length {
            return Err(Error::Incomplete);
        }
        // No usable prefix is exposed on native failure/short read.
        Ok(bytes)
    }
}
impl Drop for CascFile {
    fn drop(&mut self) {
        // SAFETY: handle has one owner, is never shared or manually closed; no
        // borrowed file/storage escapes. Native close is noexcept and consumes it.
        unsafe {
            rce_casc_close(self.handle.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abi_and_status_codes_are_checked() {
        assert_eq!(unsafe { rce_casc_abi() }, 1);
        assert_eq!(status(4), Err(Error::Encrypted));
        assert_eq!(status(5), Err(Error::MissingFile));
        assert_eq!(status(7), Err(Error::Incomplete));
        assert_eq!(status(99), Err(Error::Native));
    }
    #[test]
    fn invalid_native_arguments_clear_outputs_and_do_not_open_storage() {
        let mut handle = std::ptr::dangling_mut::<c_void>();
        let mut size = 123;
        // SAFETY: native v1 explicitly admits null input as an error; valid
        // out-parameters are provided and no opaque handle is dereferenced.
        assert_eq!(
            unsafe { rce_casc_open(std::ptr::null(), 2, 1, &mut handle, &mut size) },
            1
        );
        assert!(handle.is_null());
        assert_eq!(size, 0);
        let mut count = 0;
        assert_eq!(
            unsafe { rce_casc_read(std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut count) },
            1
        );
    }
}
