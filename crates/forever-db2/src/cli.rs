// SPDX-License-Identifier: GPL-3.0-or-later
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Invocation {
    pub file: PathBuf,
    pub json: bool,
}

pub fn parse(args: Vec<OsString>) -> Result<Option<Invocation>, ()> {
    if args.len() == 1 && args[0] == "--help" {
        return Ok(None);
    }
    let mut file = None;
    let mut json = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--file" && file.is_none() {
            let value = args.next().ok_or(())?;
            if value.is_empty() || value.to_string_lossy().starts_with("--") {
                return Err(());
            }
            file = Some(PathBuf::from(value));
        } else if arg == "--json" && !json {
            json = true;
        } else {
            return Err(());
        }
    }
    Ok(Some(Invocation {
        file: file.ok_or(())?,
        json,
    }))
}
