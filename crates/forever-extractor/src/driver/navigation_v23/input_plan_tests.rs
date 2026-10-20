// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: synthetic admission tests for sealed v20/v22 input plans.

use super::{Plan, checked_input_plans};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const V20: &str =
    include_str!("../../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
const V22: &str =
    include_str!("../../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
const V23: &str =
    include_str!("../../../../../tests/contracts/fixtures/forever-product-v23/map48.tsv");

static NEXT: AtomicU64 = AtomicU64::new(0);

struct PlanFiles {
    directory: PathBuf,
    terrain_package: PathBuf,
    terrain: PathBuf,
    vmap_package: PathBuf,
    vmap: PathBuf,
}

impl PlanFiles {
    fn new(terrain: &Plan, vmap: &Plan) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "rce-v23-input-plan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let terrain_package = directory.join("terrain-package");
        let vmap_package = directory.join("vmap-package");
        fs::create_dir(&terrain_package).unwrap();
        fs::create_dir(&vmap_package).unwrap();
        let terrain_path = directory.join("terrain.tsv");
        let vmap_path = directory.join("vmap.tsv");
        fs::write(&terrain_path, terrain.render()).unwrap();
        fs::write(&vmap_path, vmap.render()).unwrap();
        Self {
            directory,
            terrain_package,
            terrain: terrain_path,
            vmap_package,
            vmap: vmap_path,
        }
    }

    fn inputs(&self) -> crate::driver::SelectedNavigationInputs<'_> {
        crate::driver::SelectedNavigationInputs {
            terrain_package: &self.terrain_package,
            terrain_plan: &self.terrain,
            vmap_package: &self.vmap_package,
            vmap_plan: &self.vmap,
        }
    }
}

impl Drop for PlanFiles {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn check_input_plans(product: &Plan, terrain: &Plan, vmap: &Plan) -> Result<(), &'static str> {
    let files = PlanFiles::new(terrain, vmap);
    let inputs = files.inputs();
    match checked_input_plans(product, &inputs) {
        Ok((terrain, vmap)) => {
            drop(terrain);
            drop(vmap);
            Ok(())
        }
        Err(error) => Err(error.code),
    }
}

#[test]
fn canonical_v20_and_v22_empty_db2_plans_are_admitted_for_matching_v23_locale() {
    let terrain = Plan::parse(V20).unwrap();
    let vmap = Plan::parse(V22).unwrap();
    let product = Plan::parse(V23).unwrap();
    assert!(terrain.db2.is_empty());
    assert!(vmap.db2.is_empty());
    assert_eq!(terrain.locale, product.locale);
    assert_eq!(vmap.locale, product.locale);

    assert_eq!(check_input_plans(&product, &terrain, &vmap), Ok(()));
}

#[test]
fn canonical_wrong_input_profiles_are_rejected() {
    let terrain = Plan::parse(V20).unwrap();
    let vmap = Plan::parse(V22).unwrap();
    let product = Plan::parse(V23).unwrap();

    assert_eq!(
        check_input_plans(&product, &vmap, &vmap),
        Err("product_navigation_input_profile")
    );
    assert_eq!(
        check_input_plans(&product, &terrain, &terrain),
        Err("product_navigation_input_profile")
    );
}

#[test]
fn canonical_input_plans_must_match_the_v23_product_locale() {
    let terrain = Plan::parse(V20).unwrap();
    let vmap = Plan::parse(V22).unwrap();
    let spanish_product = Plan::parse(&V23.replace("locale\tenUS", "locale\tesES")).unwrap();
    assert_eq!(spanish_product.locale, forever_casc::Locale::EsEs);
    assert_eq!(terrain.locale, forever_casc::Locale::EnUs);
    assert_eq!(vmap.locale, forever_casc::Locale::EnUs);

    assert_eq!(
        check_input_plans(&spanish_product, &terrain, &vmap),
        Err("product_navigation_input_profile")
    );
}
