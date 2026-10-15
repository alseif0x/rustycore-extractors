// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plan/package CLI checks; no client or native process interface.
#![cfg(target_os = "linux")]
use forever_casc::{
    integrity,
    package::{Component, Context, Run},
    terrain::tilelist,
};
use forever_extractor::plan::Plan;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
const TSV: &str = "forever-product\t12\nbuild\t70374\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70374-v1\ndecryption\tdisabled\ncomponents\tmaps\nterrain\tmap:2\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-v12-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("plan.tsv"), TSV).unwrap();
        Self(p)
    }
    fn command(&self, verb: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg(verb)
            .arg("--plan")
            .arg(self.0.join("plan.tsv"))
            .args(extra)
            .output()
            .unwrap()
    }
    fn publish(&self, valid: bool) {
        let p = Plan::parse(TSV).unwrap();
        let context = Context {
            build: 70374,
            locale: p.locale,
            components: 8,
            source_sha256: [1; 32],
            plan_sha256: integrity::digest(p.render().as_bytes()),
            producer_sha256: [2; 32],
        };
        let mut run =
            Run::create(&self.0.join("output"), context, &[], Default::default()).unwrap();
        let list = tilelist::TileList::new_70374(70374).unwrap();
        run.put(Component::Terrain, "maps/0002.tilelist", list.bytes())
            .unwrap();
        if valid {
            let coverage = format!(
                "terrain-coverage\t1\nprofile\tlinux70374-full-maps-available-map-v1\ncontext\t{}\nselection\tmap:2\npack\ttrue\nmap-bytes\t1000\t800\nmap-auth\tfalse\ttrue\nmap-rows\t1\t0\t1\nsection\t0\t1\t0\tadmitted\nsection\t1\t9\t0\tunknown-tact\nmap\t2\t0\t0\tmaps/0002.tilelist\nfinish\t1\t4096\t0\t4096\t0\t1\n",
                context.json()
            );
            run.put(
                Component::Terrain,
                "maps/selected-terrain-coverage-v1.tsv",
                coverage.as_bytes(),
            )
            .unwrap();
        }
        run.finish_component(Component::Terrain).unwrap();
        run.finish().unwrap();
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn v12_cli_inspect_discloses_available_map_scope() {
    let l = Lab::new();
    let r = l.command("inspect", &[]);
    assert!(r.status.success());
    let s = String::from_utf8(r.stdout).unwrap();
    assert!(s.contains("forever-product-v12"));
    assert!(s.contains("producer_reported"));
    assert!(s.contains("\"whole_client_map_coverage\":false"));
    assert!(s.contains("\"full_extractor_complete\":false"));
}
#[test]
fn v12_cli_both_verifiers_require_coverage_and_whole_map_lists() {
    for valid in [false, true] {
        let l = Lab::new();
        l.publish(valid);
        let out = l.0.join("output");
        for verb in ["verify", "verify-links"] {
            let r = l.command(verb, &["--output", out.to_str().unwrap()]);
            assert_eq!(r.status.success(), valid);
            if valid {
                let s = String::from_utf8(r.stdout).unwrap();
                assert!(s.contains("declared_available_map_coverage_links"));
                assert!(s.contains("\"full_extractor_complete\":false"));
            }
        }
    }
}
#[test]
fn v12_cli_keys_scratch_and_native_options_reject_without_reading_paths() {
    let l = Lab::new();
    for extra in [
        vec![
            "--client",
            "/authored/absent/client",
            "--output",
            "/authored/absent/output",
            "--keys",
            "/authored/absent/keys",
        ],
        vec![
            "--client",
            "/authored/absent/client",
            "--output",
            "/authored/absent/output",
            "--scratch",
            "/authored/absent/scratch",
        ],
        vec![
            "--client",
            "/authored/absent/client",
            "--output",
            "/authored/absent/output",
            "--backend",
            "native",
        ],
    ] {
        assert!(!l.command("extract", &extra).status.success());
    }
}
#[test]
fn v12_cli_legacy_all_alias_is_not_accepted() {
    let l = Lab::new();
    fs::write(l.0.join("plan.tsv"), TSV.replace("map:2", "all")).unwrap();
    assert!(!l.command("inspect", &[]).status.success());
}
