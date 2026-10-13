// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: separate scalable package NTFS declarations.
// Shares the reviewed v1 security representation; v1 implementation is unchanged.
// SAFETY: Raw pointers are confined here. All calls are synchronous, all
// borrowed File handles remain alive, and owned native handles are closed once.
use super::Error;
use std::{
    ffi::c_void,
    fs::File,
    mem::{align_of, offset_of, size_of, size_of_val},
    os::windows::io::{AsRawHandle, FromRawHandle},
    ptr::{null, null_mut},
};
type Handle = *mut c_void;
const FULL: u32 = 0x001f_01ff;
#[repr(C)]
#[derive(PartialEq, Eq)]
pub(super) struct Sid {
    bytes: [u8; 68],
    len: usize,
}
#[repr(C)]
struct Unicode {
    len: u16,
    max: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct Attributes {
    len: u32,
    root: Handle,
    name: *mut Unicode,
    flags: u32,
    security: *mut c_void,
    qos: *mut c_void,
}
#[repr(C)]
#[derive(Default)]
struct Io {
    status: usize,
    information: usize,
}
#[repr(C)]
struct Descriptor {
    revision: u8,
    sbz: u8,
    control: u16,
    owner: Handle,
    group: Handle,
    sacl: Handle,
    dacl: Handle,
}
#[repr(C)]
struct TokenUser {
    sid: Handle,
    attributes: u32,
}
#[repr(C)]
#[derive(Default)]
struct AclInfo {
    count: u32,
    used: u32,
    free: u32,
}
#[repr(C)]
struct Ace {
    kind: u8,
    flags: u8,
    size: u16,
    mask: u32,
}
#[repr(C)]
struct Rename {
    replace: u8,
    pad: [u8; 7],
    root: Handle,
    len: u32,
    name: [u16; 255],
}
const _: () = {
    assert!(
        size_of::<Unicode>() == 16
            && align_of::<Unicode>() == 8
            && offset_of!(Unicode, buffer) == 8
    );
    assert!(
        size_of::<Attributes>() == 48
            && offset_of!(Attributes, root) == 8
            && offset_of!(Attributes, security) == 32
    );
    assert!(size_of::<Io>() == 16 && align_of::<Io>() == 8);
    assert!(size_of::<Descriptor>() == 40 && offset_of!(Descriptor, owner) == 8);
    assert!(size_of::<TokenUser>() == 16);
    assert!(size_of::<AclInfo>() == 12);
    assert!(size_of::<Ace>() == 8);
    assert!(
        size_of::<Rename>() == 536
            && offset_of!(Rename, root) == 8
            && offset_of!(Rename, len) == 16
            && offset_of!(Rename, name) == 20
    );
};
#[link(name = "ntdll")]
unsafe extern "system" {
    #[cfg(test)]
    fn NtSetSecurityObject(handle: Handle, information: u32, descriptor: Handle) -> i32;
    fn NtCreateFile(
        handle: *mut Handle,
        access: u32,
        attributes: *mut Attributes,
        io: *mut Io,
        allocation: *const i64,
        flags: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea: Handle,
        ea_len: u32,
    ) -> i32;
    fn NtSetInformationFile(handle: Handle, io: *mut Io, data: Handle, len: u32, class: i32)
    -> i32;
    fn NtQueryInformationFile(
        handle: Handle,
        io: *mut Io,
        data: Handle,
        len: u32,
        class: i32,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: Handle) -> i32;
    fn GetCurrentProcess() -> Handle;
    fn GetCurrentThread() -> Handle;
    fn GetLastError() -> u32;
    fn LocalFree(memory: Handle) -> Handle;
    fn FlushFileBuffers(handle: Handle) -> i32;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    #[cfg(test)]
    fn ImpersonateSelf(level: i32) -> i32;
    #[cfg(test)]
    fn RevertToSelf() -> i32;

    fn OpenThreadToken(thread: Handle, access: u32, as_self: i32, token: *mut Handle) -> i32;
    fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
    fn GetTokenInformation(
        token: Handle,
        class: i32,
        information: Handle,
        len: u32,
        needed: *mut u32,
    ) -> i32;
    fn InitializeAcl(acl: Handle, len: u32, revision: u32) -> i32;
    fn AddAccessAllowedAceEx(acl: Handle, revision: u32, flags: u32, mask: u32, sid: Handle)
    -> i32;
    fn InitializeSecurityDescriptor(descriptor: Handle, revision: u32) -> i32;
    fn SetSecurityDescriptorOwner(descriptor: Handle, owner: Handle, defaulted: i32) -> i32;
    fn SetSecurityDescriptorDacl(
        descriptor: Handle,
        present: i32,
        acl: Handle,
        defaulted: i32,
    ) -> i32;
    fn SetSecurityDescriptorControl(descriptor: Handle, interest: u16, set: u16) -> i32;
    fn GetSecurityInfo(
        handle: Handle,
        kind: i32,
        information: u32,
        owner: *mut Handle,
        group: *mut Handle,
        dacl: *mut Handle,
        sacl: *mut Handle,
        descriptor: *mut Handle,
    ) -> u32;
    fn IsValidSecurityDescriptor(descriptor: Handle) -> i32;
    fn GetSecurityDescriptorLength(descriptor: Handle) -> u32;
    fn GetSecurityDescriptorControl(
        descriptor: Handle,
        control: *mut u16,
        revision: *mut u32,
    ) -> i32;
    fn GetSecurityDescriptorOwner(
        descriptor: Handle,
        owner: *mut Handle,
        defaulted: *mut i32,
    ) -> i32;
    fn GetSecurityDescriptorDacl(
        descriptor: Handle,
        present: *mut i32,
        acl: *mut Handle,
        defaulted: *mut i32,
    ) -> i32;
    fn IsValidAcl(acl: Handle) -> i32;
    fn GetAclInformation(acl: Handle, information: Handle, len: u32, class: i32) -> i32;
    fn GetAce(acl: Handle, index: u32, ace: *mut Handle) -> i32;
}
struct Owned(Handle);
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 as isize != -1 {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}
struct Security(Handle);
impl Drop for Security {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn bounded(pointer: Handle, len: usize, base: Handle, total: usize) -> bool {
    (pointer as usize)
        .checked_sub(base as usize)
        .and_then(|start| start.checked_add(len))
        .is_some_and(|end| end <= total)
}
// Reads only after proving the fixed header lies in the API-owned buffer, then
// validates the variable SID extent before constructing a byte slice.
unsafe fn sid_bytes<'a>(pointer: Handle, base: Handle, total: usize) -> Result<&'a [u8], Error> {
    if !bounded(pointer, 8, base, total) {
        return Err(Error::Privacy);
    }
    let p = pointer.cast::<u8>();
    let revision = unsafe { *p };
    let count = unsafe { *p.add(1) };
    let len = 8 + usize::from(count) * 4;
    if revision != 1 || count > 15 || !bounded(pointer, len, base, total) {
        return Err(Error::Privacy);
    }
    Ok(unsafe { std::slice::from_raw_parts(p, len) })
}
pub(super) fn process_sid() -> Result<Sid, Error> {
    let mut token = null_mut();
    // No impersonation, including same-SID impersonation, is admitted.
    let thread = unsafe { OpenThreadToken(GetCurrentThread(), 8, 1, &mut token) };
    if thread != 0 {
        drop(Owned(token));
        return Err(Error::Impersonation);
    }
    if unsafe { GetLastError() } != 1008 {
        return Err(Error::Token);
    }
    if unsafe { OpenProcessToken(GetCurrentProcess(), 8, &mut token) } == 0 {
        return Err(Error::Token);
    }
    let token = Owned(token);
    let mut buffer = [0_usize; 32];
    let base = buffer.as_mut_ptr().cast();
    let mut needed = 0;
    if unsafe { GetTokenInformation(token.0, 1, base, size_of_val(&buffer) as u32, &mut needed) }
        == 0
        || needed as usize > size_of_val(&buffer)
        || (needed as usize) < size_of::<TokenUser>()
    {
        return Err(Error::Token);
    }
    let user = unsafe { &*base.cast::<TokenUser>() };
    let bytes = unsafe { sid_bytes(user.sid, base, needed as usize) }.map_err(|_| Error::Token)?;
    let mut sid = Sid {
        bytes: [0; 68],
        len: bytes.len(),
    };
    sid.bytes[..bytes.len()].copy_from_slice(bytes);
    Ok(sid)
}
fn acl(acl: Handle, sid: &Sid, mask: u32) -> Result<(), Error> {
    if unsafe { InitializeAcl(acl, 96, 2) } == 0
        || unsafe { AddAccessAllowedAceEx(acl, 2, 0, mask, sid.bytes.as_ptr().cast_mut().cast()) }
            == 0
    {
        return Err(Error::Privacy);
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Directory,
    Read,
    Modify,
    Lock,
}
pub(super) fn create(parent: &File, name: &str, directory: bool, sid: &Sid) -> Result<File, Error> {
    let mut storage = [0_u32; 24];
    let acl_pointer = storage.as_mut_ptr().cast();
    acl(acl_pointer, sid, FULL)?;
    let mut descriptor = Descriptor {
        revision: 0,
        sbz: 0,
        control: 0,
        owner: null_mut(),
        group: null_mut(),
        sacl: null_mut(),
        dacl: null_mut(),
    };
    let sd = (&mut descriptor as *mut Descriptor).cast();
    if unsafe { InitializeSecurityDescriptor(sd, 1) } == 0
        || unsafe { SetSecurityDescriptorOwner(sd, sid.bytes.as_ptr().cast_mut().cast(), 0) } == 0
        || unsafe { SetSecurityDescriptorDacl(sd, 1, acl_pointer, 0) } == 0
        || unsafe { SetSecurityDescriptorControl(sd, 0x1000, 0x1000) } == 0
    {
        return Err(Error::Privacy);
    }
    native_open(
        parent,
        name,
        if directory {
            Mode::Directory
        } else {
            Mode::Modify
        },
        true,
        sd,
    )
}
pub(super) fn open(parent: &File, name: &str, mode: Mode) -> Result<File, Error> {
    native_open(parent, name, mode, false, null_mut())
}
fn native_open(
    parent: &File,
    name: &str,
    mode: Mode,
    create: bool,
    security: Handle,
) -> Result<File, Error> {
    let count = name.encode_utf16().count();
    if count == 0 || count > 255 || name.contains(['\\', '/']) {
        return Err(Error::Path);
    }
    let mut wide = [0_u16; 255];
    for (to, from) in wide.iter_mut().zip(name.encode_utf16()) {
        *to = from;
    }
    let mut unicode = Unicode {
        len: (count * 2) as u16,
        max: (count * 2) as u16,
        buffer: wide.as_mut_ptr(),
    };
    let mut attributes = Attributes {
        len: size_of::<Attributes>() as u32,
        root: parent.as_raw_handle(),
        name: &mut unicode,
        flags: 0x40,
        security,
        qos: null_mut(),
    };
    let mut io = Io::default();
    let mut raw = null_mut();
    let directory = matches!(mode, Mode::Directory);
    // Directory leases deny deletion but allow writes by the same owner; all
    // identities/privacy are checked before/after use. File read/modify leases
    // deny competing writes and deletes. Lock handles permit other lock opens.
    let (access, share) = match mode {
        Mode::Directory => (0x0012_00a1, 3),
        Mode::Read => (0x0012_0081, 1),
        Mode::Modify => (0x0013_0083, 1),
        Mode::Lock => (0x0012_0083, 3),
    };
    let status = unsafe {
        NtCreateFile(
            &mut raw,
            access,
            &mut attributes,
            &mut io,
            null(),
            if directory { 0x10 } else { 0x80 },
            share,
            if create { 2 } else { 1 },
            0x0020_0020 | if directory { 1 } else { 0x40 },
            null_mut(),
            0,
        )
    };
    let owned = Owned(raw);
    if status != 0 || io.status as u32 != 0 {
        return Err(match status as u32 {
            0xc000_0035 => Error::Exists,
            0xc000_0034 | 0xc000_003a => Error::Missing,
            0xc000_0043 => Error::Busy,
            _ => Error::Open,
        });
    }
    if raw.is_null() || raw as isize == -1 || io.information != if create { 2 } else { 1 } {
        return Err(Error::Identity);
    }
    std::mem::forget(owned);
    Ok(unsafe { File::from_raw_handle(raw) })
}
/// FileNameInformation reports the filesystem's stored spelling; unlike a
/// pathname comparison this rejects case aliases after the relative open.
pub(super) fn exact_name(file: &File, expected: &str) -> Result<(), Error> {
    let mut storage = [0_u64; 8192];
    let mut io = Io::default();
    let status = unsafe {
        NtQueryInformationFile(
            file.as_raw_handle(),
            &mut io,
            storage.as_mut_ptr().cast(),
            size_of_val(&storage) as u32,
            9,
        )
    };
    if status != 0
        || io.status as u32 != 0
        || io.information < 6
        || io.information > size_of_val(&storage)
    {
        return Err(Error::Identity);
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), io.information) };
    let length = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| Error::Identity)?) as usize;
    if length == 0 || !length.is_multiple_of(2) || length.checked_add(4) != Some(io.information) {
        return Err(Error::Identity);
    }
    let units = bytes[4..].as_chunks::<2>().0;
    let start = units
        .iter()
        .rposition(|v| u16::from_le_bytes(*v) == 92)
        .map_or(0, |v| v + 1);
    if !units[start..]
        .iter()
        .map(|v| u16::from_le_bytes(*v))
        .eq(expected.encode_utf16())
    {
        return Err(Error::Alias);
    }
    Ok(())
}
pub(super) fn remove(file: &File) -> Result<(), Error> {
    let mut delete = 1_u8;
    let mut io = Io::default();
    let status = unsafe {
        NtSetInformationFile(
            file.as_raw_handle(),
            &mut io,
            (&mut delete as *mut u8).cast(),
            1,
            13,
        )
    };
    if status != 0 || io.status as u32 != 0 {
        return Err(Error::Remove);
    }
    Ok(())
}
pub(super) fn ordinary_directory(file: &File) -> Result<(), Error> {
    let mut flags = 0_u32;
    let mut io = Io::default();
    let status = unsafe {
        NtQueryInformationFile(
            file.as_raw_handle(),
            &mut io,
            (&mut flags as *mut u32).cast(),
            4,
            71,
        )
    };
    if status != 0 || io.status as u32 != 0 || io.information != 4 || flags != 0 {
        return Err(Error::Unsupported);
    }
    Ok(())
}
pub(super) fn flush(file: &File) -> Result<(), Error> {
    if unsafe { FlushFileBuffers(file.as_raw_handle()) } == 0 {
        Err(Error::Flush)
    } else {
        Ok(())
    }
}
pub(super) fn rename(file: &File, destination: &File, name: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 255 || !name.is_ascii() {
        return Err(Error::Path);
    }
    let mut data = Rename {
        replace: 0,
        pad: [0; 7],
        root: destination.as_raw_handle(),
        len: (name.len() * 2) as u32,
        name: [0; 255],
    };
    for (to, from) in data.name.iter_mut().zip(name.bytes()) {
        *to = u16::from(from)
    }
    let mut io = Io::default();
    let status = unsafe {
        NtSetInformationFile(
            file.as_raw_handle(),
            &mut io,
            (&mut data as *mut Rename).cast(),
            (offset_of!(Rename, name) + name.len() * 2) as u32,
            10,
        )
    };
    if status != 0 || io.status as u32 != 0 {
        return Err(if status as u32 == 0xc000_0035 {
            Error::Exists
        } else {
            Error::Rename
        });
    }
    Ok(())
}
pub(super) fn private(file: &File, sid: &Sid) -> Result<(), Error> {
    let mut raw = null_mut();
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            1,
            5,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut raw,
        )
    };
    let sd = Security(raw);
    if status != 0 || raw.is_null() || unsafe { IsValidSecurityDescriptor(raw) } == 0 {
        return Err(Error::Privacy);
    }
    let len = unsafe { GetSecurityDescriptorLength(raw) } as usize;
    if !(40..=4096).contains(&len) {
        return Err(Error::Privacy);
    }
    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(raw, &mut control, &mut revision) } == 0
        || revision != 1
        || control & 0x1004 != 0x1004
        || control & (1 | 8 | 0x100 | 0x400) != 0
    {
        return Err(Error::Privacy);
    }
    let mut owner = null_mut();
    let mut defaulted = 0;
    if unsafe { GetSecurityDescriptorOwner(raw, &mut owner, &mut defaulted) } == 0
        || defaulted != 0
        || unsafe { sid_bytes(owner, raw, len) }? != &sid.bytes[..sid.len]
    {
        return Err(Error::Privacy);
    }
    let mut present = 0;
    let mut acl_pointer = null_mut();
    if unsafe { GetSecurityDescriptorDacl(raw, &mut present, &mut acl_pointer, &mut defaulted) }
        == 0
        || present == 0
        || defaulted != 0
        || acl_pointer.is_null()
        || !bounded(acl_pointer, 8, raw, len)
        || unsafe { IsValidAcl(acl_pointer) } == 0
    {
        return Err(Error::Privacy);
    }
    if unsafe { *acl_pointer.cast::<u8>() } != 2 {
        return Err(Error::Privacy);
    }
    let mut info = AclInfo::default();
    if unsafe {
        GetAclInformation(
            acl_pointer,
            (&mut info as *mut AclInfo).cast(),
            size_of::<AclInfo>() as u32,
            2,
        )
    } == 0
        || info.count != 1
        || info.used as usize != 16 + sid.len
        || !bounded(acl_pointer, info.used as usize, raw, len)
    {
        return Err(Error::Privacy);
    }
    let mut ace = null_mut();
    if unsafe { GetAce(acl_pointer, 0, &mut ace) } == 0 || !bounded(ace, 8 + sid.len, raw, len) {
        return Err(Error::Privacy);
    }
    let header = unsafe { &*ace.cast::<Ace>() };
    if header.kind != 0
        || header.flags != 0
        || header.mask != FULL
        || header.size as usize != 8 + sid.len
    {
        return Err(Error::Privacy);
    }
    if unsafe { sid_bytes(ace.cast::<u8>().add(8).cast(), raw, len) }? != &sid.bytes[..sid.len] {
        return Err(Error::Privacy);
    }
    drop(sd);
    Ok(())
}
#[cfg(test)]
pub(super) fn test_dacl(file: &File, sid: &Sid, mask: u32) -> Result<(), u32> {
    // Avoid SetSecurityInfo's automatic inheritance processing when restoring
    // the profile's exact no-auto-inheritance descriptor. The native user-mode
    // setter accepts this exact absolute SD; the test checks restored control.
    let mut storage = [0_u32; 24];
    let pointer = storage.as_mut_ptr().cast();
    acl(pointer, sid, mask).map_err(|_| u32::MAX)?;
    let mut descriptor = Descriptor {
        revision: 0,
        sbz: 0,
        control: 0,
        owner: null_mut(),
        group: null_mut(),
        sacl: null_mut(),
        dacl: null_mut(),
    };
    let sd = (&mut descriptor as *mut Descriptor).cast();
    if unsafe { InitializeSecurityDescriptor(sd, 1) } == 0
        || unsafe { SetSecurityDescriptorDacl(sd, 1, pointer, 0) } == 0
        || unsafe { SetSecurityDescriptorControl(sd, 0x1500, 0x1000) } == 0
    {
        return Err(unsafe { GetLastError() });
    }
    // Only the DACL is changed. The existing owner SID is never replaced.
    let status = unsafe { NtSetSecurityObject(file.as_raw_handle(), 4, sd) };
    if status != 0 {
        return Err(status as u32);
    }
    Ok(())
}
#[cfg(test)]
pub(super) fn test_security_control(file: &File) -> Result<u16, u32> {
    let mut raw = null_mut();
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            1,
            5,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut raw,
        )
    };
    let _owned = Security(raw);
    if status != 0 {
        return Err(status);
    }
    if raw.is_null() {
        return Err(u32::MAX);
    }
    let mut control = 0;
    let mut revision = 0;
    if unsafe { GetSecurityDescriptorControl(raw, &mut control, &mut revision) } == 0 {
        return Err(unsafe { GetLastError() });
    }
    Ok(control)
}

#[cfg(test)]
pub(super) struct Impersonation(std::marker::PhantomData<std::rc::Rc<()>>);
#[cfg(test)]
impl Drop for Impersonation {
    fn drop(&mut self) {
        // The non-Send guard must revert on this same OS thread, including unwind.
        // A failed revert cannot safely allow the native test process to continue.
        if unsafe { RevertToSelf() } == 0 {
            std::process::abort();
        }
    }
}
#[cfg(test)]
pub(super) fn test_impersonate() -> Result<Impersonation, Error> {
    if unsafe { ImpersonateSelf(2) } == 0 {
        return Err(Error::Impersonation);
    }
    Ok(Impersonation(std::marker::PhantomData))
}
