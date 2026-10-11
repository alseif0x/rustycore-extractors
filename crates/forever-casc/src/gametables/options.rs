// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline-only GT arguments. Parsing performs no I/O or build admission.
use super::batch::Plan;
use crate::{Locale, options::CliError};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Typed(Plan),
    Catalog,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub client: PathBuf,
    pub output: PathBuf,
    pub locale: Locale,
    pub acquisition: crate::acquisition::Selection,
    pub selection: Selection,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Help,
    Run(Options),
}

pub fn parse(args: &[OsString]) -> Result<Parsed, CliError> {
    if args == [OsString::from("--help")] {
        return Ok(Parsed::Help);
    }
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err(CliError::INVALID_INVOCATION);
    }
    let mut values = BTreeMap::new();
    let mut acquisition = crate::acquisition::OptionsBuilder::default();
    for pair in args.as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or(CliError::INVALID_INVOCATION)?;
        if acquisition.push(key, &pair[1])? {
            continue;
        }
        if !matches!(key, "--client" | "--output" | "--locale" | "--group")
            || pair[1].is_empty()
            || values.insert(key, &pair[1]).is_some()
        {
            return Err(CliError::INVALID_INVOCATION);
        }
    }
    let required = |key| values.get(key).copied().ok_or(CliError::INVALID_INVOCATION);
    let locale = required("--locale")?
        .to_str()
        .ok_or(CliError::UNSUPPORTED_LOCALE)?;
    let selection = match values.get("--group") {
        None => Selection::Typed(Plan::All),
        Some(value) => match value.to_str().ok_or(CliError::INVALID_INVOCATION)? {
            "catalog" => Selection::Catalog,
            name => Selection::Typed(Plan::named(name).ok_or(CliError::INVALID_INVOCATION)?),
        },
    };
    let acquisition = acquisition.finish(super::MAX_FILE_BYTES as u32)?;
    Ok(Parsed::Run(Options {
        acquisition,
        client: PathBuf::from(required("--client")?),
        output: PathBuf::from(required("--output")?),
        locale: Locale::parse(locale).map_err(|_| CliError::UNSUPPORTED_LOCALE)?,
        selection,
    }))
}

pub fn verify_output(args: &[OsString]) -> Result<PathBuf, CliError> {
    match args {
        [flag, value] if flag == "--output" && !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(CliError::INVALID_INVOCATION),
    }
}
