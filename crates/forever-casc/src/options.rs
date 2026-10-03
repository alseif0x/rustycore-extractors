// SPDX-License-Identifier: GPL-3.0-or-later
use forever_casc::Locale;
use std::{ffi::OsString, path::PathBuf};
pub struct Options {
    pub client: PathBuf,
    pub locale: Locale,
    pub id: u32,
    pub max: u32,
    pub compare: Option<PathBuf>,
}
pub fn parse(args: Vec<OsString>) -> Result<Option<Options>, ()> {
    if args.len() == 1 && args[0] == "--help" {
        return Ok(None);
    }
    let mut client = None;
    let mut locale = None;
    let mut id = None;
    let mut max = None;
    let mut compare = None;
    let mut json = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--json" && !json {
            json = true;
            continue;
        }
        let value = args.next().ok_or(())?;
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err(());
        }
        match arg.to_str().ok_or(())? {
            "--client" if client.is_none() => client = Some(PathBuf::from(value)),
            "--locale" if locale.is_none() => {
                locale = Some(Locale::parse(value.to_str().ok_or(())?).map_err(|_| ())?)
            }
            "--file-id" if id.is_none() => {
                id = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--max-bytes" if max.is_none() => {
                max = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--compare" if compare.is_none() => compare = Some(PathBuf::from(value)),
            _ => return Err(()),
        }
    }
    let id = id.ok_or(())?;
    let max = max.unwrap_or(128 * 1024 * 1024);
    if id == 0 || id == u32::MAX || max == 0 || max > 128 * 1024 * 1024 {
        return Err(());
    }
    Ok(Some(Options {
        client: client.ok_or(())?,
        locale: locale.ok_or(())?,
        id,
        max,
        compare,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &str) -> Vec<OsString> {
        s.split_whitespace().map(OsString::from).collect()
    }
    #[test]
    fn explicit_input_locale_id_and_budget_required_or_bounded() {
        let good = "--client local --locale esES --file-id 1361031";
        assert!(parse(args(good)).is_ok());
        for bad in [
            "",
            "--client local",
            "--client x --locale zzZZ --file-id 1",
            "--client x --locale enUS --file-id 0",
            "--client x --locale enUS --file-id 4294967295",
            "--client x --locale enUS --file-id 1 --max-bytes 0",
            "--client x --locale enUS --file-id 1 --max-bytes 134217729",
            "--client x --locale enUS --file-id 1 --json --json",
            "--client x --locale enUS --file-id 1 --client y",
        ] {
            assert!(parse(args(bad)).is_err(), "{bad}");
        }
    }
}
