// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: explicit resource policy and safety-ceiling tests.
use super::*;
#[test]
fn bounded_factories_are_every_original_default() {
    let p = Profile::BoundedV1;
    assert_eq!(Profile::default(), p);
    assert_eq!(p.collector_limits(), super::super::Limits::default());
    assert_eq!(p.spool_limits(), spool::Limits::default());
    assert_eq!(p.startup_limits(), gameobjects::Limits::default());
    assert_eq!(p.inventory_limits(), inventory_spool::Limits::default());
    assert_eq!(p.stage_limits(), staged::Limits::default());
    assert_eq!(p.model_limits(), staged::models::Limits::default());
    assert_eq!(p.package_limits(), staged::package::Limits::default());
}
#[test]
fn expanded_factories_preserve_file_parser_memory_and_format_ceilings() {
    let p = Profile::DiskWorldV1;
    let d = Profile::BoundedV1;
    let a = p.collector_limits();
    let b = d.collector_limits();
    assert_eq!(
        (a.max_file_bytes, a.max_memory_bytes, a.m2, a.wmo),
        (b.max_file_bytes, b.max_memory_bytes, b.m2, b.wmo)
    );
    let a = p.spool_limits();
    let b = d.spool_limits();
    assert_eq!(a.max_active_bytes, b.max_active_bytes);
    let a = p.startup_limits();
    let b = d.startup_limits();
    assert_eq!(
        (
            a.max_db2_bytes,
            a.max_direct_rows,
            a.max_copy_records,
            a.max_decode_allowance_bytes
        ),
        (
            b.max_db2_bytes,
            b.max_direct_rows,
            b.max_copy_records,
            b.max_decode_allowance_bytes
        )
    );
    let i = p.inventory_limits();
    assert_eq!(i.cache.max_memory_bytes, 64 * 1024 * 1024);
    assert!(
        2 * p.spool_limits().max_descriptor_bytes
            + 512 * 1024
            + i.cache.max_memory_bytes
            + 1024 * 1024
            < i.max_heap_bytes
    );
    assert!(i.cache.max_memory_bytes < d.inventory_limits().cache.max_memory_bytes);
    let a = p.stage_limits();
    assert_eq!(a.world.world.run.cache, i.cache);
    let b = d.stage_limits();
    assert_eq!((a.max_heap_bytes, a.records), (b.max_heap_bytes, b.records));
    let a = p.model_limits();
    let b = d.model_limits();
    assert_eq!(
        (a.max_memory_bytes, a.conversion),
        (b.max_memory_bytes, b.conversion)
    );
    let a = p.package_limits();
    let b = d.package_limits();
    assert_eq!(
        (a.max_memory_bytes, a.max_maps, a.assembly),
        (b.max_memory_bytes, b.max_maps, b.assembly)
    );
    assert_eq!(p.id(), "linux-collision-disk-world-resources-v1");
}
#[test]
fn every_collector_cumulative_field_is_explicit_and_downward_only() {
    use super::super::Error;
    let p = Profile::DiskWorldV1;
    p.collector_limits().validate_resources(p).unwrap();
    assert_eq!(p.collector_limits().validate(), Err(Error::InvalidLimits));
    for axis in 0..6 {
        let mut l = p.collector_limits();
        match axis {
            0 => l.max_reads += 1,
            1 => l.max_models += 1,
            2 => l.max_requests += 1,
            3 => l.max_dependency_requests += 1,
            4 => l.max_source_bytes += 1,
            _ => l.max_raw_bytes += 1,
        }
        assert_eq!(l.validate_resources(p), Err(Error::InvalidLimits));
    }
    let mut l = p.collector_limits();
    l.max_reads = 0;
    l.max_models = 0;
    l.max_requests = 0;
    l.max_dependency_requests = 0;
    l.max_source_bytes = 0;
    l.max_raw_bytes = 0;
    l.validate_resources(p).unwrap();
}
#[test]
fn expanded_collector_cannot_raise_file_memory_or_geometry_safety_limits() {
    use super::super::Error;
    for axis in 0..5 {
        let p = Profile::DiskWorldV1;
        let mut l = p.collector_limits();
        match axis {
            0 => l.max_file_bytes += 1,
            1 => l.max_memory_bytes += 1,
            2 => l.m2.max_vertices += 1,
            3 => l.wmo.max_groups += 1,
            _ => l.wmo.max_projection_bytes += 1,
        }
        assert_eq!(l.validate_resources(p), Err(Error::InvalidLimits));
    }
}
#[test]
fn declared_cumulative_capacities_fit_checked_x64_arithmetic() {
    let p = Profile::DiskWorldV1;
    if !p.supported() {
        return;
    }
    assert_eq!(p.collector_limits().max_reads, 1_000_000);
    assert_eq!(p.collector_limits().max_models, 65_536);
    assert_eq!(p.spool_limits().max_records, 262_144);
    assert_eq!(
        p.spool_limits().max_disk_bytes as u64,
        128 * 1024 * 1024 * 1024
    );
    assert_eq!(
        p.spool_limits().max_reload_bytes as u64,
        4 * 1024 * 1024 * 1024 * 1024
    );
    assert_eq!(p.stage_limits().max_records, 8_000_000);
    assert_eq!(p.package_limits().max_files, 1_000_000);
    assert_eq!(p.package_limits().max_memory_bytes, 2 * 1024 * 1024 * 1024);
}
