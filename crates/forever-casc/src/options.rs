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
    /// Explicit immutable metadata catalog opt-in; never enabled by default.
    pub schema_catalog: Option<&'static str>,
    pub acquisition: crate::acquisition::Selection,
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
    let mut acquisition = crate::acquisition::OptionsBuilder::default();
    for pair in args.as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or(CliError::INVALID_INVOCATION)?;
        if acquisition.push(key, &pair[1])? {
            continue;
        }
        if !matches!(
            key,
            "--client" | "--locale" | "--output" | "--manifest" | "--schema-catalog"
        ) || pair[1].is_empty()
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
    let schema_catalog = match values.get("--schema-catalog") {
        Some(value) if manifest.is_some() => match value.to_str() {
            Some(forever_db2::catalog::CATALOG_ID) => Some(forever_db2::catalog::CATALOG_ID),
            Some(forever_db2::catalog::WOWDBDEFS_70205_CATALOG_ID) => {
                Some(forever_db2::catalog::WOWDBDEFS_70205_CATALOG_ID)
            }
            _ => return Err(CliError::INVALID_INVOCATION),
        },
        Some(_) => return Err(CliError::INVALID_INVOCATION),
        None => None,
    };
    Ok(Parsed::Run(Options {
        client,
        output,
        locale,
        manifest,
        schema_catalog,
        acquisition: acquisition.finish(crate::batch::MAX_FILE_BYTES)?,
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
    fn catalog_selection_is_exact_and_requires_explicit_manifest() {
        let base = ["--client", "c", "--locale", "enUS", "--output", "o"];
        for extra in [
            vec!["--schema-catalog", "pinned-metadata-v1"],
            vec!["--manifest", "m", "--schema-catalog", "unknown"],
            vec!["--manifest", "m", "--schema-catalog", "PINNED-METADATA-V1"],
            vec![
                "--manifest",
                "m",
                "--schema-catalog",
                "pinned-metadata-v1",
                "--schema-catalog",
                "pinned-metadata-v1",
            ],
        ] {
            assert_eq!(
                run(&[base.to_vec(), extra].concat()),
                Err(CliError::INVALID_INVOCATION)
            );
        }
        let Parsed::Run(options) = run(&[
            base.to_vec(),
            vec!["--manifest", "m", "--schema-catalog", "pinned-metadata-v1"],
        ]
        .concat())
        .unwrap() else {
            panic!("run expected")
        };
        assert_eq!(options.schema_catalog, Some("pinned-metadata-v1"));
        let Parsed::Run(default) = run(&base).unwrap() else {
            panic!("run expected")
        };
        assert_eq!(default.schema_catalog, None);
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

    #[test]
    fn acquisition_defaults_native_and_rust_requires_explicit_layouts() {
        let base = ["--client", "c", "--locale", "enUS", "--output", "o"];
        let Parsed::Run(native) = run(&base).unwrap() else {
            panic!("run");
        };
        assert_eq!(native.acquisition, crate::acquisition::Selection::Native);
        for root in [
            "headerless-18125",
            "header-30080",
            "header-50893-v1",
            "header-50893-v2",
        ] {
            for index in ["contiguous", "paged"] {
                let values = [
                    base.to_vec(),
                    vec![
                        "--backend",
                        "rust-local",
                        "--index-layout",
                        index,
                        "--root-layout",
                        root,
                        "--max-file-bytes",
                        "1024",
                    ],
                ]
                .concat();
                let Parsed::Run(options) = run(&values).unwrap() else {
                    panic!("run");
                };
                assert_eq!(
                    options.acquisition.profile().unwrap().max_file_bytes(),
                    1024
                );
                assert_eq!(options.schema_catalog, None);
            }
        }
    }

    #[test]
    fn acquisition_rejects_incomplete_ambiguous_or_enlarged_policy() {
        let base = ["--client", "c", "--locale", "enUS", "--output", "o"];
        for extra in [
            vec!["--backend", "other"],
            vec!["--backend", "rust-local"],
            vec!["--index-layout", "paged"],
            vec!["--root-layout", "header-30080"],
            vec!["--max-file-bytes", "1"],
            vec!["--backend", "native", "--max-file-bytes", "1"],
            vec!["--backend", "native", "--backend", "native"],
            vec![
                "--backend",
                "rust-local",
                "--index-layout",
                "paged",
                "--root-layout",
                "header-30080",
                "--max-file-bytes",
                "16777217",
            ],
            vec![
                "--backend",
                "rust-local",
                "--index-layout",
                "paged",
                "--root-layout",
                "header-30080",
                "--max-file-bytes",
                "0",
            ],
        ] {
            assert_eq!(
                run(&[base.to_vec(), extra].concat()),
                Err(CliError::INVALID_INVOCATION)
            );
        }
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
