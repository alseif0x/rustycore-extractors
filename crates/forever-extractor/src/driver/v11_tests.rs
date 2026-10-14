// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn v11_camera_budget_is_downward_before_requested_read() {
    assert_eq!(camera_remaining(0).unwrap(), cameras::MAX_FILE_BYTES as u64);
    assert_eq!(camera_remaining(cameras::MAX_TOTAL_BYTES - 1).unwrap(), 1);
    for total in [
        cameras::MAX_TOTAL_BYTES,
        cameras::MAX_TOTAL_BYTES + 1,
        usize::MAX,
    ] {
        assert_eq!(
            camera_remaining(total).unwrap_err().code,
            "camera_policy_limit"
        );
    }
}
#[test]
fn v11_invalid_manual_plan_rejects_before_client_paths() {
    let mut p = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv"
    ))
    .unwrap();
    p.components = 1;
    assert_eq!(
        open(&p, Path::new("/nonexistent-authored-v11-client"))
            .err()
            .unwrap()
            .code,
        "product_plan_component_scope"
    );
}

#[test]
fn v11_db2_total_is_reduced_before_read() {
    assert_eq!(db2_remaining(0).unwrap(), DB2_FILE_BYTES);
    assert_eq!(db2_remaining(DB2_TOTAL_BYTES - 1).unwrap(), 1);
    for total in [DB2_TOTAL_BYTES, DB2_TOTAL_BYTES + 1, u64::MAX] {
        assert_eq!(
            db2_remaining(total).unwrap_err().code,
            "product_db2_total_limit"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn v11_keys_policy_rejects_before_any_client_path() {
    let mut p = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv"
    ))
    .unwrap();
    p.decryption = crate::plan::Decryption::ProvidedSalsa20V1;
    assert_eq!(
        open(&p, Path::new("/nonexistent-authored-v11-client"))
            .err()
            .unwrap()
            .code,
        "product_keys_required"
    );
}

#[cfg(target_os = "linux")]
mod sealed {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Lab(PathBuf);
    impl Lab {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "rce-v11-sealed-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
        fn output(&self) -> PathBuf {
            self.0.join("output")
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn put32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn put16(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn camera_source(ids: &[u32], copies: &[(u32, u32)]) -> Vec<u8> {
        let mut out = vec![0; 384];
        out[..4].copy_from_slice(b"WDC5");
        put32(&mut out, 4, 5);
        put32(&mut out, 136, ids.len() as u32);
        put32(&mut out, 140, 5);
        put32(&mut out, 144, 28);
        put32(&mut out, 156, cameras::SOURCE_LAYOUT);
        put32(&mut out, 160, 1);
        let max = (ids.len() as u32)
            .max(copies.iter().map(|(id, _)| *id).max().unwrap_or(0))
            .max(1);
        put32(&mut out, 164, max);
        put16(&mut out, 174, u16::MAX);
        put32(&mut out, 176, 5);
        put32(&mut out, 188, 120);
        put32(&mut out, 200, 1);
        put32(&mut out, 212, 384);
        put32(&mut out, 216, ids.len() as u32);
        put32(&mut out, 228, ids.len() as u32 * 4);
        put32(&mut out, 240, copies.len() as u32);
        for (field, (offset, count)) in [(0, 3), (96, 1), (128, 1), (160, 1), (192, 1)]
            .into_iter()
            .enumerate()
        {
            let at = 264 + 24 * field;
            put16(&mut out, at, offset);
            put32(&mut out, at + 16, 32);
            put32(&mut out, at + 20, count);
        }
        for &id in ids {
            let at = out.len();
            out.resize(at + 28, 0);
            put32(&mut out, at + 20, id);
        }
        for id in 1..=ids.len() as u32 {
            out.extend_from_slice(&id.to_le_bytes());
        }
        for &(new, old) in copies {
            out.extend_from_slice(&new.to_le_bytes());
            out.extend_from_slice(&old.to_le_bytes());
        }
        out
    }
    fn plan() -> Plan {
        Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv"
        ))
        .unwrap()
    }
    fn context(p: &Plan) -> Context {
        Context {
            build: p.build,
            locale: p.locale,
            components: p.components,
            plan_sha256: integrity::digest(p.render().as_bytes()),
            source_sha256: [1; 32],
            producer_sha256: [2; 32],
        }
    }
    fn source_bytes() -> Vec<u8> {
        let mut b = camera_source(&[0, 11], &[]);
        put32(&mut b, 152, cameras::CameraContract::TABLE_HASH);
        b
    }
    fn run(lab: &Lab, p: &Plan, source: &[u8], sealed: bool) -> Run {
        let mut r =
            Run::create(&lab.output(), context(p), &[], package::Limits::default()).unwrap();
        r.put(Component::Db2, &cameras::source_path(p.locale), source)
            .unwrap();
        if sealed {
            r.finish_component(Component::Db2).unwrap();
        }
        r
    }
    #[test]
    fn source_is_compared_before_discovery_on_fresh_and_resume() {
        let lab = Lab::new();
        let p = plan();
        let bytes = source_bytes();
        let mut r = run(&lab, &p, &bytes, true);
        let selected = camera_plan(&p, &mut r, &bytes).unwrap();
        assert_eq!(selected.file_ids(), &[0, 11].into_iter().collect());
        assert_eq!(r.completed_components().unwrap(), Component::Db2.bit());
        drop(r);
        let mut r =
            Run::resume(&lab.output(), context(&p), &[], package::Limits::default()).unwrap();
        assert_eq!(
            camera_plan(&p, &mut r, &bytes).unwrap().file_ids(),
            selected.file_ids()
        );
        assert_eq!(r.completed_components().unwrap(), Component::Db2.bit());
    }
    #[test]
    fn reread_mismatch_poison_precedes_discovery_and_publication() {
        let lab = Lab::new();
        let p = plan();
        let bytes = source_bytes();
        let mut r = run(&lab, &p, &bytes, true);
        let mut changed = bytes;
        changed[0] ^= 1;
        assert!(camera_plan(&p, &mut r, &changed).is_err());
        assert!(
            r.put(Component::Cameras, &cameras::relative_path(11), b"fake")
                .is_err()
        );
        assert!(!lab.output().join("cameras").exists());
    }
    #[test]
    fn missing_db2_seal_rejects_without_publishing_source_again() {
        let lab = Lab::new();
        let p = plan();
        let bytes = source_bytes();
        let mut r = run(&lab, &p, &bytes, false);
        assert!(camera_plan(&p, &mut r, &bytes).is_err());
        assert!(r.finish_component(Component::Cameras).is_err());
        assert!(!lab.output().join("cameras").exists());
    }
}
