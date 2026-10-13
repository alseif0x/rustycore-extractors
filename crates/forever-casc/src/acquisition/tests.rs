// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic selection/provenance/session tests only.
use super::*;
use crate::{options::CliError, verify::json};
use std::ffi::OsStr;
const CAP: u32 = 16 * 1024 * 1024;
fn select(pairs: &[(&str, &str)], cap: u32) -> Result<Selection, CliError> {
    let mut builder = OptionsBuilder::default();
    for &(key, value) in pairs {
        assert!(builder.push(key, OsStr::new(value))?);
    }
    builder.finish(cap)
}
fn explicit() -> Vec<(&'static str, &'static str)> {
    vec![
        ("--backend", "rust-local"),
        ("--index-layout", "contiguous"),
        ("--root-layout", "header-50893-v2"),
    ]
}
fn profile() -> Profile {
    select(&explicit(), CAP).unwrap().profile().unwrap()
}

#[test]
fn native_is_unchanged_default_without_any_provenance() {
    for pairs in [&[][..], &[("--backend", "native")][..]] {
        let choice = select(pairs, CAP).unwrap();
        assert_eq!(choice, Selection::Native);
        assert_eq!(choice.profile(), None);
        assert!(!choice.is_rust_local());
    }
}
#[test]
fn explicit_layouts_are_required_only_for_rust_local() {
    for pairs in [
        vec![("--backend", "rust-local")],
        vec![("--index-layout", "contiguous")],
        vec![("--root-layout", "header-50893-v2")],
        vec![("--backend", "native"), ("--max-file-bytes", "1")],
    ] {
        assert_eq!(select(&pairs, CAP), Err(CliError::INVALID_INVOCATION));
    }
    for index in ["contiguous", "paged"] {
        for root in [
            "headerless-18125",
            "header-30080",
            "header-50893-v1",
            "header-50893-v2",
        ] {
            assert!(
                select(
                    &[
                        ("--root-layout", root),
                        ("--backend", "rust-local"),
                        ("--index-layout", index)
                    ],
                    CAP
                )
                .unwrap()
                .is_rust_local()
            );
        }
    }
}
#[test]
fn common_options_reject_duplicates_empty_spelling_and_nondecimal_limits() {
    for (key, bad) in [
        ("--backend", "auto"),
        ("--index-layout", "Contiguous"),
        ("--root-layout", "auto"),
        ("--max-file-bytes", "0"),
        ("--max-file-bytes", "-1"),
        ("--max-file-bytes", "+1"),
        ("--max-file-bytes", "1.0"),
        ("--max-file-bytes", "4294967296"),
        ("--backend", ""),
    ] {
        assert!(
            OptionsBuilder::default()
                .push(key, OsStr::new(bad))
                .is_err()
        );
    }
    for (key, value) in explicit() {
        let mut builder = OptionsBuilder::default();
        assert!(builder.push(key, OsStr::new(value)).unwrap());
        assert_eq!(
            builder.push(key, OsStr::new(value)),
            Err(CliError::INVALID_INVOCATION)
        );
    }
    assert!(
        !OptionsBuilder::default()
            .push("--client", OsStr::new("synthetic"))
            .unwrap()
    );
}
#[test]
fn per_file_limit_only_tightens_the_component_ceiling() {
    for (value, valid) in [("1", true), ("4194304", true), ("4194305", false)] {
        let mut pairs = explicit();
        pairs.push(("--max-file-bytes", value));
        let result = select(&pairs, 4 * 1024 * 1024);
        assert_eq!(result.is_ok(), valid);
        if let Ok(choice) = result {
            assert_eq!(
                choice.profile().unwrap().max_file_bytes(),
                value.parse::<u32>().unwrap()
            );
        }
    }
    assert_eq!(select(&explicit(), 0), Err(CliError::INVALID_INVOCATION));
    assert_eq!(
        select(&explicit(), CAP + 1),
        Err(CliError::INVALID_INVOCATION)
    );
}
#[cfg(unix)]
#[test]
fn non_utf8_common_option_values_are_invalid() {
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(
        OptionsBuilder::default().push("--backend", OsStr::from_bytes(&[0xff])),
        Err(CliError::INVALID_INVOCATION)
    );
}
#[test]
fn profile_roundtrip_is_strict_and_independent_of_object_key_order() {
    let expected = profile();
    let text = expected.render();
    assert_eq!(Profile::parse(&text, CAP).unwrap(), expected);
    let mut parsed = json::parse(&text).unwrap();
    let json::Json::Object(fields) = &mut parsed else {
        unreachable!()
    };
    fields.reverse();
    let (_, json::Json::Object(limits)) = fields
        .iter_mut()
        .find(|(name, _)| name == "limits")
        .unwrap()
    else {
        unreachable!()
    };
    limits.reverse();
    assert_eq!(Profile::decode(&parsed, CAP), Ok(expected));
    assert!(Profile::parse(&text, 4 * 1024 * 1024).is_err());
}
#[test]
fn profile_rejects_unknown_missing_duplicate_and_weakened_properties() {
    let text = profile().render();
    for changed in [
        text.replacen("\"backend\":\"rust-local\"", "\"backend\":\"native\"", 1),
        text.replace("\"offline\":true", "\"offline\":false"),
        text.replace("\"complete_ckey_md5\":true", "\"complete_ckey_md5\":false"),
        text.replace(
            "\"ekey_validation\":\"prefix9\"",
            "\"ekey_validation\":\"full\"",
        ),
        text.replace(
            "\"resident_bytes\":805306368",
            "\"resident_bytes\":805306369",
        ),
        text.replace("\"file_bytes\":16777216", "\"file_bytes\":0"),
        text.replace("\"index_files\":4096,", ""),
        text.replace("\"backend\":", "\"extra\":1,\"backend\":"),
        text.replace("\"backend\":", "\"backend\":\"rust-local\",\"backend\":"),
    ] {
        assert!(Profile::parse(&changed, CAP).is_err(), "{changed}");
    }
}
#[test]
fn profile_rejects_mixed_index_layouts_and_any_unbounded_json_shape() {
    let text = profile().render();
    assert!(Profile::parse(&text.replacen("\"contiguous\"", "\"paged\"", 1), CAP).is_err());
    for text in [
        "[".repeat(10000),
        " ".repeat(8193),
        "[[[[[0]]]]]".to_owned(),
        "null".to_owned(),
    ] {
        assert!(Profile::parse(&text, CAP).is_err());
    }
}
#[test]
fn profile_decoder_defends_against_manually_constructed_duplicate_json_keys() {
    let mut parsed = json::parse(&profile().render()).unwrap();
    let json::Json::Object(fields) = &mut parsed else {
        unreachable!()
    };
    let (_, json::Json::Object(limits)) = fields
        .iter_mut()
        .find(|(name, _)| name == "limits")
        .unwrap()
    else {
        unreachable!()
    };
    limits[1] = limits[0].clone();
    assert!(Profile::decode(&parsed, CAP).is_err());
}
#[test]
fn session_poison_prevents_later_backend_reads_after_input_change() {
    let mut session = Session {
        backend: Backend::Scripted { calls: 0 },
        build: 70205,
        profile: Some(profile()),
        changed: false,
    };
    for _ in 0..3 {
        assert!(session.read(42, 1).unwrap_err().is_input_changed());
    }
    let Backend::Scripted { calls } = session.backend else {
        unreachable!()
    };
    assert_eq!(calls, 1);
}
#[test]
fn local_errors_keep_specific_codes_limits_and_fatal_change_identity() {
    for error in [
        local_store::Error::Limit,
        local_store::Error::Encoding(crate::encoding::Error::Limit),
        local_store::Error::Index(crate::local_index::Error::Limit),
        local_store::Error::Config(crate::build_config::Error::Limit),
        local_store::Error::Root(crate::root::Error::Limit),
    ] {
        assert_eq!(Error::LocalStore(error).exit_code(), 6);
    }
    assert_eq!(
        Error::LocalStore(local_store::Error::UnsupportedClient).exit_code(),
        3
    );
    assert_eq!(Error::BackendUnavailable.exit_code(), 2);
    assert!(Error::LocalStore(local_store::Error::InputChanged).is_input_changed());
    assert!(!Error::Incomplete.is_input_changed());
    assert_eq!(
        Error::LocalStore(local_store::Error::NoLocalEncoding(
            local_store::CandidateFailures {
                encrypted: 1,
                ..local_store::CandidateFailures::default()
            }
        ))
        .code(),
        "local_store_no_verified_encoding"
    );
}
#[cfg(not(feature = "native"))]
#[test]
fn unavailable_native_fails_before_trying_an_absent_client() {
    assert_eq!(
        Selection::Native.ensure_available().unwrap_err().code,
        "backend_unavailable"
    );
    assert_eq!(
        Session::open(
            Path::new("/does-not-exist-acquisition-test"),
            Locale::EnUs,
            Selection::Native
        )
        .err(),
        Some(Error::BackendUnavailable)
    );
    assert!(select(&explicit(), CAP).unwrap().ensure_available().is_ok());
}
#[test]
fn shared_schema_has_the_same_closed_keys_and_default_bounds() {
    let schema = json::parse(include_str!(
        "../../../../schemas/forever-acquisition-v1.schema.json"
    ))
    .unwrap();
    let json::Json::Object(root) = schema else {
        unreachable!()
    };
    let find = |name: &str| {
        root.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap()
    };
    assert_eq!(find("additionalProperties"), &json::Json::Bool(false));
    let json::Json::Object(props) = find("properties") else {
        unreachable!()
    };
    let json::Json::Object(profile_json) = json::parse(&profile().render()).unwrap() else {
        unreachable!()
    };
    assert_eq!(props.len(), profile_json.len());
    for (name, value) in profile_json {
        assert!(props.iter().any(|(key, _)| key == &name));
        if name == "limits" {
            let (_, json::Json::Object(rule)) =
                props.iter().find(|(key, _)| key == "limits").unwrap()
            else {
                unreachable!()
            };
            let (_, json::Json::Object(properties)) =
                rule.iter().find(|(key, _)| key == "properties").unwrap()
            else {
                unreachable!()
            };
            let json::Json::Object(values) = value else {
                unreachable!()
            };
            assert_eq!(properties.len(), values.len());
            for (field, value) in values {
                let (_, json::Json::Object(rules)) =
                    properties.iter().find(|(key, _)| key == &field).unwrap()
                else {
                    unreachable!()
                };
                if field != "file_bytes" {
                    assert_eq!(
                        &rules.iter().find(|(key, _)| key == "const").unwrap().1,
                        &value
                    );
                }
            }
        }
    }
}
