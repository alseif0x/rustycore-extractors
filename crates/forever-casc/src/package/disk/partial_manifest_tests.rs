use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-partial-manifest-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn context() -> Context {
    Context {
        build: 70450,
        locale: Locale::EnUs,
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
        components: Component::Db2.bit() | Component::Terrain.bit(),
    }
}

fn context_with_components(components: u8) -> Context {
    Context {
        components,
        ..context()
    }
}

fn section(lookup: &str, frame_ordinal: u32) -> PartialEncryptedSection {
    PartialEncryptedSection {
        lookup: lookup.to_owned(),
        frame_ordinal,
        decoded_size: 138,
    }
}

#[test]
fn finish_writes_deterministic_partial_declarations_and_verifier_returns_them() {
    let lab = Lab::new();
    let output = lab.0.join("package");
    let declaration = PartialEncryptedSection {
        lookup: "1122334455667788".into(),
        frame_ordinal: 2,
        decoded_size: 138,
    };
    let mut run = Run::create(&output, context(), &[], Limits::default()).unwrap();
    run.set_partial_encrypted_sections(std::slice::from_ref(&declaration))
        .unwrap();
    run.put(Component::Db2, "dbc/enUS/Map.db2", b"synthetic map")
        .unwrap();
    run.put(
        Component::Terrain,
        "maps/0002.tilelist",
        b"synthetic terrain",
    )
    .unwrap();
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    run.finish().unwrap();

    let manifest = fs::read_to_string(output.join(MANIFEST)).unwrap();
    assert!(manifest.contains(
        "\"partial_encrypted_sections\":[{\"lookup\":\"1122334455667788\",\"frame_ordinal\":2,\"decoded_size\":138}]"
    ));
    assert!(!manifest.contains("complete_source_decode_verified"));
    let manifest_header = manifest.lines().next().unwrap();
    let parsed_header = parse_header_partial_sections(manifest_header, context());
    assert!(
        parsed_header.is_ok(),
        "header {manifest_header}: {parsed_header:?}"
    );
    assert_eq!(
        parsed_header.unwrap().as_slice(),
        std::slice::from_ref(&declaration),
        "header {manifest_header}"
    );

    let mut observed = Vec::new();
    verify_with_preflight_and_partial_sections(
        &output,
        Some(context()),
        Limits::default(),
        |sections| {
            observed.extend_from_slice(sections);
            Ok(())
        },
        |_| Ok(()),
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(observed.as_slice(), std::slice::from_ref(&declaration));

    fs::write(output.join(INCOMPLETE), b"incomplete\n").unwrap();
    let mut resumed = Run::resume(&output, context(), &[], Limits::default()).unwrap();
    resumed
        .set_partial_encrypted_sections(&[declaration])
        .unwrap();
    resumed.finish().unwrap();
    assert!(!output.join(INCOMPLETE).exists());
}

#[test]
fn finish_omits_partial_declarations_when_none_are_registered() {
    let lab = Lab::new();
    let output = lab.0.join("package");
    let mut run = Run::create(&output, context(), &[], Limits::default()).unwrap();
    run.put(Component::Db2, "dbc/enUS/Map.db2", b"synthetic map")
        .unwrap();
    run.put(
        Component::Terrain,
        "maps/0002.tilelist",
        b"synthetic terrain",
    )
    .unwrap();
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    run.finish().unwrap();
    let manifest = fs::read_to_string(output.join(MANIFEST)).unwrap();
    assert!(!manifest.contains("partial_encrypted_sections"));
}

#[test]
fn partial_declarations_are_replayed_when_resuming_before_manifest_publication() {
    let lab = Lab::new();
    let output = lab.0.join("package");
    let declaration = PartialEncryptedSection {
        lookup: "1122334455667788".into(),
        frame_ordinal: 1,
        decoded_size: 138,
    };
    let mut first = Run::create(&output, context(), &[], Limits::default()).unwrap();
    first
        .set_partial_encrypted_sections(std::slice::from_ref(&declaration))
        .unwrap();
    first
        .put(Component::Db2, "dbc/enUS/Map.db2", b"synthetic map")
        .unwrap();
    first.finish_component(Component::Db2).unwrap();
    drop(first);

    let mut resumed = Run::resume(&output, context(), &[], Limits::default()).unwrap();
    resumed
        .set_partial_encrypted_sections(std::slice::from_ref(&declaration))
        .unwrap();
    assert_eq!(
        resumed
            .put(Component::Db2, "dbc/enUS/Map.db2", b"synthetic map")
            .unwrap(),
        Put::AlreadyPresent
    );
    resumed.finish_component(Component::Db2).unwrap();
    resumed
        .put(
            Component::Terrain,
            "maps/0002.tilelist",
            b"synthetic terrain",
        )
        .unwrap();
    resumed.finish_component(Component::Terrain).unwrap();
    resumed.finish().unwrap();
    let mut observed = Vec::new();
    verify_with_preflight_and_partial_sections(
        &output,
        Some(context()),
        Limits::default(),
        |sections| {
            observed.extend_from_slice(sections);
            Ok(())
        },
        |_| Ok(()),
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(observed, [declaration]);
}

#[test]
fn per_table_declarations_roundtrip_coexist_and_survive_resume() {
    let lab = Lab::new();
    let output = lab.0.join("package");
    let global = section("1122334455667788", 3);
    let table_a = PartialEncryptedTable {
        path: "dbc/enUS/Alpha.db2".into(),
        sections: vec![
            section("aabbccddeeff0011", 1),
            section("aabbccddeeff0011", 4),
        ],
    };
    let table_b = PartialEncryptedTable {
        path: "dbc/enUS/Zulu.db2".into(),
        sections: vec![section("0011223344556677", 2)],
    };
    let mut run = Run::create(&output, context(), &[], Limits::default()).unwrap();
    run.set_partial_encrypted_sections(std::slice::from_ref(&global))
        .unwrap();
    // Reverse insertion order proves the setter canonicalizes table order.
    run.set_partial_encrypted_table(&table_b.path, &table_b.sections)
        .unwrap();
    run.set_partial_encrypted_table(&table_a.path, &table_a.sections)
        .unwrap();
    run.set_partial_encrypted_table(&table_a.path, &table_a.sections)
        .unwrap();
    assert_eq!(
        run.partial_encrypted_tables.as_deref(),
        Some([table_a.clone(), table_b.clone()].as_slice())
    );
    run.put(Component::Db2, "dbc/enUS/Map.db2", b"synthetic map")
        .unwrap();
    run.put(
        Component::Terrain,
        "maps/0002.tilelist",
        b"synthetic terrain",
    )
    .unwrap();
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    run.finish().unwrap();

    let manifest = fs::read_to_string(output.join(MANIFEST)).unwrap();
    let manifest_header = manifest.lines().next().unwrap();
    assert!(manifest_header.contains("\"partial_encrypted_sections\":["));
    assert!(manifest_header.contains("\"partial_encrypted_tables\":["));
    assert!(manifest_header.find("Alpha.db2").unwrap() < manifest_header.find("Zulu.db2").unwrap());
    let declarations = parse_header_partial_declarations(manifest_header, context()).unwrap();
    assert_eq!(
        declarations.sections.as_slice(),
        std::slice::from_ref(&global)
    );
    assert_eq!(declarations.tables, [table_a.clone(), table_b.clone()]);
    assert_eq!(
        parse_header_partial_sections(manifest_header, context())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&global)
    );

    let mut observed_sections = Vec::new();
    let mut observed_tables = Vec::new();
    verify_with_preflight_and_partial_declarations(
        &output,
        Some(context()),
        Limits::default(),
        |sections| {
            observed_sections.extend_from_slice(sections);
            Ok(())
        },
        |tables| {
            observed_tables.extend_from_slice(tables);
            Ok(())
        },
        |_| Ok(()),
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(observed_sections, [global]);
    assert_eq!(observed_tables, [table_a.clone(), table_b.clone()]);

    fs::write(output.join(INCOMPLETE), b"incomplete\n").unwrap();
    let resumed = Run::resume(&output, context(), &[], Limits::default()).unwrap();
    assert_eq!(
        resumed.partial_encrypted_sections.as_deref(),
        Some([section("1122334455667788", 3)].as_slice())
    );
    assert_eq!(
        resumed.partial_encrypted_tables.as_deref(),
        Some([table_a, table_b].as_slice())
    );
    resumed.finish().unwrap();
    assert!(!output.join(INCOMPLETE).exists());
}

#[test]
fn per_table_setter_admits_db2_only_and_rejects_wrong_component_masks() {
    let lab = Lab::new();
    let table = PartialEncryptedTable {
        path: "dbc/enUS/Map.db2".into(),
        sections: vec![section("1122334455667788", 1)],
    };
    let output = lab.0.join("db2-only");
    let mut run = Run::create(
        &output,
        context_with_components(Component::Db2.bit()),
        &[],
        Limits::default(),
    )
    .unwrap();
    run.set_partial_encrypted_table(&table.path, &table.sections)
        .unwrap();
    assert_eq!(
        run.partial_encrypted_tables.as_deref(),
        Some([table.clone()].as_slice())
    );

    let output = lab.0.join("wrong-mask");
    let mut run = Run::create(
        &output,
        context_with_components(Component::Terrain.bit()),
        &[],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        run.set_partial_encrypted_table(&table.path, &table.sections),
        Err(Error::Component)
    );

    let output = lab.0.join("global-db2-only");
    let mut run = Run::create(
        &output,
        context_with_components(Component::Db2.bit()),
        &[],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        run.set_partial_encrypted_sections(&table.sections),
        Err(Error::Component)
    );
}

#[test]
fn per_table_path_duplicates_empty_sections_and_limits_are_rejected() {
    let valid_section = section("1122334455667788", 1);
    for invalid_path in [
        "",
        "/absolute/Map.db2",
        "../Map.db2",
        "dbc/../Map.db2",
        "dbc/Map..db2",
        "dbc/Map.txt",
        "dbc/Map.db2\0tail",
    ] {
        assert!(
            validate_partial_encrypted_table_path(invalid_path).is_err(),
            "accepted invalid path {invalid_path:?}"
        );
    }
    assert_eq!(
        validate_partial_encrypted_table_parts("dbc/enUS/Map.db2", &[]),
        Err(Error::Schema)
    );

    let duplicate = vec![
        PartialEncryptedTable {
            path: "dbc/enUS/Map.db2".into(),
            sections: vec![valid_section.clone()],
        },
        PartialEncryptedTable {
            path: "dbc/enUS/Map.db2".into(),
            sections: vec![valid_section.clone()],
        },
    ];
    assert_eq!(
        validate_partial_encrypted_tables(&duplicate),
        Err(Error::Schema)
    );

    let too_many_tables = (0..1162)
        .map(|index| PartialEncryptedTable {
            path: format!("dbc/enUS/Table{index:04}.db2"),
            sections: vec![valid_section.clone()],
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_partial_encrypted_tables(&too_many_tables),
        Err(Error::Limit)
    );

    let mut too_many_sections = (0..1160)
        .map(|index| PartialEncryptedTable {
            path: format!("dbc/enUS/Table{index:04}.db2"),
            sections: (0..16)
                .map(|ordinal| section("1122334455667788", ordinal))
                .collect(),
        })
        .collect::<Vec<_>>();
    too_many_sections.push(PartialEncryptedTable {
        path: "dbc/enUS/Table1160.db2".into(),
        sections: (0..17)
            .map(|ordinal| section("1122334455667788", ordinal))
            .collect(),
    });
    assert_eq!(
        validate_partial_encrypted_tables(&too_many_sections),
        Err(Error::Limit)
    );
}

#[test]
fn v25_global_header_roundtrips_unchanged_and_accepts_both_declaration_fields() {
    let global = section("1122334455667788", 2);
    let table = PartialEncryptedTable {
        path: "dbc/enUS/Map.db2".into(),
        sections: vec![section("aabbccddeeff0011", 1)],
    };
    let global_header =
        header_with_partial_sections(context(), std::slice::from_ref(&global)).unwrap();
    assert!(global_header.contains("\"partial_encrypted_sections\":["));
    assert!(!global_header.contains("partial_encrypted_tables"));
    assert_eq!(
        parse_header_partial_sections(global_header.trim_end(), context())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&global)
    );

    let combined_header = header_with_partial_declarations(
        context(),
        std::slice::from_ref(&global),
        std::slice::from_ref(&table),
    )
    .unwrap();
    let parsed = parse_header_partial_declarations(combined_header.trim_end(), context()).unwrap();
    assert_eq!(parsed.sections, [global]);
    assert_eq!(parsed.tables, [table]);
}
