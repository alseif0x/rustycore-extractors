// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: synchronous borrowed-directory native space observation.
use super::Error;
use std::{
    ffi::c_void,
    fs::File,
    mem::{align_of, offset_of, size_of},
    os::windows::io::AsRawHandle,
};
#[repr(C)]
struct Io {
    status: usize,
    information: usize,
}
#[repr(C)]
#[derive(Default)]
pub(super) struct Counts {
    pub total: i64,
    pub caller: i64,
    pub actual: i64,
    pub sectors: u32,
    pub bytes_per_sector: u32,
}
const _: () = {
    assert!(size_of::<Io>() == 16 && align_of::<Io>() == 8 && offset_of!(Io, information) == 8);
    assert!(
        size_of::<Counts>() == 32 && align_of::<Counts>() == 8 && offset_of!(Counts, sectors) == 24
    );
};
#[link(name = "ntdll")]
unsafe extern "system" {
    #[allow(clashing_extern_declarations)]
    fn NtQueryVolumeInformationFile(
        file: *mut c_void,
        io: *mut Io,
        information: *mut c_void,
        length: u32,
        class: i32,
    ) -> i32;
}
pub(super) fn query(file: &File) -> Result<Counts, Error> {
    let mut counts = Counts::default();
    let mut io = Io {
        status: usize::MAX,
        information: usize::MAX,
    };
    // SAFETY: FILE_SYNCHRONOUS_IO_NONALERT borrowed directory and aligned fully
    // initialized x64 FILE_FS_FULL_SIZE_INFORMATION/IO_STATUS_BLOCK outputs.
    // The call is synchronous; no pointer or handle ownership escapes.
    let status = unsafe {
        NtQueryVolumeInformationFile(
            file.as_raw_handle(),
            &mut io,
            (&mut counts as *mut Counts).cast(),
            size_of::<Counts>() as u32,
            7,
        )
    };
    if status != 0 || io.status as u32 != 0 || io.information != size_of::<Counts>() {
        Err(Error::Unavailable)
    } else {
        Ok(counts)
    }
}
