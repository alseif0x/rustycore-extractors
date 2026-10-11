// SPDX-License-Identifier: GPL-3.0-or-later
//! Original synthetic fixtures; no client assets or copied source code.
use super::*;
mod pipeline;

fn fixture(table: GameTable, labels: &[&str], cells: &[&str], newline: &str) -> Vec<u8> {
    assert_eq!(cells.len(), table.value_columns());
    let mut text = String::from("id");
    for column in 0..table.value_columns() {
        text.push_str(&format!("\tc{column}"));
    }
    text.push_str(newline);
    for label in labels {
        text.push_str(label);
        for value in cells {
            text.push('\t');
            text.push_str(value);
        }
        text.push_str(newline);
    }
    text.into_bytes()
}

fn accepted(bytes: &[u8], table: GameTable) -> ParsedTable {
    match parse(bytes, table) {
        Ok(parsed) => parsed,
        Err(error) => panic!("unexpected synthetic admission failure: {error}"),
    }
}

fn rejected(bytes: &[u8], table: GameTable, expected: Error) {
    assert_eq!(parse(bytes, table).err(), Some(expected));
}

#[test]
fn pinned_six_identities_paths_columns_and_groups() {
    let expected = [
        (GameTable::BaseMp, "BaseMp", "BaseMp.txt", 1391664, 15),
        (GameTable::HpPerSta, "HpPerSta", "HpPerSta.txt", 1391642, 1),
        (GameTable::Xp, "xp", "xp.txt", 1391661, 5),
        (
            GameTable::SpellScaling,
            "SpellScaling",
            "SpellScaling.txt",
            1391660,
            24,
        ),
        (
            GameTable::CombatRatingsMultByILvl,
            "CombatRatingsMultByILvl",
            "CombatRatingsMultByILvl.txt",
            1391670,
            4,
        ),
        (
            GameTable::StaminaMultByILvl,
            "StaminaMultByILvl",
            "StaminaMultByILvl.txt",
            1980632,
            4,
        ),
    ];
    assert_eq!(expected.map(|row| row.0), GameTable::ALL);
    for (table, name, file, id, columns) in expected {
        assert_eq!(table.name(), name);
        assert_eq!(table.file_name(), file);
        assert_eq!(table.file_id(), id);
        assert_eq!(table.value_columns(), columns);
        assert_eq!(table.relative_path(), format!("gt/{file}"));
        assert_eq!(GameTable::named(name), Some(table));
        assert_eq!(GameTable::named(file), None);
        assert!(table.group().tables().contains(&table));
    }
    assert_eq!(GameTable::named("Xp"), None);
    assert_eq!(GameTable::named("../xp"), None);
    assert_eq!(Group::Initial.tables(), GameTable::ALL[..3]);
    assert_eq!(Group::SpellValues.tables(), GameTable::ALL[3..]);
}

#[test]
fn both_three_table_groups_preserve_every_column_and_add_zero_row() {
    let samples = ["1.5", "-0", "2.25", "-8", "0", "6.25e-2"];
    let expected = [
        0x3fc0_0000,
        0x8000_0000,
        0x4010_0000,
        0xc100_0000,
        0,
        0x3d80_0000,
    ];
    for group in [Group::Initial, Group::SpellValues] {
        for table in group.tables() {
            let values: Vec<_> = (0..table.value_columns())
                .map(|i| samples[i % samples.len()])
                .collect();
            let golden: Vec<_> = (0..table.value_columns())
                .map(|i| expected[i % expected.len()])
                .collect();
            let bytes = fixture(table, &["1", "1", "999", "ignored"], &values, "\r\n");
            let untouched = bytes.clone();
            let parsed = accepted(&bytes, table);
            assert_eq!(bytes, untouched);
            assert_eq!(parsed.table(), table);
            assert_eq!(
                parsed.row(0),
                Some(vec![0; table.value_columns()].as_slice())
            );
            for index in 1..=4 {
                assert_eq!(parsed.row(index), Some(golden.as_slice()));
            }
            assert_eq!(parsed.row(5), None);
            assert_eq!(parsed.rows().len(), 5);
            assert_eq!(
                parsed.facts(),
                Facts {
                    bytes: bytes.len(),
                    value_columns: table.value_columns(),
                    data_rows: 4,
                    consumer_rows_including_zero: 5,
                    terminated_early: false,
                }
            );
        }
    }
}

#[test]
fn header_only_files_and_final_line_without_newline() {
    for table in GameTable::ALL {
        let cells = vec!["0"; table.value_columns()];
        let mut bytes = fixture(table, &[], &cells, "\n");
        let with_newline = accepted(&bytes, table);
        bytes.pop();
        let without_newline = accepted(&bytes, table);
        assert_eq!(with_newline.rows(), without_newline.rows());
        assert_eq!(without_newline.facts().consumer_rows_including_zero, 1);
        assert!(!without_newline.facts().terminated_early);

        let mut bytes = fixture(table, &["-999"], &cells, "\n");
        bytes.pop();
        assert_eq!(accepted(&bytes, table).facts().data_rows, 1);
    }
}

#[test]
fn header_token_count_ignores_empty_tokens_and_does_not_invent_column_names() {
    let table = GameTable::HpPerSta;
    let parsed = accepted(b"\t\tlabel\t\tother\t\nanything\t2\n", table);
    assert_eq!(parsed.row(1), Some([0x4000_0000].as_slice()));
    accepted("identificador\tvalor\nñ\t1\n".as_bytes(), table);
    rejected(b"", table, Error::Header);
    rejected(b"\n", table, Error::Header);
    rejected(b"id\n", table, Error::Header);
    rejected(b"id\tvalue\tother\n", table, Error::Header);
}

#[test]
fn first_column_is_ignored_even_when_empty_or_non_numeric() {
    let parsed = accepted(
        b"id\tv\n\t1\nnot-an-id\t2\n-100\t3\n-100\t4\n",
        GameTable::HpPerSta,
    );
    assert_eq!(parsed.facts().data_rows, 4);
    assert_eq!(parsed.row(4), Some([0x4080_0000].as_slice()));
}

#[test]
fn trailing_tabs_and_first_cr_follow_source_traversal() {
    let parsed = accepted(
        b"id\tv\r\n1\t1.5\t\t\rignored\n2\t-0\r\n",
        GameTable::HpPerSta,
    );
    assert_eq!(parsed.facts().data_rows, 2);
    assert_eq!(parsed.row(1), Some([0x3fc0_0000].as_slice()));
    assert_eq!(parsed.row(2), Some([0x8000_0000].as_slice()));
}

#[test]
fn blank_or_id_only_row_stops_without_decoding_remaining_text() {
    for stop in ["", "\t\t", "only-id", "only-id\t\t", "\rignored"] {
        let bytes = format!("id\tv\n1\t2\n{stop}\nignored\tNaN\n");
        let parsed = accepted(bytes.as_bytes(), GameTable::HpPerSta);
        assert_eq!(parsed.facts().data_rows, 1);
        assert!(parsed.facts().terminated_early);
        assert_eq!(parsed.facts().bytes, bytes.len());
    }
}

#[test]
fn malformed_rows_and_internal_empty_numeric_cells_fail() {
    let table = GameTable::Xp;
    let header = "id\ta\tb\tc\td\te\n";
    for row in ["1\t1\t2", "1\t1\t2\t3\t4\t5\t6"] {
        rejected(format!("{header}{row}\n").as_bytes(), table, Error::Row);
    }
    rejected(
        format!("{header}1\t1\t2\t\t4\t5\n").as_bytes(),
        table,
        Error::Numeric,
    );
    // Last empty cells disappear before the source field-count check.
    rejected(
        format!("{header}1\t1\t2\t3\t4\t\n").as_bytes(),
        table,
        Error::Row,
    );
}

#[test]
fn utf8_and_nul_checks_cover_even_unconsumed_tail() {
    rejected(b"id\tv\n1\t\xff\n", GameTable::HpPerSta, Error::Utf8);
    rejected(b"id\tv\n\n\xff", GameTable::HpPerSta, Error::Utf8);
    rejected(b"id\tv\n1\t1\0\n", GameTable::HpPerSta, Error::Text);
    rejected(b"id\tv\n\n\0", GameTable::HpPerSta, Error::Text);
}

#[test]
fn numeric_decimal_admission_preserves_bits_and_signed_zero() {
    for (cell, bits) in [
        ("0", 0),
        ("-0", 0x8000_0000),
        ("+0.0e20", 0),
        ("-.0E-20", 0x8000_0000),
        (".5", 0x3f00_0000),
        ("1.", 0x3f80_0000),
        ("  +1.25", 0x3fa0_0000),
        ("\u{c}-2.5E+1", 0xc1c8_0000),
        ("1.401298464324817e-45", 1),
        ("3.4028234663852886e38", 0x7f7f_ffff),
    ] {
        assert_eq!(decimal_bits(cell), Ok(bits));
    }
}

#[test]
fn numeric_failures_are_errors_and_never_zero_repairs() {
    for cell in [
        "",
        " ",
        "+",
        "-",
        ".",
        "1e",
        "1e+",
        "1e-",
        "1e1.5",
        "+-1",
        "--1",
        "++1",
        "NaN",
        "nan",
        "inf",
        "-inf",
        "Infinity",
        "0x1p2",
        "-0x1p2",
        "  0X1p0",
        "1_000",
        "1,5",
        "1junk",
        "1 ",
        "1\u{b}",
        "\u{b}-2.5E+1",
        "\u{a0}1",
        "１",
        "1e9999",
        "1e-9999",
        "1e-46",
        "3.5e38",
        "-3.5e38",
    ] {
        assert_eq!(decimal_bits(cell), Err(Error::Numeric));
    }
}

#[test]
fn narrowing_policy_exposes_the_f64_boundary_instead_of_claiming_cpp_parity() {
    // This decimal lies slightly above the f32 midpoint after 1.0, but rounds
    // to that midpoint in f64. A wider source parser may choose another f32;
    // typed/oracle acceptance must detect that difference, not hide it.
    assert_eq!(decimal_bits("1.0000000596046448"), Ok(0x3f80_0000));
    assert_eq!(decimal_bits("1.0000001192092896"), Ok(0x3f80_0001));
}

#[test]
fn byte_row_and_scalar_budgets_include_the_implicit_zero_row() {
    let table = GameTable::HpPerSta;
    let bytes = b"id\tv\n1\t2\n2\t3\n";
    let limits = Limits {
        bytes: bytes.len(),
        data_rows: 2,
        values: 3,
    };
    assert_eq!(
        parse_bounded(bytes, table, limits)
            .unwrap()
            .facts()
            .data_rows,
        2
    );
    for tighter in [
        Limits {
            bytes: bytes.len() - 1,
            ..limits
        },
        Limits {
            data_rows: 1,
            ..limits
        },
        Limits {
            values: 2,
            ..limits
        },
    ] {
        assert_eq!(
            parse_bounded(bytes, table, tighter).err(),
            Some(Error::Limit)
        );
    }
    assert_eq!(
        parse_bounded(
            b"id\tv",
            table,
            Limits {
                values: 0,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );

    let table = GameTable::SpellScaling;
    let bytes = fixture(table, &["1"], &vec!["0"; 24], "\n");
    let limits = Limits {
        bytes: bytes.len(),
        data_rows: 1,
        values: 48,
    };
    assert_eq!(
        parse_bounded(&bytes, table, limits)
            .unwrap()
            .facts()
            .consumer_rows_including_zero,
        2
    );
    assert_eq!(
        parse_bounded(
            &bytes,
            table,
            Limits {
                values: 47,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn public_byte_limit_is_checked_before_text_decoding() {
    rejected(
        &vec![0xff; MAX_FILE_BYTES + 1],
        GameTable::HpPerSta,
        Error::Limit,
    );
}

#[test]
fn errors_contain_only_static_format_codes() {
    for (error, code) in [
        (Error::Limit, "gt_policy_limit"),
        (Error::Utf8, "gt_invalid_utf8"),
        (Error::Text, "gt_invalid_text"),
        (Error::Header, "gt_header_columns"),
        (Error::Row, "gt_row_columns"),
        (Error::Numeric, "gt_numeric_admission"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

mod catalog;

mod acquisition;
