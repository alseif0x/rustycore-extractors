// SPDX-License-Identifier: GPL-3.0-or-later
// Pure synthetic metadata and bootstrap controls for the explicit Linux70450 selector.
use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const GOOD: &str = "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\n\
    wow_classic_beta|1|1.60.1.70450|00000000000000000000000000000000|11111111111111111111111111111111|enUS\n";

struct Scratch(PathBuf);
impl Scratch {
    fn new(info: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "forever-admission-70450-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join(".build.info"), info).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn options() -> crate::local_store::OpenOptions {
    crate::local_store::OpenOptions {
        locale: crate::Locale::EnUs,
        index_layouts: [crate::local_index::Layout::Contiguous; 16],
        root_layout: crate::root::Layout::Header50893V2,
        limits: crate::local_store::Limits::default(),
    }
}

#[test]
fn explicit70450_admission_selects_exact_typed_version_without_changing_other_lists() {
    assert_eq!(DB2_70450_CLIENTS, &[("1.60.1.70450", 70450)]);
    assert_eq!(parse_supported(GOOD, DB2_70450_CLIENTS), Ok(70450));
    assert_eq!(native_shape(GOOD), Ok(()));
    assert_eq!(parse(GOOD), Err(Error::UnsupportedClient));
    assert_eq!(
        parse_supported(GOOD, DB2_70425_CLIENTS),
        Err(Error::UnsupportedClient)
    );
    assert_eq!(
        SUPPORTED_CLIENTS,
        &[
            ("1.60.1.70170", 70170),
            ("1.60.1.70205", 70205),
            ("1.60.1.70338", 70338)
        ]
    );
}

#[test]
fn explicit70450_rejects_neighboring_versions_and_malformed_selectors() {
    for version in [
        "1.60.1.70425",
        "1.60.1.70449",
        "1.60.1.70451",
        "1.60.2.70450",
        "1.60.1.070450",
    ] {
        let text = GOOD.replace("1.60.1.70450", version);
        assert_eq!(
            parse_supported(&text, DB2_70450_CLIENTS),
            Err(Error::UnsupportedClient)
        );
    }
    for text in [
        GOOD.replace("wow_classic_beta", "wow"),
        GOOD.replace("|1|", "|0|"),
        GOOD.replace("Version!STRING:0", "Version!STRING:0|Version!STRING:0"),
    ] {
        assert_ne!(parse_supported(&text, DB2_70450_CLIENTS), Ok(70450));
    }
    let invalid_type = GOOD.replace("Build Key!HEX:16", "Build Key!STRING:0");
    assert_eq!(parse_supported(&invalid_type, DB2_70450_CLIENTS), Ok(70450));
    assert_eq!(native_shape(&invalid_type), Err(Error::Metadata));
}

#[test]
fn explicit70450_open_admits_before_config_io_and_keeps_older_selectors_closed() {
    let admitted = Scratch::new(GOOD);
    assert_eq!(admit_db2_70450(&admitted.0), Ok(70450));
    // The target parser selected build70450 and then failed on the deliberately
    // absent synthetic config; a selector mismatch would fail earlier instead.
    assert_eq!(
        crate::local_store::LocalStore::open_db2_70450(&admitted.0, options()).err(),
        Some(crate::local_store::Error::Input)
    );

    let old = Scratch::new(&GOOD.replace("1.60.1.70450", "1.60.1.70425"));
    assert_eq!(admit_db2_70450(&old.0), Err(Error::UnsupportedClient));
    assert_eq!(
        crate::local_store::LocalStore::open_db2_70450(&old.0, options()).err(),
        Some(crate::local_store::Error::UnsupportedClient)
    );
}
