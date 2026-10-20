// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: public V23 package consumer tests.
//! Selected package admission and bounded byte projection cases.

use super::*;

#[test]
fn package_verifier_binds_coverage_sizes_hashes_and_strict_mmap_framing() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let lab = PackageLab::publish(&coverage, &global, &tile);
    let verified = verify(&lab.output, expected(&coverage)).unwrap();
    assert_eq!(verified.coverage(), &coverage);
    assert_eq!(verified.summary().files, 3);
    drop(lab);

    let mut false_size = coverage.clone();
    false_size.global_bytes += 1;
    false_size.output_bytes += 1;
    let false_size_lab = PackageLab::publish(&false_size, &global, &tile);
    let error = verify(&false_size_lab.output, expected(&false_size)).unwrap_err();
    assert_eq!(error.code(), "product_navigation_output_inventory");
    drop(false_size_lab);

    let mut invalid_header = tile.clone();
    invalid_header[19] = 1;
    let mut bad_tile_coverage = coverage.clone();
    bad_tile_coverage.tiles[0].sha256 = integrity::digest(&invalid_header);
    let bad_tile_lab = PackageLab::publish(&bad_tile_coverage, &global, &invalid_header);
    let error = verify(&bad_tile_lab.output, expected(&bad_tile_coverage)).unwrap_err();
    assert_eq!(error.code(), "mmap_noncanonical_header");
    drop(bad_tile_lab);

    let mut invalid_global = global.clone();
    invalid_global[0] = 0;
    let mut bad_global_coverage = coverage.clone();
    bad_global_coverage.global_sha256 = integrity::digest(&invalid_global);
    let bad_global_lab = PackageLab::publish(&bad_global_coverage, &invalid_global, &tile);
    let error = verify(&bad_global_lab.output, expected(&bad_global_coverage)).unwrap_err();
    assert_eq!(error.code(), "mmap_magic");
}

#[test]
fn public_consumer_verifier_checks_expected_profile_and_context() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let lab = PackageLab::publish(&coverage, &global, &tile);

    let mut wrong_map = expected(&coverage);
    wrong_map.selected_map += 1;
    // The selected-map path inventory gate diagnoses this mismatch first.
    assert_eq!(
        verify(&lab.output, wrong_map).unwrap_err().code(),
        "product_navigation_output_inventory"
    );

    let mut wrong_context = expected(&coverage);
    let mut context = coverage.context;
    context.source_sha256 = [0x91; 32];
    wrong_context.context = Some(context);
    assert_eq!(
        verify(&lab.output, wrong_context).unwrap_err().code(),
        "package_context"
    );

    let mut wrong_locale = expected(&coverage);
    wrong_locale.locale = Locale::EsEs;
    assert_eq!(
        verify(&lab.output, wrong_locale).unwrap_err().code(),
        "product_navigation_identity"
    );

    let mut wrong_plan = expected(&coverage);
    wrong_plan.plan_sha256 = [0x92; 32];
    assert_eq!(
        verify(&lab.output, wrong_plan).unwrap_err().code(),
        "product_navigation_identity"
    );

    let mut without_context = expected(&coverage);
    without_context.context = None;
    let verified = verify(&lab.output, without_context).unwrap();
    assert_eq!(verified.coverage(), &coverage);
}

#[test]
fn public_consumer_verifier_rejects_malformed_duplicate_and_noncanonical_coverage() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let canonical = String::from_utf8(coverage.render().unwrap()).unwrap();
    let tile_path = mmap::tile_path(
        coverage.identity.selected_map,
        coverage.tiles[0].tile.row,
        coverage.tiles[0].tile.column,
    )
    .unwrap();
    let tile_artifacts = vec![(tile_path, tile.clone())];

    let duplicate = canonical.replace(
        "metadata-calls\t9\n",
        "metadata-calls\t9\nmetadata-calls\t9\n",
    );
    let noncanonical = canonical.replace("tile\t0\t0\twritten\t", "tile\t00\t0\twritten\t");
    for malformed in [
        b"broken\n".to_vec(),
        duplicate.into_bytes(),
        noncanonical.into_bytes(),
    ] {
        let lab = PackageLab::publish_raw(&coverage, &malformed, &global, &tile_artifacts);
        assert_eq!(
            verify(&lab.output, expected(&coverage)).unwrap_err().code(),
            "product_navigation_coverage"
        );
    }
}

#[test]
fn public_consumer_verifier_requires_exact_written_and_empty_tile_inventory() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let coverage_bytes = coverage.render().unwrap();
    let written_path = mmap::tile_path(
        coverage.identity.selected_map,
        coverage.tiles[0].tile.row,
        coverage.tiles[0].tile.column,
    )
    .unwrap();
    let empty_path = mmap::tile_path(
        coverage.identity.selected_map,
        coverage.tiles[1].tile.row,
        coverage.tiles[1].tile.column,
    )
    .unwrap();

    let missing = PackageLab::publish_raw(&coverage, &coverage_bytes, &global, &[]);
    assert_eq!(
        verify(&missing.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );

    let extra_empty = PackageLab::publish_raw(
        &coverage,
        &coverage_bytes,
        &global,
        &[
            (written_path.clone(), tile.clone()),
            (empty_path, tile.clone()),
        ],
    );
    assert_eq!(
        verify(&extra_empty.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );

    let mismatched_path = PackageLab::publish_raw(
        &coverage,
        &coverage_bytes,
        &global,
        &[("mmaps/0048_0_00.mmtile".to_owned(), tile.clone())],
    );
    assert_eq!(
        verify(&mismatched_path.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );

    let unexpected = PackageLab::publish_raw(
        &coverage,
        &coverage_bytes,
        &global,
        &[("mmaps/unexpected.mmap".to_owned(), tile)],
    );
    assert_eq!(
        verify(&unexpected.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );
}

#[test]
fn generic_package_engine_rejects_changed_missing_linked_and_incomplete_files() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let tile_path = mmap::tile_path(
        coverage.identity.selected_map,
        coverage.tiles[0].tile.row,
        coverage.tiles[0].tile.column,
    )
    .unwrap();

    let changed = PackageLab::publish(&coverage, &global, &tile);
    let changed_path = changed.output.join(&tile_path);
    let mut altered = tile.clone();
    altered[20] ^= 1;
    fs::write(&changed_path, altered).unwrap();
    assert_eq!(
        verify(&changed.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "package_digest"
    );

    let missing = PackageLab::publish(&coverage, &global, &tile);
    fs::remove_file(missing.output.join(&tile_path)).unwrap();
    assert!(verify(&missing.output, expected(&coverage)).is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let linked = PackageLab::publish(&coverage, &global, &tile);
        fs::hard_link(
            linked.output.join(&tile_path),
            linked.root.join("outside-hard-link"),
        )
        .unwrap();
        assert!(verify(&linked.output, expected(&coverage)).is_err());

        let symlinked = PackageLab::publish(&coverage, &global, &tile);
        let symlink_path = symlinked.output.join(&tile_path);
        fs::remove_file(&symlink_path).unwrap();
        let target = symlinked.root.join("outside-target");
        fs::write(&target, &tile).unwrap();
        symlink(&target, &symlink_path).unwrap();
        assert!(verify(&symlinked.output, expected(&coverage)).is_err());
    }

    let incomplete = PackageLab::incomplete(coverage.context);
    assert_eq!(
        verify(&incomplete.output, expected(&coverage))
            .unwrap_err()
            .code(),
        "package_incomplete"
    );
}

#[test]
fn public_read_batch_returns_requested_bytes_and_declared_empty_outcomes() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let lab = PackageLab::publish(&coverage, &global, &tile);
    let empty = coverage.tiles[1].tile;
    let written = coverage.tiles[0].tile;
    let requested = [empty, written];

    let batch = read_batch(
        &lab.output,
        expected(&coverage),
        &requested,
        ReadLimits::default(),
    )
    .unwrap();
    assert_eq!(batch.coverage(), &coverage);
    assert_eq!(batch.summary().files, 3);
    assert_eq!(batch.global_bytes(), global.as_slice());
    assert_eq!(batch.tiles().len(), 2);
    assert_eq!(batch.tiles()[0].tile(), empty);
    assert!(batch.tiles()[0].is_empty());
    assert_eq!(batch.tiles()[0].bytes(), None);
    assert_eq!(batch.tiles()[1].tile(), written);
    assert!(!batch.tiles()[1].is_empty());
    assert_eq!(batch.tiles()[1].bytes(), Some(tile.as_slice()));
}

#[test]
fn public_read_batch_rejects_bad_requests_before_package_payload_work() {
    let expected = Expected {
        selected_map: 48,
        locale: Locale::EnUs,
        plan_sha256: [0x34; 32],
        context: None,
    };
    let nonexistent = std::env::temp_dir().join(format!(
        "rce-navigation-v23-invalid-request-{}-{}",
        std::process::id(),
        NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
    ));

    assert_eq!(
        read_batch(
            &nonexistent,
            expected,
            &[
                Coordinate { row: 0, column: 0 },
                Coordinate { row: 0, column: 0 }
            ],
            ReadLimits::default(),
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_request"
    );
    assert_eq!(
        read_batch(
            &nonexistent,
            expected,
            &[Coordinate { row: 64, column: 0 }],
            ReadLimits::default(),
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_request"
    );

    let too_many = vec![Coordinate { row: 0, column: 0 }; 65];
    assert_eq!(
        read_batch(&nonexistent, expected, &too_many, ReadLimits::default(),)
            .unwrap_err()
            .code(),
        "product_navigation_read_limit"
    );

    let oversized = ReadLimits {
        max_batch_bytes: 128 * 1024 * 1024 + 1,
        ..ReadLimits::default()
    };
    assert_eq!(
        read_batch(&nonexistent, expected, &[], oversized)
            .unwrap_err()
            .code(),
        "product_navigation_read_limit"
    );
}

#[test]
fn public_read_batch_bounds_retained_bytes_and_rejects_tiles_outside_coverage() {
    let mut coverage = sample();
    let (global, tile) = framed_fixture(&mut coverage);
    let lab = PackageLab::publish(&coverage, &global, &tile);

    let outside = [Coordinate { row: 3, column: 3 }];
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&coverage),
            &outside,
            ReadLimits::default(),
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_unavailable"
    );

    let insufficient_batch = ReadLimits {
        max_batch_bytes: global.len() + tile.len() - 1,
        ..ReadLimits::default()
    };
    let tile_path = mmap::tile_path(
        coverage.identity.selected_map,
        coverage.tiles[0].tile.row,
        coverage.tiles[0].tile.column,
    )
    .unwrap();
    let mut changed = tile.clone();
    changed[20] ^= 1;
    fs::write(lab.output.join(tile_path), changed).unwrap();
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&coverage),
            &[coverage.tiles[0].tile],
            insufficient_batch,
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_limit"
    );

    let insufficient_file = ReadLimits {
        max_file_bytes: tile.len() - 1,
        ..ReadLimits::default()
    };
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&coverage),
            &[coverage.tiles[0].tile],
            insufficient_file,
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_limit"
    );
}
