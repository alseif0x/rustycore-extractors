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

#[test]
fn request_heavy_changes_only_the_request_ceiling() {
    let p = Profile::RequestHeavyV1;
    if !p.supported() {
        return;
    }
    assert_eq!(p.id(), "linux-collision-request-heavy-resources-v1");
    let d = Profile::BoundedV1;
    let mut collector = p.collector_limits();
    assert_eq!(collector.max_requests, 16_000_000);
    collector.max_requests = d.collector_limits().max_requests;
    assert_eq!(collector, d.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_requests, 16_000_000);
    startup.collision.max_requests = d.startup_limits().collision.max_requests;
    assert_eq!(startup, d.startup_limits());

    assert_eq!(p.spool_limits(), d.spool_limits());
    assert_eq!(p.list_profile(), d.list_profile());
    assert_eq!(p.inventory_limits(), d.inventory_limits());
    assert_eq!(p.stage_limits(), d.stage_limits());
    assert_eq!(p.model_limits(), d.model_limits());
    assert_eq!(p.package_limits(), d.package_limits());
    p.collector_limits().validate_resources(p).unwrap();
}

#[test]
fn source_heavy_changes_only_source_bytes_from_dependency_heavy() {
    let p = Profile::SourceHeavyV1;
    let dependency_heavy = Profile::DependencyHeavyV1;
    assert_eq!(p.id(), "linux-collision-source-heavy-resources-v1");
    assert_eq!(p.supported(), cfg!(target_arch = "x86_64"));

    let mut collector = p.collector_limits();
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_source_bytes, 512 * 1024 * 1024);
    collector.max_source_bytes = dependency_heavy.collector_limits().max_source_bytes;
    assert_eq!(collector, dependency_heavy.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_requests, 16_000_000);
    assert_eq!(startup.collision.max_dependency_requests, 16_000_000);
    assert_eq!(startup.collision.max_source_bytes, 512 * 1024 * 1024);
    startup.collision.max_source_bytes =
        dependency_heavy.startup_limits().collision.max_source_bytes;
    assert_eq!(startup, dependency_heavy.startup_limits());

    assert_eq!(p.spool_limits(), dependency_heavy.spool_limits());
    assert_eq!(p.list_profile(), dependency_heavy.list_profile());
    assert_eq!(p.inventory_limits(), dependency_heavy.inventory_limits());
    assert_eq!(p.stage_limits(), dependency_heavy.stage_limits());
    assert_eq!(p.model_limits(), dependency_heavy.model_limits());
    assert_eq!(p.package_limits(), dependency_heavy.package_limits());
    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
    }
}

#[test]
fn source_heavy_changes_only_request_dependency_and_source_from_smaller_profiles() {
    let p = Profile::SourceHeavyV1;
    let bounded = Profile::BoundedV1;
    let request_heavy = Profile::RequestHeavyV1;

    let mut against_bounded = p.collector_limits();
    assert_eq!(against_bounded.max_requests, 16_000_000);
    assert_eq!(against_bounded.max_dependency_requests, 16_000_000);
    assert_eq!(against_bounded.max_source_bytes, 512 * 1024 * 1024);
    against_bounded.max_requests = bounded.collector_limits().max_requests;
    against_bounded.max_dependency_requests = bounded.collector_limits().max_dependency_requests;
    against_bounded.max_source_bytes = bounded.collector_limits().max_source_bytes;
    assert_eq!(against_bounded, bounded.collector_limits());

    let mut against_request_heavy = p.collector_limits();
    against_request_heavy.max_dependency_requests =
        request_heavy.collector_limits().max_dependency_requests;
    against_request_heavy.max_source_bytes = request_heavy.collector_limits().max_source_bytes;
    assert_eq!(against_request_heavy, request_heavy.collector_limits());

    let mut startup = p.startup_limits();
    startup.collision.max_requests = bounded.startup_limits().collision.max_requests;
    startup.collision.max_dependency_requests =
        bounded.startup_limits().collision.max_dependency_requests;
    startup.collision.max_source_bytes = bounded.startup_limits().collision.max_source_bytes;
    assert_eq!(startup, bounded.startup_limits());

    let mut startup_against_request_heavy = p.startup_limits();
    startup_against_request_heavy.collision.max_requests =
        request_heavy.startup_limits().collision.max_requests;
    startup_against_request_heavy
        .collision
        .max_dependency_requests = request_heavy
        .startup_limits()
        .collision
        .max_dependency_requests;
    startup_against_request_heavy.collision.max_source_bytes =
        request_heavy.startup_limits().collision.max_source_bytes;
    assert_eq!(
        startup_against_request_heavy,
        request_heavy.startup_limits()
    );
}

#[test]
fn dependency_heavy_changes_only_request_and_dependency_request_ceilings() {
    let p = Profile::DependencyHeavyV1;
    assert_eq!(p.id(), "linux-collision-dependency-heavy-resources-v1");
    assert_eq!(p.supported(), cfg!(target_arch = "x86_64"));
    let bounded = Profile::BoundedV1;
    let request_heavy = Profile::RequestHeavyV1;

    let mut collector = p.collector_limits();
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    collector.max_requests = bounded.collector_limits().max_requests;
    collector.max_dependency_requests = bounded.collector_limits().max_dependency_requests;
    assert_eq!(collector, bounded.collector_limits());

    let mut against_request_heavy = p.collector_limits();
    against_request_heavy.max_dependency_requests =
        request_heavy.collector_limits().max_dependency_requests;
    assert_eq!(against_request_heavy, request_heavy.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_requests, 16_000_000);
    assert_eq!(startup.collision.max_dependency_requests, 16_000_000);
    startup.collision.max_requests = bounded.startup_limits().collision.max_requests;
    startup.collision.max_dependency_requests =
        bounded.startup_limits().collision.max_dependency_requests;
    assert_eq!(startup, bounded.startup_limits());

    let mut startup_against_request_heavy = p.startup_limits();
    startup_against_request_heavy
        .collision
        .max_dependency_requests = request_heavy
        .startup_limits()
        .collision
        .max_dependency_requests;
    assert_eq!(
        startup_against_request_heavy,
        request_heavy.startup_limits()
    );

    assert_eq!(p.spool_limits(), bounded.spool_limits());
    assert_eq!(p.list_profile(), bounded.list_profile());
    assert_eq!(
        p.list_profile(),
        forever_vmap::gameobject_models::streaming::BudgetProfile::BoundedV1
    );
    assert_eq!(p.inventory_limits(), bounded.inventory_limits());
    assert_eq!(p.stage_limits(), bounded.stage_limits());
    assert_eq!(p.model_limits(), bounded.model_limits());
    assert_eq!(p.package_limits(), bounded.package_limits());
    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
    }
}

#[test]
fn model_heavy_changes_only_model_identity_capacities_from_read_heavy() {
    use super::super::Error;

    let p = Profile::ModelHeavyV1;
    let read = Profile::ReadHeavyV1;
    assert_eq!(p.id(), "linux-collision-model-heavy-resources-v1");
    assert_eq!(p.supported(), read.supported());

    let mut collector = p.collector_limits();
    assert_eq!(collector.max_models, 65_536);
    collector.max_models = read.collector_limits().max_models;
    assert_eq!(collector, read.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_models, 65_536);
    startup.collision.max_models = read.startup_limits().collision.max_models;
    assert_eq!(startup, read.startup_limits());

    let mut spool = p.spool_limits();
    assert_eq!(spool.max_records, 65_536);
    spool.max_records = read.spool_limits().max_records;
    assert_eq!(spool, read.spool_limits());
    assert_eq!(p.inventory_limits(), read.inventory_limits());
    assert_eq!(
        p.stage_limits().max_callbacks,
        read.stage_limits().max_callbacks
    );
    assert_eq!(
        p.stage_limits().max_records,
        read.stage_limits().max_records
    );
    assert_eq!(
        p.stage_limits().world.world.run.placements.max_models,
        65_536
    );
    let mut world = p.stage_limits().world;
    world.world.run.placements.max_models =
        read.stage_limits().world.world.run.placements.max_models;
    assert_eq!(world, read.stage_limits().world);

    let mut model = p.model_limits();
    assert_eq!(model.max_models, 65_536);
    model.max_models = read.model_limits().max_models;
    assert_eq!(model, read.model_limits());

    let mut package = p.package_limits();
    assert_eq!(package.max_models, 65_536);
    package.max_models = read.package_limits().max_models;
    assert_eq!(package, read.package_limits());

    assert_eq!(p.list_profile(), read.list_profile());
    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
        let mut excessive = p.collector_limits();
        excessive.max_models += 1;
        assert_eq!(excessive.validate_resources(p), Err(Error::InvalidLimits));
        assert!(read.collector_limits().max_models < p.collector_limits().max_models);
    }
}

#[test]
fn source_expanded_changes_only_source_bytes_from_model_heavy() {
    let p = Profile::SourceExpandedV1;
    let model = Profile::ModelHeavyV1;
    assert_eq!(p.id(), "linux-collision-source-expanded-resources-v1");
    assert_eq!(p.supported(), model.supported());

    let mut collector = p.collector_limits();
    assert_eq!(collector.max_source_bytes, 1024 * 1024 * 1024);
    assert_eq!(collector.max_models, 65_536);
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);
    collector.max_source_bytes = model.collector_limits().max_source_bytes;
    assert_eq!(collector, model.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_source_bytes, 1024 * 1024 * 1024);
    startup.collision.max_source_bytes = model.startup_limits().collision.max_source_bytes;
    assert_eq!(startup, model.startup_limits());

    assert_eq!(p.spool_limits(), model.spool_limits());
    assert_eq!(p.list_profile(), model.list_profile());
    assert_eq!(p.inventory_limits(), model.inventory_limits());
    assert_eq!(p.stage_limits(), model.stage_limits());
    assert_eq!(p.model_limits(), model.model_limits());
    assert_eq!(p.package_limits(), model.package_limits());

    let downward = Profile::BoundedV1.collector_limits();
    assert!(downward.max_source_bytes < p.collector_limits().max_source_bytes);
    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
        assert_eq!(
            p.startup_limits().collision.max_source_bytes,
            1024 * 1024 * 1024
        );
        let mut excessive = p.collector_limits();
        excessive.max_source_bytes += 1;
        assert_eq!(
            excessive.validate_resources(p),
            Err(super::super::Error::InvalidLimits)
        );
        assert!(model.collector_limits().max_source_bytes < p.collector_limits().max_source_bytes);
    }
}

#[test]
fn model_output_expanded_matches_source_expanded_limits_exactly() {
    use super::super::Error;

    let p = Profile::ModelOutputExpandedV1;
    let source_expanded = Profile::SourceExpandedV1;
    assert_eq!(p.id(), "linux-collision-model-output-expanded-resources-v1");
    assert_eq!(p.supported(), source_expanded.supported());
    assert_ne!(p, Profile::DiskWorldV1);
    assert_eq!(p.collector_limits(), source_expanded.collector_limits());
    assert_eq!(p.startup_limits(), source_expanded.startup_limits());
    assert_eq!(p.spool_limits(), source_expanded.spool_limits());
    assert_eq!(p.list_profile(), source_expanded.list_profile());
    assert_eq!(p.inventory_limits(), source_expanded.inventory_limits());
    assert_eq!(p.stage_limits(), source_expanded.stage_limits());
    assert_eq!(p.model_limits(), source_expanded.model_limits());
    assert_eq!(p.package_limits(), source_expanded.package_limits());
    assert_eq!(p.collector_limits().max_source_bytes, 1024 * 1024 * 1024);
    assert_eq!(p.collector_limits().max_models, 65_536);
    assert_eq!(p.collector_limits().max_reads, 1_000_000);
    assert_eq!(p.collector_limits().max_requests, 16_000_000);
    assert_eq!(p.collector_limits().max_dependency_requests, 16_000_000);
    assert_eq!(p.collector_limits().max_raw_bytes, 256 * 1024 * 1024);

    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
        for (axis, mut limits) in [
            p.collector_limits(),
            p.collector_limits(),
            p.collector_limits(),
            p.collector_limits(),
            p.collector_limits(),
            p.collector_limits(),
        ]
        .into_iter()
        .enumerate()
        {
            match axis {
                0 => limits.max_reads += 1,
                1 => limits.max_models += 1,
                2 => limits.max_requests += 1,
                3 => limits.max_dependency_requests += 1,
                4 => limits.max_source_bytes += 1,
                _ => limits.max_raw_bytes += 1,
            }
            assert_eq!(limits.validate_resources(p), Err(Error::InvalidLimits));
        }
        let mut downward = p.collector_limits();
        downward.max_reads = 4096;
        downward.max_models = 4096;
        downward.max_requests = 4096;
        downward.max_dependency_requests = 4096;
        downward.max_source_bytes = 256 * 1024 * 1024;
        downward.max_raw_bytes = 128 * 1024 * 1024;
        downward.validate_resources(p).unwrap();
    }
}

#[test]
fn model_bytes_expanded_matches_model_output_casc_limits_exactly() {
    use super::super::Error;

    let profile = Profile::ModelBytesExpandedV1;
    let model_output = Profile::ModelOutputExpandedV1;
    assert_eq!(
        profile.id(),
        "linux-collision-model-bytes-expanded-resources-v1"
    );
    assert_eq!(profile.supported(), model_output.supported());
    assert_ne!(profile, Profile::DiskWorldV1);
    assert_eq!(profile.collector_limits(), model_output.collector_limits());
    assert_eq!(profile.startup_limits(), model_output.startup_limits());
    assert_eq!(profile.spool_limits(), model_output.spool_limits());
    assert_eq!(profile.list_profile(), model_output.list_profile());
    assert_eq!(profile.inventory_limits(), model_output.inventory_limits());
    assert_eq!(profile.stage_limits(), model_output.stage_limits());
    assert_eq!(profile.model_limits(), model_output.model_limits());
    assert_eq!(profile.package_limits(), model_output.package_limits());

    let collector = profile.collector_limits();
    assert_eq!(collector.max_source_bytes, 1024 * 1024 * 1024);
    assert_eq!(collector.max_models, 65_536);
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);
    assert_eq!(profile.startup_limits().collision, collector);

    if profile.supported() {
        collector.validate_resources(profile).unwrap();
        for (axis, mut excessive) in [collector; 6].into_iter().enumerate() {
            match axis {
                0 => excessive.max_source_bytes += 1,
                1 => excessive.max_models += 1,
                2 => excessive.max_reads += 1,
                3 => excessive.max_requests += 1,
                4 => excessive.max_dependency_requests += 1,
                _ => excessive.max_raw_bytes += 1,
            }
            assert_eq!(
                excessive.validate_resources(profile),
                Err(Error::InvalidLimits)
            );
        }

        let mut downward = collector;
        downward.max_source_bytes = 128 * 1024 * 1024;
        downward.max_models = 4096;
        downward.max_reads = 4096;
        downward.max_requests = 4096;
        downward.max_dependency_requests = 4096;
        downward.max_raw_bytes = 128 * 1024 * 1024;
        downward.validate_resources(profile).unwrap();
    }
}

#[test]
fn source_expanded_v2_changes_only_source_bytes_from_model_bytes_expanded() {
    use super::super::Error;

    let profile = Profile::SourceExpandedV2;
    let model_bytes = Profile::ModelBytesExpandedV1;
    const SOURCE_CAP: usize = 2 * 1024 * 1024 * 1024;

    assert_eq!(profile.id(), "linux-collision-source-expanded-resources-v2");
    assert_eq!(profile.supported(), model_bytes.supported());
    assert_ne!(profile, Profile::DiskWorldV1);

    let mut collector = profile.collector_limits();
    assert_eq!(collector.max_source_bytes, SOURCE_CAP);
    assert_eq!(collector.max_models, 65_536);
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);
    collector.max_source_bytes = model_bytes.collector_limits().max_source_bytes;
    assert_eq!(collector, model_bytes.collector_limits());

    let mut startup = profile.startup_limits();
    assert_eq!(startup.collision.max_source_bytes, SOURCE_CAP);
    startup.collision.max_source_bytes = model_bytes.startup_limits().collision.max_source_bytes;
    assert_eq!(startup, model_bytes.startup_limits());

    assert_eq!(profile.spool_limits(), model_bytes.spool_limits());
    assert_eq!(profile.list_profile(), model_bytes.list_profile());
    assert_eq!(profile.inventory_limits(), model_bytes.inventory_limits());
    assert_eq!(profile.stage_limits(), model_bytes.stage_limits());
    assert_eq!(profile.model_limits(), model_bytes.model_limits());
    assert_eq!(profile.package_limits(), model_bytes.package_limits());

    if profile.supported() {
        profile
            .collector_limits()
            .validate_resources(profile)
            .unwrap();
        let mut reduced = profile.collector_limits();
        reduced.max_source_bytes = SOURCE_CAP - 1;
        reduced.validate_resources(profile).unwrap();

        let mut excessive = profile.collector_limits();
        excessive.max_source_bytes = SOURCE_CAP + 1;
        assert_eq!(
            excessive.validate_resources(profile),
            Err(Error::InvalidLimits)
        );
    }
}

#[test]
fn placement_expanded_v1_uses_source_expanded_v2_casc_limits() {
    use super::super::Error;

    let profile = Profile::PlacementExpandedV1;
    let source_expanded = Profile::SourceExpandedV2;
    let collector = profile.collector_limits();

    assert_eq!(
        profile.id(),
        "linux-collision-placement-expanded-resources-v1"
    );
    assert_eq!(profile.supported(), source_expanded.supported());
    assert_ne!(profile, Profile::DiskWorldV1);
    assert_eq!(collector.max_source_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(collector.max_models, 65_536);
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);
    assert_eq!(profile.startup_limits().collision, collector);
    assert_eq!(
        profile.collector_limits(),
        source_expanded.collector_limits()
    );
    assert_eq!(profile.startup_limits(), source_expanded.startup_limits());
    assert_eq!(profile.spool_limits(), source_expanded.spool_limits());
    assert_eq!(profile.list_profile(), source_expanded.list_profile());
    assert_eq!(
        profile.inventory_limits(),
        source_expanded.inventory_limits()
    );
    assert_eq!(profile.stage_limits(), source_expanded.stage_limits());
    assert_eq!(profile.model_limits(), source_expanded.model_limits());
    assert_eq!(profile.package_limits(), source_expanded.package_limits());
    assert_eq!(
        profile.stage_limits().world.world.run.placements,
        source_expanded.stage_limits().world.world.run.placements
    );

    if profile.supported() {
        collector.validate_resources(profile).unwrap();
        let mut reduced = collector;
        reduced.max_source_bytes -= 1;
        reduced.max_models -= 1;
        reduced.max_reads -= 1;
        reduced.max_requests -= 1;
        reduced.max_dependency_requests -= 1;
        reduced.max_raw_bytes -= 1;
        reduced.validate_resources(profile).unwrap();

        for (axis, mut excessive) in [collector; 6].into_iter().enumerate() {
            match axis {
                0 => excessive.max_source_bytes += 1,
                1 => excessive.max_models += 1,
                2 => excessive.max_reads += 1,
                3 => excessive.max_requests += 1,
                4 => excessive.max_dependency_requests += 1,
                _ => excessive.max_raw_bytes += 1,
            }
            assert_eq!(
                excessive.validate_resources(profile),
                Err(Error::InvalidLimits)
            );
        }
    }
}

#[test]
fn assembly_expanded_v1_uses_placement_expanded_casc_limits() {
    use super::super::Error;

    let profile = Profile::AssemblyExpandedV1;
    let placement_expanded = Profile::PlacementExpandedV1;
    let collector = profile.collector_limits();

    assert_eq!(
        profile.id(),
        "linux-collision-assembly-expanded-resources-v1"
    );
    assert_eq!(profile.supported(), placement_expanded.supported());
    assert_ne!(profile, Profile::DiskWorldV1);
    assert_eq!(
        profile.collector_limits(),
        placement_expanded.collector_limits()
    );
    assert_eq!(
        profile.startup_limits(),
        placement_expanded.startup_limits()
    );
    assert_eq!(profile.spool_limits(), placement_expanded.spool_limits());
    assert_eq!(profile.list_profile(), placement_expanded.list_profile());
    assert_eq!(
        profile.inventory_limits(),
        placement_expanded.inventory_limits()
    );
    assert_eq!(profile.stage_limits(), placement_expanded.stage_limits());
    assert_eq!(profile.model_limits(), placement_expanded.model_limits());
    assert_eq!(
        profile.package_limits(),
        placement_expanded.package_limits()
    );
    assert_eq!(
        profile.stage_limits().world.world.run.placements.max_models,
        65_536
    );

    if profile.supported() {
        collector.validate_resources(profile).unwrap();
        for (axis, mut excessive) in [collector; 6].into_iter().enumerate() {
            match axis {
                0 => excessive.max_source_bytes += 1,
                1 => excessive.max_models += 1,
                2 => excessive.max_reads += 1,
                3 => excessive.max_requests += 1,
                4 => excessive.max_dependency_requests += 1,
                _ => excessive.max_raw_bytes += 1,
            }
            assert_eq!(
                excessive.validate_resources(profile),
                Err(Error::InvalidLimits)
            );
        }
        let mut reduced = collector;
        reduced.max_source_bytes -= 1;
        reduced.max_models -= 1;
        reduced.max_reads -= 1;
        reduced.max_requests -= 1;
        reduced.max_dependency_requests -= 1;
        reduced.max_raw_bytes -= 1;
        reduced.validate_resources(profile).unwrap();
    }
}

#[test]
fn gameobject_expanded_v1_matches_assembly_expanded_limits() {
    use super::super::Error;

    let profile = Profile::GameObjectExpandedV1;
    let assembly = Profile::AssemblyExpandedV1;
    let collector = profile.collector_limits();

    assert_eq!(
        profile.id(),
        "linux-collision-gameobject-expanded-resources-v1"
    );
    assert_eq!(profile.supported(), assembly.supported());
    assert_ne!(profile, Profile::DiskWorldV1);
    assert_eq!(profile.collector_limits(), assembly.collector_limits());
    assert_eq!(profile.startup_limits(), assembly.startup_limits());
    assert_eq!(profile.spool_limits(), assembly.spool_limits());
    assert_eq!(profile.list_profile(), assembly.list_profile());
    assert_eq!(profile.inventory_limits(), assembly.inventory_limits());
    assert_eq!(profile.stage_limits(), assembly.stage_limits());
    assert_eq!(profile.model_limits(), assembly.model_limits());
    assert_eq!(profile.package_limits(), assembly.package_limits());

    let bounded = Profile::BoundedV1;
    assert_eq!(
        profile.startup_limits().max_candidate_requests,
        bounded.startup_limits().max_candidate_requests
    );
    assert_eq!(
        profile.startup_limits().max_db2_bytes,
        bounded.startup_limits().max_db2_bytes
    );
    assert_eq!(
        profile.startup_limits().max_direct_rows,
        bounded.startup_limits().max_direct_rows
    );
    assert_eq!(
        profile.startup_limits().max_copy_records,
        bounded.startup_limits().max_copy_records
    );
    assert_eq!(
        profile.startup_limits().max_decode_allowance_bytes,
        bounded.startup_limits().max_decode_allowance_bytes
    );

    if profile.supported() {
        collector.validate_resources(profile).unwrap();
        let mut reduced = collector;
        reduced.max_source_bytes -= 1;
        reduced.max_models -= 1;
        reduced.max_reads -= 1;
        reduced.max_requests -= 1;
        reduced.max_dependency_requests -= 1;
        reduced.max_raw_bytes -= 1;
        reduced.validate_resources(profile).unwrap();

        for (axis, mut excessive) in [collector; 6].into_iter().enumerate() {
            match axis {
                0 => excessive.max_source_bytes += 1,
                1 => excessive.max_models += 1,
                2 => excessive.max_reads += 1,
                3 => excessive.max_requests += 1,
                4 => excessive.max_dependency_requests += 1,
                _ => excessive.max_raw_bytes += 1,
            }
            assert_eq!(
                excessive.validate_resources(profile),
                Err(Error::InvalidLimits)
            );
        }
    }
}

#[test]
fn read_heavy_changes_only_read_capacity_from_source_heavy() {
    let p = Profile::ReadHeavyV1;
    let source_heavy = Profile::SourceHeavyV1;
    assert_eq!(p.id(), "linux-collision-read-heavy-resources-v1");
    assert_eq!(p.supported(), source_heavy.supported());

    let mut collector = p.collector_limits();
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_source_bytes, 512 * 1024 * 1024);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);
    collector.max_reads = source_heavy.collector_limits().max_reads;
    assert_eq!(collector, source_heavy.collector_limits());

    let mut startup = p.startup_limits();
    assert_eq!(startup.collision.max_reads, 1_000_000);
    assert_eq!(startup.collision.max_requests, 16_000_000);
    assert_eq!(startup.collision.max_dependency_requests, 16_000_000);
    assert_eq!(startup.collision.max_source_bytes, 512 * 1024 * 1024);
    assert_eq!(startup.collision.max_raw_bytes, 256 * 1024 * 1024);
    startup.collision.max_reads = source_heavy.startup_limits().collision.max_reads;
    assert_eq!(startup, source_heavy.startup_limits());

    assert_eq!(p.spool_limits(), source_heavy.spool_limits());
    assert_eq!(p.list_profile(), source_heavy.list_profile());
    assert_eq!(p.inventory_limits(), source_heavy.inventory_limits());
    assert_eq!(p.stage_limits(), source_heavy.stage_limits());
    assert_eq!(p.model_limits(), source_heavy.model_limits());
    assert_eq!(p.package_limits(), source_heavy.package_limits());
    if p.supported() {
        p.collector_limits().validate_resources(p).unwrap();
    }
}
