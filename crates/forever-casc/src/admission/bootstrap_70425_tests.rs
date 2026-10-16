// SPDX-License-Identifier: GPL-3.0-or-later
// Pure metadata controls for the explicit Linux70425 selector.
use super::*;

const GOOD: &str = "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\n\
    wow_classic_beta|1|1.60.1.70425|00000000000000000000000000000000|11111111111111111111111111111111|enUS\n";

#[test]
fn explicit70425_metadata_selects_exact_version_with_typed_native_shape() {
    assert_eq!(DB2_70425_CLIENTS, &[("1.60.1.70425", 70425)]);
    assert_eq!(parse_supported(GOOD, DB2_70425_CLIENTS), Ok(70425));
    assert_eq!(native_shape(GOOD), Ok(()));
}

#[test]
fn explicit70425_metadata_preserves_legacy_and70374_lists() {
    assert_eq!(
        SUPPORTED_CLIENTS,
        &[
            ("1.60.1.70170", 70170),
            ("1.60.1.70205", 70205),
            ("1.60.1.70338", 70338)
        ]
    );
    assert_eq!(DB2_70374_CLIENTS, &[("1.60.1.70374", 70374)]);
    assert_eq!(parse(GOOD), Err(Error::UnsupportedClient));
    assert_eq!(
        parse_supported(GOOD, DB2_70374_CLIENTS),
        Err(Error::UnsupportedClient)
    );
    for (version, build) in SUPPORTED_CLIENTS {
        let text = GOOD.replace("1.60.1.70425", version);
        assert_eq!(parse(&text), Ok(*build));
        assert_eq!(
            parse_supported(&text, DB2_70425_CLIENTS),
            Err(Error::UnsupportedClient)
        );
    }
}

#[test]
fn explicit70425_metadata_rejects_other_builds_products_and_inactive_rows() {
    for version in [
        "1.60.1.70374",
        "1.60.1.70424",
        "1.60.1.70426",
        "1.60.1.99999",
        "1.60.2.70425",
        "1.60.1.070425",
    ] {
        let text = GOOD.replace("1.60.1.70425", version);
        assert_eq!(
            parse_supported(&text, DB2_70425_CLIENTS),
            Err(Error::UnsupportedClient)
        );
    }
    for text in [
        GOOD.replace("wow_classic_beta", "wow"),
        GOOD.replace("|1|", "|0|"),
    ] {
        assert_eq!(
            parse_supported(&text, DB2_70425_CLIENTS),
            Err(Error::UnsupportedClient)
        );
    }
}

#[test]
fn explicit70425_metadata_rejects_duplicate_active_and_selector_columns() {
    let row = GOOD.lines().nth(1).unwrap();
    assert_eq!(
        parse_supported(&format!("{GOOD}{row}\n"), DB2_70425_CLIENTS),
        Err(Error::AmbiguousClient)
    );
    for text in [
        GOOD.replace("Version!STRING:0", "Version!STRING:0|Version!STRING:0"),
        GOOD.replace("Product!STRING:0", "Product!STRING:0|Product!STRING:0"),
        String::new(),
    ] {
        assert_eq!(
            parse_supported(&text, DB2_70425_CLIENTS),
            Err(Error::Metadata)
        );
    }
}

#[test]
fn explicit70425_typed_shape_keeps_existing_error_priority() {
    for text in [
        GOOD.replace("Build Key!HEX:16", "Build Key!STRING:0"),
        GOOD.replace("00000000000000000000000000000000", "invalid"),
        GOOD.replace("|enUS\n", "|xx\n"),
        GOOD.replace("|enUS\n", "|en\0US\n"),
    ] {
        // Version selection still succeeds; admit_selected then calls native_shape.
        assert_eq!(parse_supported(&text, DB2_70425_CLIENTS), Ok(70425));
        assert_eq!(native_shape(&text), Err(Error::Metadata));
    }
    let invalid_active = GOOD.replace("|1|", "|2|");
    // Public admission selects before native_shape; preserve that existing priority.
    assert_eq!(
        parse_supported(&invalid_active, DB2_70425_CLIENTS),
        Err(Error::UnsupportedClient)
    );
    assert_eq!(native_shape(&invalid_active), Err(Error::Metadata));
}
