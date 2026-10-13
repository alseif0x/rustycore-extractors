// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original reviewed GNU/Linux x86_64 lease FFI boundary.
use super::Error;
use std::ffi::c_int;
const F_GETOWN: c_int = 9;
const F_SETOWN: c_int = 8;
const F_SETOWN_EX: c_int = 15;
const F_SETLEASE: c_int = 1024;
const F_GETLEASE: c_int = 1025;
const F_WRLCK: c_int = 1;
const F_UNLCK: c_int = 2;
const SIGIO: c_int = 29;
const SIG_BLOCK: c_int = 0;
#[repr(C)]
struct Owner {
    kind: c_int,
    tid: c_int,
}
#[repr(C)]
struct SigSet {
    words: [u64; 16],
}
const _: () = assert!(size_of::<SigSet>() == 128 && align_of::<SigSet>() == 8);
const _: () = assert!(size_of::<Owner>() == 8 && align_of::<Owner>() == 4);
unsafe extern "C" {
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
    fn gettid() -> c_int;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigpending(set: *mut SigSet) -> c_int;
    fn sigismember(set: *const SigSet, signal: c_int) -> c_int;
    fn pthread_sigmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
}
struct Setup {
    fd: c_int,
    keep: bool,
}
impl Drop for Setup {
    fn drop(&mut self) {
        // SAFETY: borrowed File outlives the scoped helper; integer commands
        // accept c_int arguments. No pointer or ownership transfer occurs.
        unsafe {
            if !self.keep {
                fcntl(self.fd, F_SETLEASE, F_UNLCK);
            }
            fcntl(self.fd, F_SETOWN, 0 as c_int);
        }
    }
}
pub(super) fn exclusive_without_owner(fd: c_int) -> bool {
    // SAFETY: fd is borrowed from a live File, both commands take no third
    // argument. A concurrent lease break can only make this check stricter.
    unsafe { fcntl(fd, F_GETLEASE) == F_WRLCK && fcntl(fd, F_GETOWN) == 0 }
}
pub(super) fn establish(fd: c_int) -> Result<(), Error> {
    establish_with_hook(fd, || {})
}
pub(super) fn establish_with_hook(fd: c_int, hook: impl FnOnce()) -> Result<(), Error> {
    let mut blocked = SigSet { words: [0; 16] };
    // SAFETY: SigSet has the GNU x86_64 ABI admitted by the enclosing cfg;
    // pointers are valid, writable, aligned, and live through each libc call.
    // pthread_sigmask only changes this short-lived helper, never its caller.
    unsafe {
        if sigemptyset(&mut blocked) != 0
            || sigaddset(&mut blocked, SIGIO) != 0
            || pthread_sigmask(SIG_BLOCK, &blocked, std::ptr::null_mut()) != 0
        {
            return Err(Error::Unavailable);
        }
        if fcntl(fd, F_GETOWN) != 0 || fcntl(fd, F_GETLEASE) != F_UNLCK {
            return Err(Error::Unavailable);
        }
        let owner = Owner {
            kind: 0,
            tid: gettid(),
        };
        if owner.tid <= 0 || fcntl(fd, F_SETOWN_EX, &owner as *const Owner) != 0 {
            return Err(Error::Unavailable);
        }
        let mut setup = Setup { fd, keep: false };
        if fcntl(fd, F_SETLEASE, F_WRLCK) != 0 {
            return Err(Error::Unavailable);
        }
        hook();
        let mut pending = SigSet { words: [0; 16] };
        if sigpending(&mut pending) != 0 || sigismember(&pending, SIGIO) != 0 {
            return Err(Error::Unavailable);
        }
        if fcntl(fd, F_SETOWN, 0 as c_int) != 0 || !exclusive_without_owner(fd) {
            return Err(Error::Unavailable);
        }
        setup.keep = true;
        // Do not restore the helper's signal mask. Targeted pending SIGIO is
        // discarded when this helper exits; no process handler is installed.
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn caller_mask() -> Result<[u64; 16], Error> {
    let mut mask = SigSet { words: [0; 16] };
    // SAFETY: null set asks pthread_sigmask to query only; output has the
    // admitted GNU ABI and remains valid for the duration of this call.
    if unsafe { pthread_sigmask(SIG_BLOCK, std::ptr::null(), &mut mask) } != 0 {
        Err(Error::Unavailable)
    } else {
        Ok(mask.words)
    }
}
