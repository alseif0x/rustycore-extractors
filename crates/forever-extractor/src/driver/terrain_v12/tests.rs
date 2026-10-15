// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use forever_casc::integrity;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
const TSV: &str = "forever-product\t12\nbuild\t70374\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70374-v1\ndecryption\tdisabled\ncomponents\tmaps\nterrain\tmap:2\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";
fn product() -> Plan {
    Plan::parse(TSV).unwrap()
}
fn context(p: &Plan) -> Context {
    Context {
        build: 70374,
        locale: p.locale,
        source_sha256: [1; 32],
        plan_sha256: integrity::digest(p.render().as_bytes()),
        producer_sha256: [2; 32],
        components: 8,
    }
}
fn coverage(count: u32) -> Coverage {
    let p = product();
    let mut sections = [None; 16];
    sections[0] = Some(SectionCoverage {
        ordinal: 0,
        declared_records: 1,
        declared_copies: 0,
        state: SectionState::Admitted,
    });
    sections[1] = Some(SectionCoverage {
        ordinal: 1,
        declared_records: 9,
        declared_copies: 2,
        state: SectionState::UnknownTact,
    });
    Coverage {
        context: context(&p),
        selection: p.terrain,
        pack: p.pack,
        map: available::MapFacts {
            declared_bytes: 1000,
            available_bytes: 800,
            full_ckey_verified: false,
            encoded_key_authenticated: true,
            work: Default::default(),
            direct_rows: 1,
            copy_records: 0,
            materialized_rows: 1,
            sections,
        },
        maps: vec![(2, count)],
    }
}
fn map() -> Vec<u8> {
    use forever_terrain::{height, maps};
    let area = maps::AreaSection::from_ids(&[42; 256]);
    let height = height::HeightSection::from_bits(
        &vec![0; height::V9_COUNT],
        &vec![0; height::V8_COUNT],
        Default::default(),
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    maps::assemble_70374(
        70374,
        maps::MapSections {
            area: &area,
            height: &height,
            liquid: None,
            holes: None,
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}
fn files(count: u32) -> Vec<(String, Vec<u8>)> {
    let mut list = tilelist::TileList::new_70374(70374).unwrap();
    let mut f = Vec::new();
    if count == 1 {
        list.set(5, 6).unwrap();
        f.push((plan::map_relative_path(2, 5, 6).unwrap(), map()));
    }
    f.push((list_path(2), list.bytes().to_vec()));
    f.push((
        COVERAGE_PATH.into(),
        coverage(count).render().unwrap().into_bytes(),
    ));
    f
}
fn artifact(p: &str, b: &[u8]) -> Artifact {
    Artifact {
        component: Component::Terrain,
        path: p.into(),
        bytes: b.len() as u64,
        sha256: integrity::digest(b),
    }
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-v12-product-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn out(&self) -> PathBuf {
        self.0.join("output")
    }
    fn run(&self) -> Run {
        Run::create(&self.out(), context(&product()), &[], package_limits()).unwrap()
    }
    fn publish(&self, f: &[(String, Vec<u8>)]) {
        let mut r = self.run();
        for (p, b) in f {
            r.put(Component::Terrain, p, b).unwrap();
        }
        r.finish_component(Component::Terrain).unwrap();
        r.finish().unwrap();
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn v12_coverage_roundtrip_discloses_omissions_without_plaintext_claim() {
    let c = coverage(1);
    let text = c.render().unwrap();
    let r = Coverage::parse(text.as_bytes()).unwrap();
    assert_eq!(r.render().unwrap(), text);
    assert!(!r.map.full_ckey_verified);
    assert!(r.map.encoded_key_authenticated);
    assert_eq!(r.map.sections[1].unwrap().state, SectionState::UnknownTact);
}
#[test]
fn v12_transient_candidate_work_is_not_canonical_replay_identity() {
    let mut c = coverage(1);
    let first = c.render().unwrap();
    c.map.work.strict_candidate_attempts = 7;
    c.map.work.partial_candidate_attempts = 8;
    c.map.work.partial_encoded_bytes_read = 91;
    assert_eq!(first, c.render().unwrap());
}
#[test]
fn v12_coverage_rejects_false_auth_counts_sections_and_noncanonical_bytes() {
    let base = coverage(1);
    let mut c = base.clone();
    c.map.full_ckey_verified = false;
    c.map.encoded_key_authenticated = false;
    assert!(c.render().is_err());
    let mut c = base.clone();
    c.map.materialized_rows = 0;
    assert!(c.render().is_err());
    let mut c = base.clone();
    c.map.full_ckey_verified = true;
    assert!(c.render().is_err());
    let mut c = base.clone();
    c.map.direct_rows = 2;
    assert!(c.render().is_err());
    let mut c = base.clone();
    c.map.sections[1].as_mut().unwrap().ordinal = 3;
    assert!(c.render().is_err());
    let mut c = base.clone();
    c.maps.push((2, 0));
    assert!(c.render().is_err());
    let mut b = base.render().unwrap();
    b.pop();
    assert!(Coverage::parse(b.as_bytes()).is_err());
    assert!(Coverage::parse(&vec![b'x'; COVERAGE_BYTES + 1]).is_err());
}
#[test]
fn v12_empty_available_domain_has_explicit_coverage_only() {
    let mut p = product();
    p.terrain = Selection::All;
    let mut c = coverage(0);
    c.context = context(&p);
    c.selection = p.terrain;
    c.maps.clear();
    c.map.sections = [None; 16];
    c.map.direct_rows = 0;
    c.map.copy_records = 0;
    c.map.materialized_rows = 0;
    let bytes = c.render().unwrap().into_bytes();
    let a = artifact(COVERAGE_PATH, &bytes);
    admit_metadata(&p, std::slice::from_ref(&a)).unwrap();
    let mut v = VerifyFacts::new();
    v.inspect(&a, &bytes).unwrap();
    v.finish(&p, context(&p)).unwrap();
}
#[test]
fn v12_full_map_and_empty_whole_map_verify_both_scopes() {
    for count in [0, 1] {
        let l = Lab::new();
        l.publish(&files(count));
        let a = crate::verify::verify(&product(), &l.out(), None).unwrap();
        assert_eq!(
            a,
            crate::verify::verify_links(&product(), &l.out(), None).unwrap()
        );
        assert_eq!(a.files, 2 + count as usize);
    }
}
#[test]
fn v12_metadata_rejects_missing_coverage_lists_foreign_tiles_and_oversize() {
    let f = files(1);
    let a: Vec<_> = f.iter().map(|(p, b)| artifact(p, b)).collect();
    admit_metadata(&product(), &a).unwrap();
    for i in 0..a.len() {
        let mut bad = a.clone();
        bad.remove(i);
        assert!(admit_metadata(&product(), &bad).is_err() || i == 0);
    }
    let mut bad = a.clone();
    bad[0].path = plan::map_relative_path(3, 5, 6).unwrap();
    assert!(admit_metadata(&product(), &bad).is_err());
    let mut bad = a.clone();
    bad.last_mut().unwrap().bytes = COVERAGE_BYTES as u64 + 1;
    assert!(admit_metadata(&product(), &bad).is_err());
}
#[test]
fn v12_payload_verifier_rejects_missing_or_unlisted_tiles_and_context_drift() {
    for mode in 0..4 {
        let l = Lab::new();
        let mut f = files(1);
        match mode {
            0 => {
                f.remove(0);
            }
            1 => {
                let mut list = tilelist::TileList::new_70374(70374).unwrap();
                list.set(6, 6).unwrap();
                f[1].1 = list.bytes().to_vec();
            }
            2 => {
                let mut c = coverage(1);
                c.context.producer_sha256 = [3; 32];
                f[2].1 = c.render().unwrap().into_bytes();
            }
            _ => {
                f[0].1[8..12].copy_from_slice(&70205u32.to_le_bytes());
            }
        }
        l.publish(&f);
        assert!(crate::verify::verify(&product(), &l.out(), None).is_err());
        assert!(crate::verify::verify_links(&product(), &l.out(), None).is_err());
    }
}
#[test]
fn v12_sealed_component_replay_checks_every_existing_payload_and_coverage() {
    let l = Lab::new();
    let f = files(1);
    let mut r = l.run();
    for (p, b) in &f {
        r.put(Component::Terrain, p, b).unwrap();
    }
    r.finish_component(Component::Terrain).unwrap();
    drop(r);
    let mut resumed = Run::resume(&l.out(), context(&product()), &[], package_limits()).unwrap();
    assert_eq!(resumed.completed_components().unwrap(), 8);
    for (p, b) in &f {
        assert_eq!(
            resumed.put(Component::Terrain, p, b).unwrap(),
            package::Put::AlreadyPresent
        );
    }
    resumed.finish_component(Component::Terrain).unwrap();
    resumed.finish().unwrap();
    crate::verify::verify(&product(), &l.out(), None).unwrap();
}
#[test]
fn v12_replay_modified_coverage_is_rejected_and_no_prefix_verifies() {
    let l = Lab::new();
    let f = files(1);
    let mut r = l.run();
    r.put(Component::Terrain, &f[0].0, &f[0].1).unwrap();
    drop(r);
    assert!(crate::verify::verify(&product(), &l.out(), None).is_err());
    let mut resumed = Run::resume(&l.out(), context(&product()), &[], package_limits()).unwrap();
    for (p, b) in &f {
        resumed.put(Component::Terrain, p, b).unwrap();
    }
    resumed.finish_component(Component::Terrain).unwrap();
    drop(resumed);
    let mut resumed = Run::resume(&l.out(), context(&product()), &[], package_limits()).unwrap();
    let mut c = coverage(1);
    c.map.available_bytes -= 1;
    assert!(
        resumed
            .put(
                Component::Terrain,
                COVERAGE_PATH,
                c.render().unwrap().as_bytes()
            )
            .is_err()
    );
    assert!(crate::verify::verify(&product(), &l.out(), None).is_err());
}
#[test]
fn v12_duplicate_tile_and_list_callbacks_fail_closed() {
    let mut bits = Bits::new(2);
    bits.tile(1, 2).unwrap();
    assert!(bits.tile(1, 2).is_err());
    let mut list = tilelist::TileList::new_70374(70374).unwrap();
    list.set(1, 2).unwrap();
    bits.tile_list(list.bytes()).unwrap();
    assert_eq!(bits.maps, bits.listed);
    assert!(bits.tile_list(list.bytes()).is_err());
}
