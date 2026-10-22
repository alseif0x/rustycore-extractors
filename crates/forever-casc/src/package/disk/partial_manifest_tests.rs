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
