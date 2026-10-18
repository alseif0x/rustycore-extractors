// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Explicit70425/70450 GNU15-v2: one genuine leased world through publication.
use super::*;
use crate::collision_coverage as coverage;
use forever_casc::{
    collision::{compatible::startup, plan::MapSelection},
    integrity,
    local_store::{
        Error as StoreError, ProductDiagnosticObserver, ProductDiagnosticPhase,
        ProductDiagnosticRequest, ProductDiagnosticRequestKind,
    },
    package::Context,
};
use staged::{
    compatible as placement,
    models::{compatible as models, package::compatible as package},
};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
fn failure(e: package::Error<Error>) -> Error {
    let code = e.code();
    match e.cause {
        package::Cause::Callback(e) if code == staged::package::Kind::Callback.code() => e,
        _ => code.into(),
    }
}
fn put_collision(run: &mut Run, sealed: bool, path: &str, bytes: &[u8]) -> Result<(), Error> {
    if sealed {
        run.verify_sealed_artifact(&forever_casc::package::Artifact {
            component: Component::Collision,
            path: path.to_owned(),
            bytes: bytes.len() as u64,
            sha256: integrity::digest(bytes),
        })?;
    } else {
        run.put(Component::Collision, path, bytes)?;
    }
    Ok(())
}

struct ProductDiagnosticSink {
    parent: anchors::Chain,
    leaf: OsString,
    phase: ProductDiagnosticPhase,
    next_ordinal: u32,
    pending: Option<(
        ProductDiagnosticRequest,
        StoreError,
        bool,
        Option<&'static str>,
    )>,
    tolerated_absences: [u32; ProductDiagnosticRequestKind::COUNT],
    finished: bool,
}
impl ProductDiagnosticSink {
    fn from_environment(client: &anchors::Chain) -> Result<Option<Self>, Error> {
        Self::from_optional_output(client, std::env::var_os("FOREVER_PRODUCT_DIAGNOSTIC"))
    }

    fn from_optional_output(
        client: &anchors::Chain,
        output: Option<OsString>,
    ) -> Result<Option<Self>, Error> {
        let Some(output) = output else {
            return Ok(None);
        };
        Self::at_path(client, &PathBuf::from(output)).map(Some)
    }

    fn at_path(client: &anchors::Chain, output: &Path) -> Result<Self, Error> {
        anchors::validate(output)?;
        let leaf = output
            .file_name()
            .ok_or_else(|| Error::from("product_diagnostic_path"))?
            .to_os_string();
        let parent_path = output
            .parent()
            .ok_or_else(|| Error::from("product_diagnostic_path"))?;
        let parent = anchors::Chain::open(parent_path, true)?;
        if parent.contains_leaf(client) {
            return Err("product_diagnostic_path".into());
        }
        let anchored = anchors::anchor(parent.file()).join(&leaf);
        match std::fs::symlink_metadata(anchored) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("product_diagnostic_path".into()),
        }
        Ok(Self {
            parent,
            leaf,
            phase: ProductDiagnosticPhase::Other,
            next_ordinal: 0,
            pending: None,
            tolerated_absences: [0; ProductDiagnosticRequestKind::COUNT],
            finished: false,
        })
    }

    fn cause_json(cause: StoreError, blte_code: Option<&'static str>) -> String {
        if let StoreError::NoLocalEncoding(failures) = cause {
            let blte_field = match blte_code {
                Some(
                    code @ ("blte_invalid_header"
                    | "blte_truncated"
                    | "blte_size_mismatch"
                    | "blte_checksum_mismatch"
                    | "blte_invalid_zlib"
                    | "blte_encrypted_unsupported"
                    | "blte_invalid_encryption_header"
                    | "blte_cipher_unsupported"
                    | "blte_encryption_key_unavailable"
                    | "blte_encryption_requires_content_key"
                    | "blte_recursive_unsupported"
                    | "blte_unknown_mode"),
                ) => format!(",\"blte_code\":\"{code}\""),
                _ => String::new(),
            };
            return format!(
                "{{\"variant\":\"NoLocalEncoding\",\"candidate_failures\":{{\"missing\":{},\"incomplete\":{},\"encrypted\":{},\"corrupt\":{},\"unsupported\":{}}}{blte_field}}}",
                failures.missing,
                failures.incomplete,
                failures.encrypted,
                failures.corrupt,
                failures.unsupported,
            );
        }
        let variant = match cause {
            StoreError::Input => "Input",
            StoreError::InputChanged => "InputChanged",
            StoreError::BuildMetadata => "BuildMetadata",
            StoreError::UnsupportedClient => "UnsupportedClient",
            StoreError::AmbiguousClient => "AmbiguousClient",
            StoreError::Limit => "Limit",
            StoreError::IndexSet => "IndexSet",
            StoreError::UnknownSize => "UnknownSize",
            StoreError::MissingFile => "MissingFile",
            StoreError::Integrity => "Integrity",
            StoreError::NoLocalEncoding(_) => unreachable!(),
            StoreError::Config(_) => "Config",
            StoreError::Index(_) => "Index",
            StoreError::Encoding(_) => "Encoding",
            StoreError::Root(_) => "Root",
            StoreError::NamesDisabled => "NamesDisabled",
            StoreError::NamesAlreadyEnabled => "NamesAlreadyEnabled",
            StoreError::Name(_) => "Name",
        };
        format!("{{\"variant\":\"{variant}\"}}")
    }

    fn write_once(
        &mut self,
        request: ProductDiagnosticRequest,
        cause: StoreError,
        guards_observed: bool,
        blte_code: Option<&'static str>,
    ) {
        if self.parent.check().is_err() {
            return;
        }
        let file_data_id = request
            .file_data_id
            .map_or_else(|| "null".to_owned(), |id| id.to_string());
        let json = format!(
            "{{\"phase\":\"{}\",\"request_kind\":\"{}\",\"file_data_id\":{},\"ordinal\":{},\"typed_cause\":{},\"code\":\"{}\",\"guards_observed\":{},\"tolerated_absences\":{}}}",
            request.phase.name(),
            request.request_kind.name(),
            file_data_id,
            request.ordinal,
            Self::cause_json(cause, blte_code),
            cause.code(),
            guards_observed || matches!(cause, StoreError::InputChanged),
            self.tolerated_absences_json(),
        );
        let target = anchors::anchor(self.parent.file()).join(&self.leaf);
        let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(target)
        else {
            return;
        };
        let _ = file.write_all(json.as_bytes());
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    }

    fn tolerated_absences_json(&self) -> String {
        let mut json = String::from("{");
        for (index, request_kind) in ProductDiagnosticRequestKind::ALL.iter().enumerate() {
            if index != 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "\"{}\":{}",
                request_kind.name(),
                self.tolerated_absences[request_kind.index()]
            ));
        }
        json.push('}');
        json
    }
}
impl ProductDiagnosticObserver for ProductDiagnosticSink {
    fn set_phase(&mut self, phase: ProductDiagnosticPhase) {
        if self.phase != phase {
            self.phase = phase;
            self.next_ordinal = 0;
        }
    }

    fn begin_request(
        &mut self,
        request_kind: ProductDiagnosticRequestKind,
        file_data_id: Option<u32>,
    ) -> Option<ProductDiagnosticRequest> {
        if self.finished || self.pending.is_some() {
            return None;
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Some(ProductDiagnosticRequest {
            phase: self.phase,
            request_kind,
            file_data_id,
            ordinal,
        })
    }

    fn tolerated_absence(&mut self, request: ProductDiagnosticRequest) {
        if self.finished || self.pending.is_some() {
            return;
        }
        let count = &mut self.tolerated_absences[request.request_kind.index()];
        *count = count.saturating_add(1);
    }

    fn source_failure(
        &mut self,
        request: ProductDiagnosticRequest,
        cause: StoreError,
        guards_observed: bool,
    ) {
        self.source_failure_with_blte_code(request, cause, guards_observed, None);
    }

    fn source_failure_with_blte_code(
        &mut self,
        request: ProductDiagnosticRequest,
        cause: StoreError,
        guards_observed: bool,
        blte_code: Option<&'static str>,
    ) {
        if !self.finished && self.pending.is_none() {
            self.pending = Some((request, cause, guards_observed, blte_code));
        }
    }

    fn finish(&mut self, terminal_code: Option<&'static str>, source_guard_changed: bool) {
        if self.finished {
            return;
        }
        self.finished = true;
        let Some((request, cause, guards_observed, blte_code)) = self.pending else {
            return;
        };
        if terminal_code == Some(cause.code()) {
            self.write_once(
                request,
                cause,
                guards_observed || source_guard_changed,
                blte_code,
            );
        }
    }
}

pub(in crate::driver) fn product_diagnostic_for_db2(
    client_path: &Path,
    output: Option<OsString>,
) -> Result<Option<Box<dyn ProductDiagnosticObserver>>, Error> {
    let Some(output) = output else {
        return Ok(None);
    };
    let client = anchors::Chain::open(client_path, true)?;
    let sink = ProductDiagnosticSink::at_path(&client, &PathBuf::from(output))?;
    Ok(Some(Box::new(sink)))
}

impl Preflight {
    pub(in crate::driver) fn run_compatible_70425(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        self.run_compatible(store, run, selected, context, 70425)
    }
    pub(in crate::driver) fn run_compatible_70450(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        self.run_compatible(store, run, selected, context, 70450)
    }
    fn run_compatible(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
        expected_build: u32,
    ) -> Result<(), Error> {
        if context.build != expected_build {
            return Err("product_build_mismatch".into());
        }
        let result = if expected_build == 70450 {
            match ProductDiagnosticSink::from_environment(&self.client) {
                Ok(diagnostic) => {
                    store.set_product_diagnostic_observer(
                        diagnostic.map(|sink| Box::new(sink) as Box<dyn ProductDiagnosticObserver>),
                    );
                    store.set_product_diagnostic_phase(ProductDiagnosticPhase::Startup);
                    self.compatible_inner(store, run, selected, context)
                }
                Err(error) => Err(error),
            }
        } else {
            self.compatible_inner(store, run, selected, context)
        };
        // All transient/final compatible owners have dropped before these last
        // independent guards, including on allocation/setup/operation failure.
        let source = store.verify_unchanged().map_err(|e| Error::from(e.code()));
        let source_guard_changed = source.as_ref().err().is_some_and(|error| {
            error.code == forever_casc::local_store::Error::InputChanged.code()
        });
        let guard = self.verify_unchanged();
        let priority = result.as_ref().err().is_some_and(|e| {
            e.code == forever_casc::local_store::Error::InputChanged.code()
                || e.code == forever_casc::collision::payload::Error::Changed.code()
        });
        let result = if source.is_err() {
            source
        } else if priority {
            result
        } else {
            guard.and(result)
        };
        store.finish_product_diagnostic(
            result.as_ref().err().map(|error| error.code),
            source_guard_changed,
        );
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn compatible_inner(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        // A sealed resume still rebuilds the genuine six-owner result, then
        // compares every artifact against the immutable published component.
        let sealed = run.completed_components()? & Component::Collision.bit() != 0;
        // Reserved before the first startup read; all subsequent phases receive
        // the reduced ceiling. Common Run and guard charges remain disjoint.
        let combined = COMBINED
            .checked_sub(coverage::HEAP)
            .ok_or("product_disk_limit")?;
        let mut encoded = coverage::Encoder::new()?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(coverage::MAX_MAPS)
            .map_err(|_| "product_allocation")?;
        if ids.capacity() > coverage::MAX_MAPS {
            return Err("product_disk_limit".into());
        }
        let Repositories {
            namespace,
            raw,
            wdt,
            cache,
            placements,
            vmo,
            maps,
        } = self.repositories()?;
        let resources = resources::Profile::DiskWorldV1;
        let wdts = inventory_spool::Inventory::new(wdt, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        let caches = inventory_spool::Inventory::new(cache, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.startup_limits();
        limits.collision.max_memory_bytes = limits.collision.max_memory_bytes.min(combined);
        store.set_product_diagnostic_phase(ProductDiagnosticPhase::Startup);
        let mut startup = match context.build {
            70425 => startup::extract_startup_fdid_native_gnu15_70425(store, limits, raw),
            70450 => startup::extract_startup_fdid_native_gnu15_70450(store, limits, raw),
            _ => return Err("product_build_mismatch".into()),
        }
        .map_err(|e| Error::from(e.code()))?;
        let old = resources.stage_limits();
        let limits = placement::Limits {
            world: forever_casc::collision::compatible::world::inventory::Limits {
                world: old.world.world,
                max_combined_memory_bytes: combined,
            },
            max_heap_bytes: old.max_heap_bytes,
            max_callbacks: old.max_callbacks,
            max_records: old.max_records,
            records: old.records,
        };
        startup.set_product_diagnostic_phase(ProductDiagnosticPhase::Placements);
        let stage = placement::extract_selected_with_coverage(
            startup,
            wdts,
            caches,
            placements,
            selected.map_or(MapSelection::All, MapSelection::MapAndAncestors),
            limits,
        )
        .map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.model_limits();
        store.set_product_diagnostic_phase(ProductDiagnosticPhase::Models);
        limits.max_memory_bytes = limits.max_memory_bytes.min(combined);
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(usize::try_from(self.budget.output_bytes).map_err(|_| "product_disk_limit")?);
        let models = models::prepare(stage, vmo, limits).map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.package_limits();
        store.set_product_diagnostic_phase(ProductDiagnosticPhase::Assembly);
        limits.max_memory_bytes = limits.max_memory_bytes.min(combined);
        limits.max_files = limits.max_files.min(262144);
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(usize::try_from(self.budget.output_bytes).map_err(|_| "product_disk_limit")?);
        let mut done =
            package::assemble(models, maps, limits).map_err(|e| Error::from(e.code()))?;
        done.with_coverage(0,|view|{encoded.capture(context,selected,view)?;for m in view.maps().iter().filter(|m|m.wdt()==forever_casc::collision::run::compatible_inventory::coverage::Wdt::Present){if ids.len()==ids.capacity(){return Err("product_disk_limit".into());}ids.push(m.id());}Ok::<_,Error>(())}).map_err(failure)?;
        // First traversal records the complete final inventory before any put.
        // Proofs authenticate complete ID order; they are not private envelopes.
        for &id in &ids {
            done.with_map(id, 0, |m| {
                let b = m.ordered_dir_bin();
                encoded.artifact(
                    &coverage::proof_path(id),
                    b.len() + 16,
                    coverage::proof_digest(id, b)?,
                )
            })
            .map_err(failure)?;
        }
        done.visit(0, |a| {
            encoded.artifact(
                &format!("vmaps/{}", a.name()),
                a.bytes().len(),
                integrity::digest(a.bytes()),
            )
        })
        .map_err(failure)?;
        encoded.finish()?;
        let (files, bytes) = encoded.totals();
        if files > 262144 || bytes > self.budget.output_bytes {
            return Err("product_disk_limit".into());
        }
        for &id in &ids {
            done.with_map(id, coverage::MAX_BYTES, |m| {
                let raw = m.ordered_dir_bin();
                let size = raw.len().checked_add(16).ok_or("product_disk_limit")?;
                if size > coverage::MAX_BYTES {
                    return Err("product_disk_limit".into());
                }
                let mut b = Vec::new();
                b.try_reserve_exact(size)
                    .map_err(|_| "product_allocation")?;
                if b.capacity() > size {
                    return Err("product_disk_limit".into());
                }
                b.extend_from_slice(&coverage::proof_header(id, raw.len())?);
                b.extend_from_slice(raw);
                self.verify_unchanged()?;
                namespace.check()?;
                put_collision(run, sealed, &coverage::proof_path(id), &b)?;
                self.verify_unchanged()?;
                namespace.check()?;
                Ok::<_, Error>(())
            })
            .map_err(failure)?;
        }
        done.visit(0, |a| {
            self.verify_unchanged()?;
            namespace.check()?;
            put_collision(run, sealed, &format!("vmaps/{}", a.name()), a.bytes())?;
            self.verify_unchanged()?;
            namespace.check()?;
            Ok::<_, Error>(())
        })
        .map_err(failure)?;
        self.verify_unchanged()?;
        namespace.check()?;
        put_collision(run, sealed, coverage::PATH, encoded.finish()?)?;
        done.verify_unchanged().map_err(|e| Error::from(e.code()))?;
        namespace.check()?;
        self.verify_unchanged()
    }
}
#[cfg(all(
    test,
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
mod tests;

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use forever_casc::local_store::{CandidateFailures, ProductDiagnosticRequestKind};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Lab(PathBuf);
    impl Lab {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rce-product-source-diagnostic-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("client")).unwrap();
            fs::create_dir(root.join("reports")).unwrap();
            Self(root)
        }
        fn client(&self) -> anchors::Chain {
            anchors::Chain::open(&self.0.join("client"), true).unwrap()
        }
        fn output(&self) -> PathBuf {
            self.0.join("reports/first-source.json")
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn unset_diagnostic_option_creates_no_file() {
        let lab = Lab::new();
        let sink = ProductDiagnosticSink::from_optional_output(&lab.client(), None).unwrap();
        assert!(sink.is_none());
        assert!(!lab.output().exists());
    }

    #[test]
    fn tolerated_absence_then_terminal_failure_writes_only_the_propagated_cause() {
        let lab = Lab::new();
        let mut sink = ProductDiagnosticSink::at_path(&lab.client(), &lab.output()).unwrap();
        sink.set_phase(ProductDiagnosticPhase::Startup);
        let absent = sink
            .begin_request(ProductDiagnosticRequestKind::GameObjectModel, Some(2198569))
            .unwrap();
        assert_eq!(absent.ordinal, 0);
        sink.tolerated_absence(absent);

        sink.set_phase(ProductDiagnosticPhase::Placements);
        let request = sink
            .begin_request(ProductDiagnosticRequestKind::Wdt, Some(1349477))
            .unwrap();
        assert_eq!(request.ordinal, 0);
        sink.source_failure(
            request,
            StoreError::NoLocalEncoding(CandidateFailures {
                missing: 1,
                incomplete: 2,
                encrypted: 3,
                corrupt: 4,
                unsupported: 5,
            }),
            false,
        );
        assert!(!lab.output().exists());
        sink.finish(Some("local_store_no_verified_encoding"), false);
        let first = fs::read_to_string(lab.output()).unwrap();
        assert!(
            sink.begin_request(ProductDiagnosticRequestKind::TileAdt, Some(9001),)
                .is_none()
        );
        sink.source_failure(request, StoreError::MissingFile, false);
        sink.finish(Some("file_unavailable"), false);
        assert_eq!(fs::read_to_string(lab.output()).unwrap(), first);
        assert_eq!(first.lines().count(), 1);
        assert_eq!(
            first,
            "{\"phase\":\"placements\",\"request_kind\":\"wdt\",\"file_data_id\":1349477,\"ordinal\":0,\"typed_cause\":{\"variant\":\"NoLocalEncoding\",\"candidate_failures\":{\"missing\":1,\"incomplete\":2,\"encrypted\":3,\"corrupt\":4,\"unsupported\":5}},\"code\":\"local_store_no_verified_encoding\",\"guards_observed\":false,\"tolerated_absences\":{\"db2_table\":0,\"wdt\":0,\"tile_adt\":0,\"root_m2\":0,\"wmo_root\":0,\"wmo_group\":0,\"child\":0,\"gameobject_model\":1,\"other\":0}}\n"
        );
        for forbidden in [
            "client-root",
            "path",
            "content_key",
            "encoded_key",
            "bytes",
            "payload",
        ] {
            assert!(
                !first.contains(forbidden),
                "unexpected diagnostic field {forbidden}"
            );
        }
    }

    #[test]
    fn all_tolerated_absences_finish_without_a_terminal_record() {
        let lab = Lab::new();
        let mut sink = ProductDiagnosticSink::at_path(&lab.client(), &lab.output()).unwrap();
        let request = sink
            .begin_request(ProductDiagnosticRequestKind::GameObjectModel, Some(42))
            .unwrap();
        sink.tolerated_absence(request);
        sink.finish(None, false);
        assert!(!lab.output().exists());
    }

    #[test]
    fn diagnostic_output_inside_client_root_is_refused_without_creation() {
        let lab = Lab::new();
        let client_path = lab.0.join("client");
        let client = anchors::Chain::open(&client_path, true).unwrap();
        let output = client_path.join("diagnostic.json");
        let result = ProductDiagnosticSink::at_path(&client, &output);
        assert!(matches!(result, Err(error) if error.code == "product_diagnostic_path"));
        assert!(!output.exists());
    }
}
