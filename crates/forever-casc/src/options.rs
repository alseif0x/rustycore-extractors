// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure argument parsing for `forever-db2-extract`. It performs no I/O, no
//! build admission and no client access, so it is unit-testable without a
//! client or the native feature. Errors carry only static, redacted codes.
use crate::Locale;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub client: PathBuf,
    pub output: PathBuf,
    pub locale: Locale,
    pub manifest: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Help,
    Run(Options),
}

/// Static code plus process exit. Never contains an argument value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliError {
    pub code: &'static str,
    pub exit: u8,
}

impl CliError {
    pub const INVALID_INVOCATION: Self = Self {
        code: "invalid_invocation",
        exit: 2,
    };
    pub const UNSUPPORTED_LOCALE: Self = Self {
        code: "unsupported_locale",
        exit: 2,
    };

    pub const fn new(code: &'static str, exit: u8) -> Self {
        Self { code, exit }
    }
}

/// Parses the CLI contract: `--help` alone, or even argument/key-value pairs
/// drawn from the fixed option set with no duplicates and no empty values.
pub fn parse(args: &[OsString]) -> Result<Parsed, CliError> {
    if args == [OsString::from("--help")] {
        return Ok(Parsed::Help);
    }
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err(CliError::INVALID_INVOCATION);
    }
    let mut values: BTreeMap<&str, &OsString> = BTreeMap::new();
    for pair in args.as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or(CliError::INVALID_INVOCATION)?;
        if !matches!(key, "--client" | "--locale" | "--output" | "--manifest")
            || pair[1].is_empty()
            || values.insert(key, &pair[1]).is_some()
        {
            return Err(CliError::INVALID_INVOCATION);
        }
    }
    let client = PathBuf::from(values.get("--client").ok_or(CliError::INVALID_INVOCATION)?);
    let output = PathBuf::from(values.get("--output").ok_or(CliError::INVALID_INVOCATION)?);
    let locale = Locale::parse(
        values
            .get("--locale")
            .and_then(|value| value.to_str())
            .ok_or(CliError::INVALID_INVOCATION)?,
    )
    .map_err(|_| CliError::UNSUPPORTED_LOCALE)?;
    let manifest = values.get("--manifest").map(PathBuf::from);
    Ok(Parsed::Run(Options {
        client,
        output,
        locale,
        manifest,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn run(values: &[&str]) -> Result<Parsed, CliError> {
        parse(&args(values))
    }

    #[test]
    fn help_is_standalone_and_exclusive() {
        assert_eq!(run(&["--help"]), Ok(Parsed::Help));
        for bad in [
            &["--help", "--json"][..],
            &["--help", "--client", "x"][..],
            &[][..],
        ] {
            assert_eq!(run(bad), Err(CliError::INVALID_INVOCATION));
        }
    }

    #[test]
    fn odd_missing_unknown_and_duplicate_arguments_reject() {
        for bad in [
            &["--client"][..],
            &["--client", "x", "--locale"][..],
            &["--client", "x", "--locale", "enUS"][..],
            &["--private-option", "secret"][..],
            &[
                "--client", "x", "--locale", "enUS", "--client", "y", "--output", "z",
            ][..],
            &["--client", "", "--locale", "enUS", "--output", "z"][..],
        ] {
            assert_eq!(run(bad), Err(CliError::INVALID_INVOCATION), "{bad:?}");
        }
    }

    #[test]
    fn selects_locale_and_optional_manifest() {
        let parsed = run(&[
            "--client",
            "c",
            "--locale",
            "esES",
            "--output",
            "o",
            "--manifest",
            "m",
        ])
        .unwrap();
        match parsed {
            Parsed::Run(options) => {
                assert_eq!(options.locale, Locale::EsEs);
                assert_eq!(options.client, PathBuf::from("c"));
                assert_eq!(options.output, PathBuf::from("o"));
                assert_eq!(options.manifest, Some(PathBuf::from("m")));
            }
            Parsed::Help => panic!("expected run"),
        }
        assert_eq!(
            run(&["--client", "c", "--locale", "enUS", "--output", "o"])
                .unwrap()
                .locale_name(),
            "enUS"
        );
    }

    #[test]
    fn invalid_locale_uses_unsupported_locale_exit_two() {
        assert_eq!(
            run(&["--client", "c", "--locale", "zzZZ", "--output", "o"]),
            Err(CliError::UNSUPPORTED_LOCALE)
        );
        assert_eq!(CliError::UNSUPPORTED_LOCALE.exit, 2);
    }

    #[test]
    fn errors_are_redacted_and_exit_codes_match_the_taxonomy() {
        let error = run(&["--client", "/private/secret", "--locale", "enUS"]).unwrap_err();
        assert_eq!(error.code, "invalid_invocation");
        assert_eq!(error.exit, 2);
        assert!(!format!("{error:?}").contains("/private/secret"));
        // Admission/IO taxonomy used by the binary after parsing.
        assert_eq!(Error::UnsupportedClient.code(), "unsupported_client");
        assert_eq!(Error::UnsupportedClient.exit_code(), 3);
        assert_eq!(Error::Input.exit_code(), 4);
        assert_eq!(Error::InvalidLocale.code(), "unsupported_locale");
        assert_eq!(Error::InvalidLocale.exit_code(), 2);
        assert_eq!(error, CliError::new("invalid_invocation", 2));
    }

    impl Parsed {
        fn locale_name(&self) -> &'static str {
            match self {
                Self::Run(options) => options.locale.name(),
                Self::Help => "help",
            }
        }
    }
}
