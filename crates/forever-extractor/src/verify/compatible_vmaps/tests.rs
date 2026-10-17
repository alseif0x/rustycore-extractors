// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
use super::*;
use forever_casc::{integrity, package::Run};
use std::{
    fs,
    os::unix::fs::symlink,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    context: Context,
    plan: Plan,
}
impl Fixture {
    fn new() -> Self {
        let plan = Plan::parse(crate::plan::v14_tests::V14).unwrap();
        let root = std::env::temp_dir().join(format!(
            "rce-v14-offline-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let context = Context {
            build: 70425,
            locale: forever_casc::Locale::EsEs,
            components: Component::Collision.bit(),
            source_sha256: [1; 32],
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            producer_sha256: [3; 32],
        };
        Self {
            root,
            context,
            plan,
        }
    }
    fn output(&self) -> PathBuf {
        self.root.join("package")
    }
    fn files(
        &self,
        maps: &[(u32, Option<u32>, bool, u8)],
        extra: Option<(&str, &[u8])>,
    ) -> Vec<(String, Vec<u8>)> {
        let mut files = vec![(
            "vmaps/GameObjectModels.dtree".into(),
            gameobject_models::DTREE_MAGIC.to_vec(),
        )];
        for &(id, _, present, _) in maps {
            if present {
                let built = assembly::build(id, &[], Default::default()).unwrap();
                files.push((tree_path(id), built.tree_bytes().to_vec()));
                files.push((
                    cov::proof_path(id),
                    cov::proof_header(id, 0).unwrap().to_vec(),
                ));
            }
        }
        if let Some((path, b)) = extra {
            files.push((path.to_owned(), b.to_vec()));
        }
        let mut enc = cov::fixture(self.context, maps);
        for (path, b) in &files {
            enc.artifact(path, b.len(), integrity::digest(b)).unwrap();
        }
        files.push((cov::PATH.into(), enc.finish().unwrap().to_vec()));
        files
    }
    fn publish(&self, files: &[(String, Vec<u8>)]) {
        let mut run = Run::create(&self.output(), self.context, &[], Default::default()).unwrap();
        for (path, b) in files {
            run.put(Component::Collision, path, b).unwrap();
        }
        run.finish_component(Component::Collision).unwrap();
        run.finish().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn v14_verify_genuine_format_empty_domain_and_empty_map_witness() {
    for maps in [
        &[][..],
        &[(0, None, true, 0)][..],
        &[(0, None, false, 0)][..],
    ] {
        let f = Fixture::new();
        f.publish(&f.files(maps, None));
        let a = verify(&f.plan, &f.output(), Some(f.context)).unwrap();
        assert_eq!(a.context, f.context);
    }
}
#[test]
fn v14_verify_rejects_self_consistent_manifest_with_spurious_artifact() {
    let f = Fixture::new();
    f.publish(&f.files(&[], Some(("vmaps/unrequested.bin", b"no"))));
    assert!(verify(&f.plan, &f.output(), None).is_err());
}
#[test]
fn v14_verify_rejects_omitted_tree_and_extra_missing_wdt_tree() {
    let f = Fixture::new();
    let mut files = f.files(&[(0, None, true, 0)], None);
    files.retain(|(p, _)| p != &tree_path(0));
    f.publish(&files);
    assert!(verify(&f.plan, &f.output(), None).is_err());
    let f = Fixture::new();
    let tree = assembly::build(0, &[], Default::default()).unwrap();
    f.publish(&f.files(
        &[(0, None, false, 0)],
        Some((&tree_path(0), tree.tree_bytes())),
    ));
    assert!(verify(&f.plan, &f.output(), None).is_err());
}
#[test]
fn v14_verify_plan_context_and_requested_map_are_not_interchangeable() {
    let f = Fixture::new();
    f.publish(&f.files(&[], None));
    let selected = Plan {
        map_domain: Some(0),
        ..f.plan.clone()
    };
    assert!(verify(&selected, &f.output(), None).is_err());
    let changed = Context {
        source_sha256: [9; 32],
        ..f.context
    };
    assert!(verify(&f.plan, &f.output(), Some(changed)).is_err());
}
#[test]
fn v14_verify_rejects_map_unbound_association_witness() {
    let f = Fixture::new();
    let mut files = f.files(&[(0, None, true, 0)], None);
    let at = files
        .iter()
        .position(|(p, _)| p == &cov::proof_path(0))
        .unwrap();
    files[at].1 = cov::proof_header(1, 0).unwrap().to_vec();
    let mut enc = cov::fixture(f.context, &[(0, None, true, 0)]);
    for (p, b) in files.iter().filter(|(p, _)| p != cov::PATH) {
        enc.artifact(p, b.len(), integrity::digest(b)).unwrap();
    }
    files.iter_mut().find(|(p, _)| p == cov::PATH).unwrap().1 = enc.finish().unwrap().to_vec();
    f.publish(&files);
    assert!(verify(&f.plan, &f.output(), None).is_err());
}
#[test]
fn v14_verify_refuses_symlink_root_even_for_valid_package() {
    let f = Fixture::new();
    f.publish(&f.files(&[], None));
    let link = f.root.join("linked");
    symlink(f.output(), &link).unwrap();
    assert!(verify(&f.plan, &link, None).is_err());
}
#[test]
fn v14_verify_integrity_rejects_changed_coverage_without_semantic_fallback() {
    let f = Fixture::new();
    f.publish(&f.files(&[], None));
    fs::write(f.output().join(cov::PATH), b"not coverage").unwrap();
    assert!(verify(&f.plan, &f.output(), None).is_err());
}
