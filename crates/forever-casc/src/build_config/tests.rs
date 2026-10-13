// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic build-config parser tests.

use super::*;

const ROOT: &str = "00112233445566778899aabbccddeeff";
const ENCODING: &str = "ffeeddccbbaa99887766554433221100";
const EKEY: &str = "0123456789abcdeffedcba9876543210";
const MINIMAL: &[u8] = b"root = 00112233445566778899aabbccddeeff\nencoding = ffeeddccbbaa99887766554433221100 0123456789abcdeffedcba9876543210\n";
// Independent golden: Python hashlib.md5 over the exact 117 synthetic bytes.
const MINIMAL_MD5: [u8; 16] = [
    0x2d, 0x52, 0xa9, 0xa9, 0xa0, 0x09, 0xec, 0x8a, 0x25, 0x8f, 0x31, 0x51, 0x98, 0xc8, 0xbd, 0x14,
];
const ROOT_BYTES: [u8; 16] = [
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
];
const ENCODING_BYTES: [u8; 16] = [
    0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00,
];
const EKEY_BYTES: [u8; 16] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
];
const LIMITS: Limits = Limits {
    max_bytes: 16 * 1024,
    max_lines: 100,
    max_keys: 50,
};

fn checked(bytes: &[u8]) -> Result<BuildConfig<'_>, Error> {
    parse(bytes, md5::digest(bytes), LIMITS)
}

fn with_lines(lines: &str) -> Vec<u8> {
    let mut bytes = MINIMAL.to_vec();
    bytes.extend_from_slice(lines.as_bytes());
    bytes
}

#[test]
fn independent_minimal_golden_retains_full_identifiers() {
    assert_eq!(MINIMAL.len(), 117);
    let config = parse(MINIMAL, MINIMAL_MD5, LIMITS).unwrap();
    assert_eq!(
        config.root,
        ContentReference {
            content_key: ROOT_BYTES,
            encoded_key: None,
            content_size: None,
            encoded_size: None,
        }
    );
    assert_eq!(
        config.encoding,
        ContentReference {
            content_key: ENCODING_BYTES,
            encoded_key: Some(EKEY_BYTES),
            content_size: None,
            encoded_size: None,
        }
    );
    assert_eq!(config.build_uid, None);
    assert_eq!(config.build_name, None);
    assert_eq!(config.ignored_keys, 0);
}

#[test]
fn optional_root_ekey_and_sizes_are_order_independent() {
    let input = format!(
        "encoding-size = 456 123\nroot-size = 91 84\nencoding = {ENCODING} {EKEY}\nroot = {ROOT} {ENCODING}\n"
    );
    let config = checked(input.as_bytes()).unwrap();
    assert_eq!(config.root.encoded_key, Some(ENCODING_BYTES));
    assert_eq!(config.root.content_size, Some(91));
    assert_eq!(config.root.encoded_size, Some(84));
    assert_eq!(config.encoding.content_size, Some(456));
    assert_eq!(config.encoding.encoded_size, Some(123));
}

#[test]
fn absent_unknown_zero_and_largest_known_sizes_remain_distinct() {
    for (suffix, root_content, root_encoded) in [
        ("", None, None),
        ("root-size = 0\n", Some(0), None),
        ("root-size = 0 0\n", Some(0), Some(0)),
        ("root-size = 4294967295\n", None, None),
        ("root-size = 4294967295 0\n", None, Some(0)),
        ("root-size = 0 4294967295\n", Some(0), None),
        (
            "root-size = 4294967294 4294967294\n",
            Some(u32::MAX - 1),
            Some(u32::MAX - 1),
        ),
    ] {
        let input = with_lines(suffix);
        let config = checked(&input).unwrap();
        assert_eq!(config.root.content_size, root_content, "{suffix}");
        assert_eq!(config.root.encoded_size, root_encoded, "{suffix}");
    }
    let input = with_lines("encoding-size = 4294967295 4294967294\n");
    let config = checked(&input).unwrap();
    assert_eq!(config.encoding.content_size, None);
    assert_eq!(config.encoding.encoded_size, Some(u32::MAX - 1));
}

#[test]
fn sizes_reject_overflow_signs_non_decimal_and_extra_tokens() {
    for name in ["root-size", "encoding-size"] {
        for value in [
            "4294967296",
            "1 4294967296",
            "-1",
            "+1",
            "1.0",
            "0x10",
            "1x",
            "1 2 3",
            "1 # comment",
        ] {
            let input = with_lines(&format!("{name} = {value}\n"));
            assert_eq!(checked(&input), Err(Error::Sizes), "{name}: {value}");
        }
    }
}

#[test]
fn metadata_strings_are_borrowed_without_product_or_build_inference() {
    let input = with_lines("build-uid = unknown_product\nbuild-name = 99999_unknown_1.2.3_ñ\n");
    let config = checked(&input).unwrap();
    assert_eq!(config.build_uid, Some("unknown_product"));
    assert_eq!(config.build_name, Some("99999_unknown_1.2.3_ñ"));
    let uid_offset = input
        .windows(15)
        .position(|part| part == b"unknown_product")
        .unwrap();
    assert_eq!(
        config.build_uid.unwrap().as_ptr(),
        input[uid_offset..].as_ptr()
    );
}

#[test]
fn relevant_names_and_hex_accept_ascii_case_aliases() {
    let input = format!(
        "ROOT = {}\nEnCoDiNg = {} {}\nROOT-SIZE = 0007 0008\nBUILD-UID = wow\nBUILD-NAME = beta\n",
        ROOT.to_ascii_uppercase(),
        ENCODING.to_ascii_uppercase(),
        EKEY.to_ascii_uppercase()
    );
    let config = checked(input.as_bytes()).unwrap();
    assert_eq!(config.root.content_key, ROOT_BYTES);
    assert_eq!(config.encoding.content_key, ENCODING_BYTES);
    assert_eq!(config.encoding.encoded_key, Some(EKEY_BYTES));
    assert_eq!(config.root.content_size, Some(7));
    assert_eq!(config.root.encoded_size, Some(8));
    assert_eq!(config.build_uid, Some("wow"));
}

#[test]
fn duplicate_relevant_fields_reject_even_identical_or_case_aliased() {
    for (name, value) in [
        ("root", ROOT),
        ("encoding", &format!("{ENCODING} {EKEY}")),
        ("root-size", "1 2"),
        ("encoding-size", "1 2"),
        ("build-uid", "wow"),
        ("build-name", "synthetic"),
    ] {
        let prefix = if matches!(name, "root" | "encoding") {
            String::new()
        } else {
            format!("{name} = {value}\n")
        };
        for second_name in [name.to_string(), name.to_ascii_uppercase()] {
            let input = with_lines(&format!("{prefix}{second_name} = {value}\n"));
            assert_eq!(checked(&input), Err(Error::Duplicate), "{name}");
        }
    }
    let input = with_lines(&format!("root = {ENCODING}\n"));
    assert_eq!(checked(&input), Err(Error::Duplicate));
}

#[test]
fn hash_lengths_and_hex_alphabet_are_exact() {
    for invalid in [
        "00112233445566778899aabbccddeef",
        "00112233445566778899aabbccddeeff0",
        "00112233445566778899aabbccddeefg",
        "00112233445566778899aabbccddeefé",
        "0x112233445566778899aabbccddeeff",
    ] {
        for input in [
            format!("root = {invalid}\nencoding = {ENCODING} {EKEY}\n"),
            format!("root = {ROOT} {invalid}\nencoding = {ENCODING} {EKEY}\n"),
            format!("root = {ROOT}\nencoding = {invalid} {EKEY}\n"),
            format!("root = {ROOT}\nencoding = {ENCODING} {invalid}\n"),
        ] {
            assert_eq!(checked(input.as_bytes()), Err(Error::Hashes), "{invalid}");
        }
    }
}

#[test]
fn root_accepts_one_or_two_hashes_and_encoding_requires_exactly_two() {
    for input in [
        format!("root = {ROOT} {EKEY} {ENCODING}\nencoding = {ENCODING} {EKEY}\n"),
        format!("root = {ROOT}\nencoding = {ENCODING}\n"),
        format!("root = {ROOT}\nencoding = {ENCODING} {EKEY} {ROOT}\n"),
        format!("root = {ROOT} # comment\nencoding = {ENCODING} {EKEY}\n"),
    ] {
        assert_eq!(checked(input.as_bytes()), Err(Error::Hashes));
    }
    // A 16-byte all-zero identifier is parsed faithfully, never used to skip MD5.
    let input = format!("root = {}\nencoding = {ENCODING} {EKEY}\n", "0".repeat(32));
    assert_eq!(checked(input.as_bytes()).unwrap().root.content_key, [0; 16]);
}

#[test]
fn both_required_records_must_exist() {
    for input in [
        String::new(),
        "# comments only\n\n".to_owned(),
        format!("root = {ROOT}\n"),
        format!("encoding = {ENCODING} {EKEY}\n"),
        "root-size = 7\nencoding-size = 9\n".to_owned(),
    ] {
        assert_eq!(checked(input.as_bytes()), Err(Error::Missing));
    }
}

#[test]
fn unknown_assignments_are_counted_without_reinterpreting_source_tags() {
    let input = with_lines(
        "root-config = not-a-root\nencoding-config = not-an-ekey\nsource-tags = unknown-source\nsource-tags = repeated\nroot-extra = ignored\nbuild-name-extra = ignored\nunknown-patch = = also ignored\n",
    );
    let config = checked(&input).unwrap();
    assert_eq!(config.ignored_keys, 7);
    assert_eq!(config.root.content_key, ROOT_BYTES);
    assert_eq!(config.root.encoded_key, None);
    assert_eq!(config.encoding.encoded_key, Some(EKEY_BYTES));
    assert_eq!(config.build_name, None);
}

#[test]
fn wildcard_lookalikes_cannot_supply_required_records() {
    let input = format!("root-config = {ROOT}\nencoding = {ENCODING} {EKEY}\n");
    assert_eq!(checked(input.as_bytes()), Err(Error::Missing));
    let input = format!("root = {ROOT}\nencoding-config = {ENCODING} {EKEY}\n");
    assert_eq!(checked(input.as_bytes()), Err(Error::Missing));
}

#[test]
fn whitespace_and_whole_line_comments_have_declared_semantics() {
    let input = format!(
        "\r\n \t# comment\r\n\troot\t=\t{ROOT}\t\r\n encoding = {ENCODING}\t{EKEY} \n\t\n build-name = # literal metadata value \r\n"
    );
    let config = checked(input.as_bytes()).unwrap();
    assert_eq!(config.root.content_key, ROOT_BYTES);
    assert_eq!(config.build_name, Some("# literal metadata value"));
    assert_eq!(config.ignored_keys, 0);
    let no_newline = &MINIMAL[..MINIMAL.len() - 1];
    assert_eq!(checked(no_newline).unwrap(), checked(MINIMAL).unwrap());
}

#[test]
fn checksum_binds_every_byte_and_has_no_zero_digest_bypass() {
    assert_eq!(parse(MINIMAL, [0; 16], LIMITS), Err(Error::Checksum));
    for suffix in ["\n", " ", "# new comment\n", "source = extra\n"] {
        let input = with_lines(suffix);
        assert_eq!(parse(&input, MINIMAL_MD5, LIMITS), Err(Error::Checksum));
    }
    let mut changed = MINIMAL.to_vec();
    changed[7] = b'f';
    assert_eq!(parse(&changed, MINIMAL_MD5, LIMITS), Err(Error::Checksum));
}

#[test]
fn checksum_precedes_utf8_syntax_duplicates_and_line_limits() {
    for bytes in [
        b"\xff".as_slice(),
        b"malformed".as_slice(),
        with_lines(&format!("root = {ROOT}\n")).as_slice(),
    ] {
        assert_eq!(
            parse(
                bytes,
                [0; 16],
                Limits {
                    max_lines: 0,
                    ..LIMITS
                }
            ),
            Err(Error::Checksum)
        );
    }
    assert_eq!(
        parse(
            MINIMAL,
            [0; 16],
            Limits {
                max_bytes: 0,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
}

#[test]
fn utf8_and_control_character_rejections_are_structured() {
    let mut invalid_utf8 = MINIMAL.to_vec();
    invalid_utf8.extend_from_slice(b"unknown = \xff\n");
    assert_eq!(checked(&invalid_utf8), Err(Error::Text));
    for control in [0, 1, 11, 12, 13, 31, 127] {
        let mut input = MINIMAL.to_vec();
        input.extend_from_slice(b"unknown = a");
        input.push(control);
        input.extend_from_slice(b"b\n");
        assert_eq!(checked(&input), Err(Error::Text), "control {control}");
    }
}

#[test]
fn assignment_syntax_is_checked_for_known_and_unknown_names() {
    for suffix in [
        "root\n",
        "unknown\n",
        "= value\n",
        "root = \n",
        "unknown =\n",
        "bad name = value\n",
        "bad.name = value\n",
        "é = value\n",
        "root: value\n",
    ] {
        let input = with_lines(suffix);
        assert_eq!(checked(&input), Err(Error::Syntax), "{suffix}");
    }
    let name79 = "a".repeat(79);
    let input = with_lines(&format!("{name79} = ignored\n"));
    assert_eq!(checked(&input).unwrap().ignored_keys, 1);
    let input = with_lines(&format!("{name79}a = ignored\n"));
    assert_eq!(checked(&input), Err(Error::Syntax));
}

#[test]
fn exact_limits_accept_and_one_below_rejects() {
    let exact = Limits {
        max_bytes: 117,
        max_lines: 2,
        max_keys: 2,
    };
    assert!(parse(MINIMAL, MINIMAL_MD5, exact).is_ok());
    for limits in [
        Limits {
            max_bytes: 116,
            ..exact
        },
        Limits {
            max_lines: 1,
            ..exact
        },
        Limits {
            max_keys: 1,
            ..exact
        },
    ] {
        assert_eq!(parse(MINIMAL, MINIMAL_MD5, limits), Err(Error::Limit));
    }
}

#[test]
fn blank_comments_and_unknown_duplicates_consume_their_limits() {
    let input = with_lines("\n# comment\n");
    let digest = md5::digest(&input);
    assert!(
        parse(
            &input,
            digest,
            Limits {
                max_lines: 4,
                max_keys: 2,
                ..LIMITS
            }
        )
        .is_ok()
    );
    assert_eq!(
        parse(
            &input,
            digest,
            Limits {
                max_lines: 3,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    let input = with_lines("unknown = a\nunknown = b\n");
    let digest = md5::digest(&input);
    assert_eq!(
        parse(
            &input,
            digest,
            Limits {
                max_keys: 3,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(
        parse(
            &input,
            digest,
            Limits {
                max_keys: 4,
                ..LIMITS
            }
        )
        .unwrap()
        .ignored_keys,
        2
    );
}

#[test]
fn diagnostics_are_fixed_codes_without_metadata_values() {
    for (error, code) in [
        (Error::Limit, "build_config_policy_limit"),
        (Error::Checksum, "build_config_checksum_mismatch"),
        (Error::Text, "build_config_invalid_text"),
        (Error::Syntax, "build_config_invalid_assignment"),
        (Error::Duplicate, "build_config_duplicate_field"),
        (Error::Hashes, "build_config_invalid_hashes"),
        (Error::Sizes, "build_config_invalid_sizes"),
        (Error::Missing, "build_config_missing_field"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

#[test]
fn auxiliary_central_references_and_sizes_use_exact_tags() {
    let input = with_lines(&format!(
        "download = {ROOT} {EKEY}\ninstall = {ENCODING}\nsize = {ROOT}\nvfs-root = {ROOT}\nvfs-7-size = 12 9\nvfs-7 = {ENCODING} {EKEY}\ninstall-config = opaque\n"
    ));
    let config = checked(&input).unwrap();
    assert_eq!(config.auxiliary.len(), 5);
    assert_eq!(config.ignored_keys, 1);
    let vfs = config
        .auxiliary
        .iter()
        .find(|r| r.kind == AuxiliaryKind::Vfs(7))
        .unwrap();
    assert_eq!(vfs.content_key, Some(ENCODING_BYTES));
    assert_eq!(vfs.encoded_key, Some(EKEY_BYTES));
    assert_eq!(vfs.content_size, Some(12));
    assert_eq!(vfs.encoded_size, Some(9));
}

#[test]
fn patch_single_hash_is_ekey_only_but_two_hashes_keep_both() {
    let input = with_lines(&format!("patch = {EKEY}\n"));
    let config = checked(&input).unwrap();
    assert_eq!(config.auxiliary[0].content_key, None);
    assert_eq!(config.auxiliary[0].encoded_key, Some(EKEY_BYTES));
    let input = with_lines(&format!("patch = {ROOT} {EKEY}\n"));
    let config = checked(&input).unwrap();
    assert_eq!(config.auxiliary[0].content_key, Some(ROOT_BYTES));
    assert_eq!(config.auxiliary[0].encoded_key, Some(EKEY_BYTES));
}

#[test]
fn auxiliary_duplicates_include_case_and_numeric_vfs_aliases() {
    for lines in [
        format!("install = {ROOT}\nINSTALL = {ROOT}\n"),
        format!("vfs-1 = {ROOT}\nvfs-01 = {ROOT}\n"),
        "download-size = 1\nDOWNLOAD-SIZE = 2\n".to_owned(),
    ] {
        assert_eq!(checked(&with_lines(&lines)), Err(Error::Duplicate));
    }
}

#[test]
fn vfs_ids_are_bounded_integers_not_allocation_extents() {
    let input = with_lines(&format!("vfs-4294967295 = {ROOT}\n"));
    assert_eq!(checked(&input).unwrap().auxiliary.len(), 1);
    for name in ["vfs-0", "vfs-4294967296"] {
        assert_eq!(
            checked(&with_lines(&format!("{name} = {ROOT}\n"))),
            Err(Error::Syntax)
        );
    }
}

#[test]
fn auxiliary_size_only_does_not_create_a_content_key() {
    let input = with_lines("install-size = 0 4294967295\n");
    let config = checked(&input).unwrap();
    assert_eq!(config.auxiliary[0].content_key, None);
    assert_eq!(config.auxiliary[0].encoded_key, None);
    assert_eq!(config.auxiliary[0].content_size, Some(0));
}
