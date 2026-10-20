// SPDX-License-Identifier: GPL-3.0-or-later
//! The v23 composite context is available only through its selected-input API.
use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const V23: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v23/map48.tsv");
const V22: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");

static NEXT: AtomicU64 = AtomicU64::new(0);

fn missing(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "rce-v23-api-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn keyring() -> forever_casc::keys::Keyring {
    forever_casc::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap()
}

fn error_code<T>(result: Result<T, Error>) -> &'static str {
    match result {
        Ok(_) => panic!("expected v23 generic API rejection"),
        Err(error) => error.code,
    }
}

#[test]
fn generic_v23_client_and_package_entries_fail_before_path_io_or_output_creation() {
    let plan = Plan::parse(V23).unwrap();
    let client = missing("client");
    let output = missing("output");
    let keys_path = missing("keys");
    let scratch = missing("scratch");

    assert_eq!(
        error_code(open(&plan, &client)),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(open_with_keys(&plan, &client, keyring())),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(inspect(&plan, &client, [1; 32], None)),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(extract(&plan, &client, &output, [2; 32], false)),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(extract_with_keys(
            &plan,
            &client,
            &output,
            [3; 32],
            false,
            crate::input::SuppliedKeys {
                keyring: keyring(),
                source: keys_path.clone(),
            },
        )),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(extract_disk(
            &plan, &client, &output, [4; 32], false, None, &scratch,
        )),
        "product_navigation_inputs_required"
    );
    assert_eq!(
        error_code(crate::verify::verify_links(&plan, &output, None)),
        "product_navigation_inputs_required"
    );
    assert!(!client.exists());
    assert!(!output.exists());
    assert!(!keys_path.exists());
    assert!(!scratch.exists());
}

#[test]
fn dedicated_navigation_links_wrapper_refuses_other_profiles_before_path_io() {
    let plan = Plan::parse(V22).unwrap();
    let output = missing("output");
    let terrain_package = missing("terrain-package");
    let terrain_plan = missing("terrain-plan");
    let vmap_package = missing("vmap-package");
    let vmap_plan = missing("vmap-plan");
    let inputs = SelectedNavigationInputs {
        terrain_package: &terrain_package,
        terrain_plan: &terrain_plan,
        vmap_package: &vmap_package,
        vmap_plan: &vmap_plan,
    };

    assert_eq!(
        error_code(crate::verify::verify_selected_navigation_links(
            &plan, &output, None, &inputs
        )),
        "product_navigation_profile"
    );
    assert!(!output.exists());
    assert!(!terrain_package.exists());
    assert!(!terrain_plan.exists());
    assert!(!vmap_package.exists());
    assert!(!vmap_plan.exists());
}

#[test]
fn selected_navigation_wrapper_admits_mutated_plan_before_any_path_io() {
    let valid = Plan::parse(V23).unwrap();
    let invalid = Plan {
        offmesh: Some(Vec::new()),
        ..valid
    };
    let client = missing("invalid-client");
    let output = missing("invalid-output");
    let terrain_package = missing("invalid-terrain-package");
    let terrain_plan = missing("invalid-terrain-plan");
    let vmap_package = missing("invalid-vmap-package");
    let vmap_plan = missing("invalid-vmap-plan");
    let inputs = SelectedNavigationInputs {
        terrain_package: &terrain_package,
        terrain_plan: &terrain_plan,
        vmap_package: &vmap_package,
        vmap_plan: &vmap_plan,
    };

    assert_eq!(
        error_code(extract_selected_navigation(
            &invalid, &client, &output, [9; 32], false, &inputs,
        )),
        "product_plan_component_scope"
    );
    assert!(!client.exists());
    assert!(!output.exists());
    assert!(!terrain_package.exists());
    assert!(!terrain_plan.exists());
    assert!(!vmap_package.exists());
    assert!(!vmap_plan.exists());
}
