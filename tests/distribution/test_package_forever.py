# SPDX-License-Identifier: GPL-3.0-or-later
"""Synthetic distribution integrity checks. No compiler, network or client I/O."""
import copy
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

TOOL_PATH = Path(__file__).resolve().parents[2] / "tools/package_forever.py"
spec = importlib.util.spec_from_file_location("package_forever", TOOL_PATH)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
REPO = TOOL_PATH.parent.parent


def synthetic_lock():
    """Authored lock fixture, independent of concurrently edited workstation lock."""
    dependencies = {
        "forever-casc": ["forever-db2", "forever-platform", "forever-terrain", "forever-vmap", "miniz_oxide"],
        "forever-db2": [],
        "forever-extractor": ["forever-casc", "forever-db2", "forever-navigation", "forever-platform", "forever-terrain", "forever-vmap"],
        "forever-navigation": ["forever-db2", "forever-platform", "forever-terrain", "forever-vmap"],
        "forever-platform": [], "forever-terrain": [], "forever-vmap": ["forever-platform"],
    }
    lines = ["version = 4"]
    for name, deps in sorted(dependencies.items()):
        lines += ["", "[[package]]", "name = " + json.dumps(name), 'version = "0.1.0"']
        if deps:
            lines += ["dependencies = " + json.dumps(deps)]
    for key in sorted(m.DEPENDENCIES):
        name, version = key.rsplit("-", 1)
        lines += ["", "[[package]]", "name = " + json.dumps(name), "version = " + json.dumps(version), 'source = "registry+https://github.com/rust-lang/crates.io-index"', "checksum = " + json.dumps(m.DEPENDENCIES[key])]
        if name == "miniz_oxide":
            lines += ['dependencies = ["adler2"]']
    return "\n".join(lines) + "\n"


def crate_bytes(key, extra=None):
    files = {"Cargo.toml": b"[package]\n", "LICENSE": b"Original synthetic notice\n", "src/lib.rs": b"// synthetic\n"}
    if extra:
        files.update(extra)
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w:gz") as archive:
        for name, data in sorted(files.items()):
            info = tarfile.TarInfo(key + "/" + name)
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))
    return raw.getvalue()


class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="forever-dist-synthetic-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "checkout"
        self.source.mkdir()
        self.cache = self.root / "cache"
        self.cache.mkdir()
        dependencies = {}
        for key in m.DEPENDENCIES:
            data = crate_bytes(key)
            (self.cache / (key + ".crate")).write_bytes(data)
            dependencies[key] = m.digest(data)
        self.notice_payloads = {name: ("Original authored synthetic notice:" + name + "\n").encode() for name in m.TOOLCHAIN_NOTICES}
        notice_pins = {name: m.digest(data) for name, data in self.notice_payloads.items()}
        notice_patch = patch.object(m, "TOOLCHAIN_NOTICES", notice_pins)
        notice_patch.start()
        self.addCleanup(notice_patch.stop)
        notice_loader = patch.object(m, "read_toolchain_notices", return_value=self.notice_payloads)
        notice_loader.start()
        self.addCleanup(notice_loader.stop)
        self.dependencies = dependencies
        self.patch = patch.object(m, "DEPENDENCIES", dependencies)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        for name in m.REQUIRED:
            p = self.source / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(b"authored synthetic source\n")
        (self.source / "Cargo.lock").write_text(synthetic_lock())
        (self.source / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.98.0"\n')
        for crate in m.CRATES:
            p = self.source / "crates" / crate
            (p / "src").mkdir(parents=True, exist_ok=True)
            (p / "Cargo.toml").write_text("[package]\n")
            (p / "src/lib.rs").write_text("// synthetic\n")
        for name in ("crates/forever-extractor/src/main.rs", "crates/forever-casc/build.rs"):
            (self.source / name).write_text("// synthetic\n")
        for directory in ("contracts", "schemas", "tests/forever", "tests/contracts", "LICENSES", "docs/licensing"):
            (self.source / directory).mkdir(parents=True, exist_ok=True)
        (self.source / "LICENSES/MIT.txt").write_text("Original notice\n")

    @staticmethod
    def compiler(output, epoch):
        m.write_new(output / "bin/forever-extractor", b"\x7fELFauthored-test-binary", 0o755)
        m.write_new(output / "build.log", b"synthetic build\n")
        return {"rustc": "rustc 1.98.0 (synthetic)\nhost: x86_64-unknown-linux-gnu", "cargo": "cargo 1.98.0 (synthetic)"}, "x86_64-unknown-linux-gnu"

    def build(self, name="output", compiler=None):
        with patch.object(m, "compile_source", compiler or self.compiler):
            return m.build_bundle(self.source, self.root / name, self.cache, 1234)

    def assertFailure(self, code, call, *args):
        with self.assertRaisesRegex(m.Failure, "^" + code + "$"):
            call(*args)

    def test_reproducible_bundle_and_moved_rebuild(self):
        a = self.build("a")
        b = self.build("b")
        self.assertEqual(a, b)
        archive = self.root / "a/forever-extractor-linux.tar.gz"
        manifest = m.verify_archive(archive)
        self.assertFalse(manifest["full_extractor_complete"])
        self.assertEqual(manifest["source_snapshot"]["profile"], m.SOURCE_PROFILE)
        unpacked = self.root / "moved"
        unpacked.mkdir()
        # Test archive has already been checked; manually write regular members.
        with tarfile.open(archive) as stream:
            for item in stream:
                m.write_new(unpacked / item.name, stream.extractfile(item).read(), item.mode)
        with patch.object(m, "compile_source", self.compiler):
            rebuilt = m.rebuild_bundle(unpacked, self.root / "rebuilt")
        self.assertEqual(rebuilt["binary_sha256"], a["binary_sha256"])

    def test_dirty_current_source_is_snapshotted(self):
        path = self.source / "crates/forever-extractor/src/lib.rs"
        path.write_text("// uncommitted synthetic edit\n")
        self.build()
        self.assertEqual((self.root / "output/source/crates/forever-extractor/src/lib.rs").read_bytes(), path.read_bytes())

    def test_source_change_during_build_rejects_and_retains_output(self):
        def compile_changed(output, epoch):
            result = self.compiler(output, epoch)
            (self.source / "LICENSE").write_text("changed\n")
            return result
        self.assertFailure("input_changed", self.build, "output", compile_changed)
        self.assertTrue((self.root / "output/build.log").exists())

    def test_frozen_source_change_rejects(self):
        def compile_changed(output, epoch):
            result = self.compiler(output, epoch)
            (output / "source/LICENSE").write_text("changed\n")
            return result
        self.assertFailure("frozen_source_changed", self.build, "output", compile_changed)

    def test_frozen_source_added_pycache_rejects(self):
        def compile_changed(output, epoch):
            result = self.compiler(output, epoch)
            (output / "source/__pycache__").mkdir()
            (output / "source/__pycache__/unlisted.pyc").write_bytes(b"not source")
            return result
        self.assertFailure("frozen_source_changed", self.build, "output", compile_changed)

    def test_existing_output_rejected(self):
        (self.root / "output").mkdir()
        self.assertFailure("output_exists", self.build)

    def test_output_in_git_rejected(self):
        (self.root / ".git").mkdir()
        self.assertFailure("output_inside_git", self.build)

    def test_output_inside_source_rejected(self):
        self.assertFailure("output_inside_source", m.new_output, self.source / "new", self.source)

    def test_source_symlink_rejected(self):
        path = self.source / "LICENSE"
        path.unlink()
        path.symlink_to(self.source / "AUTHORS")
        self.assertFailure("input_symlink", self.build)

    def test_source_directory_symlink_rejected(self):
        (self.source / "tests/forever/linked").symlink_to(self.source / "contracts", target_is_directory=True)
        self.assertFailure("input_symlink", self.build)

    def test_unapproved_source_extension_rejected(self):
        (self.source / "tests/forever/client.db2").write_bytes(b"not admitted")
        self.assertFailure("source_extension", self.build)

    def test_source_file_budget_rejected(self):
        with patch.object(m, "MAX_FILE", 1):
            self.assertFailure("input_budget", self.build)

    def test_wrong_registry_checksum_rejected(self):
        (self.cache / "adler2-2.0.1.crate").write_bytes(b"wrong")
        self.assertFailure("dependency_checksum", self.build)

    def test_missing_dependency_rejected(self):
        (self.cache / "adler2-2.0.1.crate").unlink()
        self.assertFailure("dependency_missing", self.build)

    def test_wrong_lock_rejected(self):
        p = self.source / "Cargo.lock"
        p.write_text(p.read_text().replace('version = "0.8.9"', 'version = "0.8.8"'))
        self.assertFailure("lock_checksum", self.build)

    def test_extra_lock_package_rejected(self):
        p = self.source / "Cargo.lock"
        p.write_text(p.read_text() + '\n[[package]]\nname = "other"\nversion = "1"\n')
        self.assertFailure("lock_closure", self.build)

    def test_archive_link_traversal_duplicate_and_special_rejected(self):
        for name, kind, code in (("../bad", tarfile.REGTYPE, "unsafe_path"), ("adler2-2.0.1/link", tarfile.SYMTYPE, "archive_type"), ("adler2-2.0.1/pipe", tarfile.FIFOTYPE, "archive_type")):
            with self.subTest(name=name):
                raw = io.BytesIO()
                with tarfile.open(fileobj=raw, mode="w:gz") as archive:
                    item = tarfile.TarInfo(name)
                    item.type = kind
                    archive.addfile(item)
                self.assertFailure(code, m.safe_crate_entries, raw.getvalue(), "adler2-2.0.1")
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w:gz") as archive:
            for _ in range(2):
                archive.addfile(tarfile.TarInfo("adler2-2.0.1/Cargo.toml"))
        self.assertFailure("archive_duplicate", m.safe_crate_entries, raw.getvalue(), "adler2-2.0.1")

    def test_archive_declared_budget_rejected(self):
        with patch.object(m, "MAX_FILE", 1):
            self.assertFailure("archive_budget", m.safe_crate_entries, crate_bytes("adler2-2.0.1"), "adler2-2.0.1")

    def test_payload_tamper_rejected(self):
        self.build()
        path = self.root / "output/forever-extractor-linux.tar.gz"
        # Rewrite one file, retaining original manifest and metadata.
        raw = io.BytesIO()
        import gzip
        with tarfile.open(path) as old, gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=1234, compresslevel=9) as zipped, tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as new:
            for item in old:
                data = old.extractfile(item).read()
                if item.name == "source/LICENSE":
                    data += b"tamper"
                    item.size = len(data)
                new.addfile(item, io.BytesIO(data))
        path.write_bytes(raw.getvalue())
        self.assertFailure("inventory_hash", m.verify_archive, path)

    def test_unknown_manifest_field_rejected(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        manifest["secret"] = "not permitted"
        self.assertFailure("manifest_shape", m.validate_manifest, manifest)

    def test_unsafe_and_unknown_bundle_paths_rejected(self):
        for name in ("/absolute", "a/../b", "a\\b", "a//b"):
            self.assertFailure("unsafe_path", m.bundle_path_allowed, name)
        self.assertFalse(m.bundle_path_allowed("source/client/asset.db2"))
        self.assertFalse(m.bundle_path_allowed("source/.opencode/environment.sh"))


    def test_duplicate_json_and_missing_source_rejected(self):
        self.assertFailure("json_duplicate", m.read_json, b'{"a":1,"a":2}')
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        manifest["inventory"].pop("source/crates/forever-casc/build.rs")
        self.assertFailure("manifest_missing", m.validate_manifest, manifest)

    def test_manifest_and_source_total_budgets_reject(self):
        with patch.object(m, "MAX_TOTAL", 1):
            self.assertFailure("source_budget", self.build)
        self.build("good")
        with patch.object(m, "MAX_MANIFEST", 1):
            self.assertFailure("manifest_budget", m.verify_archive, self.root / "good/forever-extractor-linux.tar.gz")

    def test_special_source_file_rejected_without_blocking(self):
        import os
        os.mkfifo(self.source / "tests/forever/fifo.py")
        self.assertFailure("input_type", self.build)

    def test_ambient_cargo_config_rejected_before_compiler(self):
        output = self.root / "ambient"
        output.mkdir()
        (self.root / ".cargo").mkdir()
        (self.root / ".cargo/config.toml").write_text("# synthetic ambient config\n")
        self.assertFailure("ambient_cargo_config", m.compile_source, output, 1234)


    def unpack(self, name="unpacked"):
        self.build("built")
        root = self.root / name
        root.mkdir()
        with tarfile.open(self.root / "built/forever-extractor-linux.tar.gz") as archive:
            for member in archive:
                m.write_new(root / member.name, archive.extractfile(member).read(), member.mode)
        return root

    def test_unpacked_binary_and_manifest_modes_rejected(self):
        root = self.unpack()
        (root / "bin/forever-extractor").chmod(0o600)
        self.assertFailure("bundle_mode", m.verify_bundle, root)
        (root / "bin/forever-extractor").chmod(0o755)
        (root / "distribution-manifest.json").chmod(0o600)
        self.assertFailure("bundle_mode", m.verify_bundle, root)

    def test_unpacked_pycache_and_unknown_empty_directory_rejected(self):
        root = self.unpack()
        (root / "source/__pycache__").mkdir()
        (root / "source/__pycache__/unlisted.bin").write_bytes(b"not admitted")
        self.assertFailure("bundle_topology", m.verify_bundle, root)
        (root / "source/__pycache__/unlisted.bin").unlink()
        (root / "source/__pycache__").rmdir()
        (root / "unknown-empty").mkdir()
        self.assertFailure("bundle_topology", m.verify_bundle, root)

    def test_unpacked_manifest_budget_checked_before_payload_read(self):
        root = self.unpack()
        real_read = m.regular_bytes
        reads = []
        def observed(path, limit=None):
            reads.append(Path(path).name)
            return real_read(path, limit)
        with patch.object(m, "MAX_TOTAL", 1), patch.object(m, "regular_bytes", observed):
            self.assertFailure("manifest_budget", m.verify_bundle, root)
        self.assertEqual(reads, ["distribution-manifest.json"])

    def test_unpacked_unknown_file_rejected_before_read(self):
        root = self.unpack()
        unknown = root / "source/client.db2"
        unknown.write_bytes(b"not admitted")
        real_read = m.regular_bytes
        def observed(path, limit=None):
            self.assertNotEqual(Path(path), unknown)
            return real_read(path, limit)
        with patch.object(m, "regular_bytes", observed):
            self.assertFailure("inventory_mismatch", m.verify_bundle, root)

    def test_archive_second_gzip_member_rejected(self):
        import gzip
        self.build()
        path = self.root / "output/forever-extractor-linux.tar.gz"
        path.write_bytes(path.read_bytes() + gzip.compress(b"", mtime=999))
        self.assertFailure("gzip_trailing", m.verify_archive, path)

    def test_archive_hidden_gnu_header_rejected(self):
        import gzip
        self.build()
        path = self.root / "output/forever-extractor-linux.tar.gz"
        original = gzip.decompress(path.read_bytes())
        with tarfile.open(fileobj=io.BytesIO(original)) as archive:
            first = archive.next()
        hidden = tarfile.TarInfo("././@LongLink")
        hidden.type = tarfile.GNUTYPE_LONGNAME
        data = first.name.encode() + b"\x00"
        hidden.size = len(data)
        hidden.uid, hidden.gid, hidden.mtime = 77, 88, 999
        prefix = hidden.tobuf(format=tarfile.GNU_FORMAT) + data + bytes((-len(data)) % 512)
        # Restore canonical record padding after inserting extension records.
        raw = (prefix + original).rstrip(b"\x00")
        raw += bytes(((-len(raw) - 1024) % 10240) + 1024)
        path.write_bytes(gzip.compress(raw, compresslevel=9, mtime=1234))
        with self.assertRaises(m.Failure) as rejected:
            m.verify_archive(path)
        self.assertIn(str(rejected.exception), ("archive_framing", "archive_trailing"))

    def test_inventory_file_prefix_collision_rejected(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        name = "source/crates/forever-db2/src/lib.rs/child.rs"
        manifest["inventory"][name] = {"bytes": 0, "sha256": m.digest(b""), "mode": 0o644}
        self.assertFailure("inventory_path_collision", m.validate_manifest, manifest)

    def test_encoded_rustflags_preserve_space_in_output_path(self):
        root = self.root / "build path with spaces"
        env = m.build_env(root, {"rustc": "/installed/rustc", "cargo": "/installed/cargo"}, 1234)
        self.assertNotIn("RUSTFLAGS", env)
        self.assertEqual(env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f"), ["--remap-path-prefix=" + str(root) + "=/forever-distribution", "-C", "debuginfo=0", "-C", "strip=debuginfo", "-C", "codegen-units=1"])


    def notice_root(self):
        root = self.root / "sysroot/share/doc/rust"
        for name, data in self.notice_payloads.items():
            m.write_new(root / name, data)
        return root

    def test_third_party_and_legacy_provenance_are_mandatory(self):
        self.build()
        for name in ("THIRD_PARTY_NOTICES.md", "licensing/import-manifest.json", "licensing/inventory.json", "licensing/source-files.json"):
            self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
        (self.source / "THIRD_PARTY_NOTICES.md").unlink()
        self.assertFailure("input_io", self.build, "missing-third")

    def test_exact_original_toolchain_notices_are_retained(self):
        root = self.notice_root()
        self.assertEqual(m.notice_documents(root), self.notice_payloads)
        self.build()
        manifest = m.verify_archive(self.root / "output/forever-extractor-linux.tar.gz")
        self.assertEqual(manifest["toolchain_notices"], m.TOOLCHAIN_NOTICES)
        for name, data in self.notice_payloads.items():
            self.assertEqual((self.root / "output/toolchain-notices" / name).read_bytes(), data)

    def test_missing_changed_and_extra_toolchain_notice_reject(self):
        root = self.notice_root()
        copyright = root / "COPYRIGHT-library.html"
        copyright.unlink()
        self.assertFailure("toolchain_notice_missing", m.notice_documents, root)
        copyright.write_bytes(b"changed")
        self.assertFailure("toolchain_notice_checksum", m.notice_documents, root)
        copyright.write_bytes(self.notice_payloads["COPYRIGHT-library.html"])
        (root / "licenses/Extra.txt").write_bytes(b"not pinned")
        self.assertFailure("toolchain_notice_closure", m.notice_documents, root)

    def test_toolchain_notice_symlink_rejected(self):
        root = self.notice_root()
        path = root / "licenses/MIT.txt"
        path.unlink()
        path.symlink_to(root / "COPYRIGHT-library.html")
        self.assertFailure("toolchain_notice_type", m.notice_documents, root)

    def test_manifest_cannot_remove_or_repin_toolchain_notices(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        changed = copy.deepcopy(manifest)
        changed["toolchain_notices"]["COPYRIGHT-library.html"] = "0" * 64
        self.assertFailure("manifest_toolchain_notices", m.validate_manifest, changed)
        changed = copy.deepcopy(manifest)
        changed["inventory"].pop("toolchain-notices/COPYRIGHT-library.html")
        self.assertFailure("manifest_missing", m.validate_manifest, changed)

    def test_moved_rebuild_needs_only_bundled_notices(self):
        root = self.unpack()
        with patch.object(m, "read_toolchain_notices", side_effect=AssertionError("external notices must not be opened")), patch.object(m, "compile_source", self.compiler):
            m.rebuild_bundle(root, self.root / "rebuilt-notices")
        for name, data in self.notice_payloads.items():
            self.assertEqual((self.root / "rebuilt-notices/toolchain-notices" / name).read_bytes(), data)

    def test_control_character_output_path_cannot_inject_encoded_flags(self):
        self.assertFailure("unsafe_output_path", m.new_output, self.root / "output\x1f-Copt-level=0", self.source)

    def test_seven_crate_ci_source_closure_is_shipped(self):
        self.build()
        required = ("crates/forever-platform/Cargo.toml", "crates/forever-platform/src/lib.rs", ".github/workflows/forever-rust.yml", "docs/forever-ci.md", "tests/contracts/test_forever_ci_contract.py")
        for name in required:
            self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
        self.assertFalse(m.bundle_path_allowed("source/.github/workflows/unreviewed.yml"))
        self.assertFalse(m.bundle_path_allowed("source/.github/secrets.json"))

    def test_current_selected_source_paths_are_all_admitted(self):
        selected = m.selected_paths(REPO)
        self.assertTrue(selected)
        for name in selected:
            with self.subTest(name=name):
                self.assertTrue(m.bundle_path_allowed("source/" + name))

    def test_current_ci_optional_inputs_are_shipped_and_moved(self):
        names = m.OPTIONAL_CI_DOCS + m.OPTIONAL_CI_SOURCE
        self.assertEqual(len(names), 8)
        self.assertTrue(set(names).issubset(m.selected_paths(REPO)))
        for name in names:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(("authored current CI input:" + name + "\n").encode())
            self.assertTrue(m.bundle_path_allowed("source/" + name))
        root = self.unpack("current-ci")
        manifest, _ = m.verify_bundle(root)
        for name in names:
            with self.subTest(name=name):
                self.assertIn(name, manifest["source_snapshot"]["files"])
                self.assertEqual((root / "source" / name).read_bytes(),
                                 (self.source / name).read_bytes())
        with patch.object(m, "compile_source", self.compiler):
            rebuilt = m.rebuild_bundle(root, self.root / "rebuilt-current-ci")
        self.assertEqual(rebuilt["binary_sha256"],
                         manifest["inventory"]["bin/forever-extractor"]["sha256"])

    def test_current_ci_optional_absence_preserves_frozen_rebuild(self):
        names = m.OPTIONAL_CI_DOCS + m.OPTIONAL_CI_SOURCE
        self.assertTrue(all(not (self.source / name).exists() for name in names))
        root = self.unpack("older-ci")
        manifest, _ = m.verify_bundle(root)
        self.assertTrue(all("source/" + name not in manifest["inventory"]
                            for name in names))
        with patch.object(m, "compile_source", self.compiler):
            rebuilt = m.rebuild_bundle(root, self.root / "rebuilt-older-ci")
        self.assertEqual(rebuilt["binary_sha256"],
                         manifest["inventory"]["bin/forever-extractor"]["sha256"])

    def test_current_ci_optional_allowlist_is_exact(self):
        self.assertEqual(m.OPTIONAL_CI_DOCS, (
            "docs/forever-linux-compatibility.md", "docs/forever-terrain-v12.md",
            "docs/forever-product-v14.md", "docs/forever-windows-membership.md",
            "docs/forever-windows-output.md", "docs/forever-product-v20.md",
            "docs/forever-key-investigation.md"))
        self.assertEqual(m.OPTIONAL_CI_SOURCE, ("tools/generate_forever_catalog.py",))
        for name in ("source/docs/forever-linux-compatibility-extra.md",
                     "source/docs/forever-terrain-v12-extra.md",
                     "source/docs/forever-product-v14-extra.md",
                      "source/docs/forever-windows-membership-extra.md",
                      "source/docs/forever-windows-output-extra.md",
                      "source/docs/forever-product-v20-extra.md",
                      "source/docs/forever-key-investigation-extra.md",
                      "source/tools/generate_forever_catalog_extra.py"):
            with self.subTest(name=name):
                self.assertFalse(m.bundle_path_allowed(name))

    def test_v20_and_key_investigation_guides_are_optional_bundle_inputs(self):
        names = (
            "docs/forever-product-v20.md",
            "docs/forever-key-investigation.md",
        )
        self.assertTrue(set(names).issubset(m.OPTIONAL_CI_DOCS))
        self.assertTrue(set(names).issubset(m.selected_paths(REPO)))
        for name in names:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(("authored synthetic guide:" + name + "\n").encode())
            self.assertTrue(m.bundle_path_allowed("source/" + name))
        self.build("v20-guides")
        manifest = m.verify_archive(
            self.root / "v20-guides/forever-extractor-linux.tar.gz"
        )
        for name in names:
            with self.subTest(name=name):
                self.assertIn(name, manifest["source_snapshot"]["files"])
                self.assertEqual(
                    (self.root / "v20-guides/source" / name).read_bytes(),
                    (self.source / name).read_bytes(),
                )

    def test_optional_windows_qa_source_is_shipped_when_present(self):
        for name in m.OPTIONAL_WINDOWS_QA_SOURCE:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(("authored optional Windows QA:" + name + "\n").encode())
        self.build()
        manifest = m.verify_archive(self.root / "output/forever-extractor-linux.tar.gz")
        self.assertEqual(manifest["source_snapshot"]["profile"], m.SOURCE_PROFILE)
        for name in m.OPTIONAL_WINDOWS_QA_SOURCE:
            with self.subTest(name=name):
                self.assertIn(name, manifest["source_snapshot"]["files"])
                self.assertEqual((self.root / "output/source" / name).read_bytes(),
                                 (self.source / name).read_bytes())

    def test_optional_windows_qa_source_absence_preserves_v10_rebuild(self):
        self.assertTrue(all(not (self.source / name).exists() for name in m.OPTIONAL_WINDOWS_QA_SOURCE))
        root = self.unpack("legacy-v10")
        manifest, _ = m.verify_bundle(root)
        self.assertEqual(manifest["source_snapshot"]["profile"], m.SOURCE_PROFILE)
        self.assertTrue(all("source/" + name not in manifest["inventory"]
                            for name in m.OPTIONAL_WINDOWS_QA_SOURCE))
        with patch.object(m, "compile_source", self.compiler):
            rebuilt = m.rebuild_bundle(root, self.root / "rebuilt-legacy-v10")
        self.assertEqual(rebuilt["binary_sha256"], manifest["inventory"]["bin/forever-extractor"]["sha256"])

    def test_optional_windows_qa_allowlist_is_exact(self):
        self.assertEqual(m.OPTIONAL_WINDOWS_QA_SOURCE, (
            "docs/forever-windows-product.md", "tools/check_forever_windows_product.ps1",
            "docs/forever-windows-producer-msvc-v2.md"))
        for name in ("source/docs/forever-windows-product-extra.md",
                     "source/tools/check_forever_windows_product_extra.ps1",
                     "source/docs/forever-windows-producer-msvc-v3.md",
                     "source/tools/unreviewed.ps1", "bin/forever-extractor.exe"):
            with self.subTest(name=name):
                self.assertFalse(m.bundle_path_allowed(name))

    def test_platform_crate_and_exact_ci_workflow_are_required(self):
        platform = self.source / "crates/forever-platform/Cargo.toml"
        platform.unlink()
        self.assertFailure("source_crate_missing", self.build)
        platform.write_text("[package]\n")
        (self.source / ".github/workflows/forever-rust.yml").unlink()
        self.assertFailure("input_io", self.build)

    def test_platform_registry_dependency_and_older_closure_rejected(self):
        lock = synthetic_lock()
        changed = lock.replace('name = "forever-platform"\nversion = "0.1.0"', 'name = "forever-platform"\nversion = "0.1.0"\ndependencies = ["adler2"]')
        self.assertFailure("lock_closure", m.validate_lock, changed.encode())
        older = lock.replace('\n[[package]]\nname = "forever-platform"\nversion = "0.1.0"\n', "\n")
        self.assertFailure("lock_closure", m.validate_lock, older.encode())

    def test_source_profile_is_versioned_and_exact(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        self.assertEqual(manifest["schema"], "forever-distribution-v1")
        manifest["source_snapshot"].pop("profile")
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)
        manifest["source_snapshot"]["profile"] = "forever-six-crate-source-v1"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)
        manifest["source_snapshot"]["profile"] = "forever-seven-crate-ci-source-v2"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)
        manifest["source_snapshot"]["profile"] = "forever-seven-crate-windows-input-source-v3"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

        manifest["source_snapshot"]["profile"] = "forever-seven-crate-windows-producer-source-v4"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)
        manifest["source_snapshot"]["profile"] = "forever-seven-crate-windows-vmap-diagnostics-source-v5"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

        manifest["source_snapshot"]["profile"] = "forever-seven-crate-windows-vmap-required-framing-source-v6"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

        manifest["source_snapshot"]["profile"] = "forever-seven-crate-windows-navigation-readers-source-v7"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

        manifest["source_snapshot"]["profile"] = "forever-seven-crate-linux-collision-leased-archive-source-v8"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

        manifest["source_snapshot"]["profile"] = "forever-seven-crate-linux-product-db2-gt-cameras-source-v11"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

    def test_exact_windows_dependency_edges_are_required(self):
        lock = synthetic_lock()
        m.validate_lock(lock.encode())
        for name in ("forever-casc", "forever-extractor", "forever-navigation", "forever-vmap"):
            with self.subTest(crate=name):
                before, package = lock.split('name = "' + name + '"', 1)
                section, after = package.split("[[package]]", 1)
                removed = section.replace('"forever-platform", ', "", 1).replace('dependencies = ["forever-platform"]\n', "", 1)
                self.assertNotEqual(removed, section)
                changed = before + 'name = "' + name + '"' + removed + "[[package]]" + after
                self.assertFailure("lock_closure", m.validate_lock, changed.encode())

    def test_unreviewed_internal_edge_rejects_with_same_packages(self):
        lock = synthetic_lock()
        before, package = lock.split('name = "forever-navigation"', 1)
        section, after = package.split("[[package]]", 1)
        changed = section.replace('"forever-db2", ', '"forever-casc", "forever-db2", ', 1)
        self.assertNotEqual(changed, section)
        self.assertFailure("lock_closure", m.validate_lock, (before + 'name = "forever-navigation"' + changed + "[[package]]" + after).encode())

    def test_windows_consumer_source_is_shipped_and_mandatory(self):
        self.build()
        for name in m.WINDOWS_INPUT_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
                manifest["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, manifest)

    def test_missing_windows_consumer_prevents_source_build(self):
        name = "crates/forever-casc/src/local_store/io/windows.rs"
        (self.source / name).unlink()
        self.assertFailure("input_io", self.build)

    def test_windows_producer_source_is_shipped_and_mandatory(self):
        self.build()
        for name in m.WINDOWS_PRODUCER_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
                manifest["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, manifest)

    def test_missing_windows_producer_api_sealer_contract_and_doc_prevent_build(self):
        for name in ("crates/forever-platform/src/windows/producer.rs",
                     "crates/forever-extractor/src/producer.rs", "tools/seal_forever_windows.py",
                     "contracts/forever-windows-producer-contract-v1.json", "docs/forever-windows-producer.md",
                     "tests/distribution/test_windows_seal.py", "tests/contracts/test_forever_windows_producer_contract.py"):
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                with patch.object(m, "compile_source", side_effect=AssertionError("missing producer source must reject before compilation")):
                    self.assertFailure("input_io", self.build, "missing-producer")
                path.write_bytes(saved)

    def test_windows_producer_doc_is_shipped_and_mandatory(self):
        self.build()
        name = "docs/forever-windows-producer.md"
        self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        manifest["inventory"].pop("source/" + name)
        self.assertFailure("manifest_missing", m.validate_manifest, manifest)

    def test_only_exact_reviewed_producer_sealer_tool_is_admitted(self):
        self.assertTrue(m.bundle_path_allowed("source/tools/seal_forever_windows.py"))
        for name in ["source/tools/unreviewed_sealer.py", "source/tools/seal_forever_windows.exe",
                     "source/tools/seal_forever_windows.py/child", "source/tests/distribution/unreviewed.py"]:
            self.assertFalse(m.bundle_path_allowed(name))

    def test_v5_vmap_framing_and_routes_source_is_shipped_and_mandatory(self):
        self.build()
        for name in m.WINDOWS_VMAP_SOURCE + m.INPUT_DIAGNOSTICS_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
                manifest["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, manifest)

    def test_missing_v5_source_contract_tests_and_docs_reject_before_compiler(self):
        for name in m.WINDOWS_VMAP_SOURCE + m.INPUT_DIAGNOSTICS_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing v5 source must reject before compilation")):
                        self.assertFailure("source_crate_missing" if name.endswith("/Cargo.toml") else "input_io", self.build, "missing-v5")
                finally:
                    path.write_bytes(saved)

    def test_only_relocated_exact_routes_doc_is_admitted(self):
        self.assertTrue(m.bundle_path_allowed("source/docs/forever-required-routes.md"))
        for name in ("source/docs/unreviewed-routes.md", "source/crates/forever-casc/docs/required-routes-v1.md"):
            self.assertFalse(m.bundle_path_allowed(name))
        path = self.source / "crates/forever-casc/docs/required-routes-v1.md"
        path.parent.mkdir(parents=True)
        path.write_text("unreviewed duplicate doc\n")
        self.assertFailure("source_extension", m.selected_paths, self.source)

    def test_v7_navigation_sources_are_shipped_and_mandatory(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        for name in m.WINDOWS_NAVIGATION_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

    def test_v7_missing_navigation_source_rejects_before_compiler(self):
        for name in m.WINDOWS_NAVIGATION_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing v7 source must reject before compilation")):
                        self.assertFailure("source_crate_missing" if name.endswith("/Cargo.toml") else "input_io", m.build_bundle, self.source, self.root / "missing-v7", self.cache, 1234)
                finally:
                    path.write_bytes(saved)

    def test_v7_navigation_payload_corruption_rejects(self):
        root = self.unpack()
        for name in m.WINDOWS_NAVIGATION_SOURCE:
            with self.subTest(name=name):
                path = root / "source" / name
                saved = path.read_bytes()
                path.write_bytes(saved + b"changed")
                try:
                    self.assertFailure("inventory_hash", m.verify_bundle, root)
                finally:
                    path.write_bytes(saved)

    def test_v7_navigation_doc_allowlist_is_exact(self):
        self.assertTrue(m.bundle_path_allowed("source/docs/forever-windows-navigation-readers.md"))
        for name in ("source/docs/windows-navigation-private.md", "source/tools/navigation_live.py",
                     "source/crates/forever-navigation/docs/windows-readers.md"):
            self.assertFalse(m.bundle_path_allowed(name))

    def test_v6_required_framing_sources_are_shipped_and_mandatory(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        for name in m.REQUIRED_FRAMING_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

    def test_v6_missing_adapter_contract_tests_or_docs_reject_before_compiler(self):
        for name in m.REQUIRED_FRAMING_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing v6 source must reject before compilation")):
                        self.assertFailure("input_io", self.build, "missing-v6")
                finally:
                    path.write_bytes(saved)

    def test_v6_exact_framing_docs_do_not_open_a_generic_allowlist(self):
        for name in ("source/docs/forever-required-framing.md", "source/docs/forever-required-framing-acceptance.md"):
            self.assertTrue(m.bundle_path_allowed(name))
        for name in ("source/docs/required-framing-private.md", "source/tools/required_framing_live.py",
                     "source/crates/forever-casc/docs/required-framing.md"):
            self.assertFalse(m.bundle_path_allowed(name))

    def test_v8_linux_archive_sources_are_shipped_and_mandatory(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        for name in m.LINUX_COLLISION_ARCHIVE_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

    def test_v8_missing_linux_archive_source_rejects_before_compiler(self):
        for name in m.LINUX_COLLISION_ARCHIVE_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing v8 source must reject before compilation")):
                        self.assertFailure("source_crate_missing" if name.endswith("/Cargo.toml") else "input_io", self.build, "missing-v8")
                finally:
                    path.write_bytes(saved)

    def test_v8_linux_archive_payload_corruption_rejects(self):
        root = self.unpack()
        for name in m.LINUX_COLLISION_ARCHIVE_SOURCE:
            with self.subTest(name=name):
                path = root / "source" / name
                saved = path.read_bytes()
                path.write_bytes(saved + b"changed")
                try:
                    self.assertFailure("inventory_hash", m.verify_bundle, root)
                finally:
                    path.write_bytes(saved)

    def test_v8_linux_archive_doc_allowlist_is_exact(self):
        self.assertTrue(m.bundle_path_allowed("source/docs/forever-collision-leased-archive.md"))
        for name in ("source/docs/private-collision-archive.md", "source/tools/collision_live.py", "source/crates/forever-casc/docs/leased-archive.md"):
            self.assertFalse(m.bundle_path_allowed(name))

    def test_v9_product_disk_sources_are_shipped_and_mandatory(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        for name in m.PRODUCT_COLLISION_DISK_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

    def test_v9_missing_product_disk_source_rejects_before_compiler(self):
        for name in m.PRODUCT_COLLISION_DISK_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing v9 source must reject before compilation")):
                        self.assertFailure("source_crate_missing" if name.endswith("/Cargo.toml") else "input_io", self.build, "missing-v9")
                finally:
                    path.write_bytes(saved)

    def test_v9_product_disk_payload_corruption_rejects(self):
        root = self.unpack()
        for name in m.PRODUCT_COLLISION_DISK_SOURCE:
            with self.subTest(name=name):
                path = root / "source" / name
                saved = path.read_bytes()
                path.write_bytes(saved + b"changed")
                try:
                    self.assertFailure("inventory_hash", m.verify_bundle, root)
                finally:
                    path.write_bytes(saved)

    def test_v9_product_disk_doc_allowlist_is_exact(self):
        self.assertTrue(m.bundle_path_allowed("source/docs/forever-product-disk.md"))
        for name in ("source/docs/private-product-disk.md", "source/tools/product_disk_live.py",
                     "source/crates/forever-extractor/docs/product-disk.md"):
            self.assertFalse(m.bundle_path_allowed(name))

    def test_v10_product_catalog_source_is_shipped_and_mandatory(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        for name in m.PRODUCT_DB2_CATALOG_SOURCE:
            with self.subTest(name=name):
                self.assertEqual((self.root / "output/source" / name).read_bytes(), (self.source / name).read_bytes())
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

    def test_v10_missing_catalog_source_rejects_before_compilation(self):
        for name in m.PRODUCT_DB2_CATALOG_SOURCE:
            with self.subTest(name=name):
                path = self.source / name
                saved = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(m, "compile_source", side_effect=AssertionError("missing catalog source must reject before compilation")):
                        self.assertFailure("input_io", self.build, "missing-v10")
                finally:
                    path.write_bytes(saved)

    def test_v10_only_exact_attribution_markdown_is_admitted(self):
        self.assertTrue(m.bundle_path_allowed("source/contracts/WOWDBDEFS-LICENSE.md"))
        for relative in ("contracts/README.md", "contracts/subdir/WOWDBDEFS-LICENSE.md", "schemas/WOWDBDEFS-LICENSE.md"):
            with self.subTest(relative=relative):
                self.assertFalse(m.bundle_path_allowed("source/" + relative))
                path = self.source / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"unapproved markdown")
                try:
                    self.assertFailure("source_extension", m.source_snapshot, self.source)
                finally:
                    path.unlink()

    def test_v10_catalog_source_payload_changes_reject(self):
        root = self.unpack()
        for name in m.PRODUCT_DB2_CATALOG_SOURCE:
            with self.subTest(name=name):
                path = root / "source" / name
                saved = path.read_bytes()
                path.write_bytes(saved + b"changed")
                try:
                    self.assertFailure("inventory_hash", m.verify_bundle, root)
                finally:
                    path.write_bytes(saved)

    def test_v10_contract_and_attribution_match_both_catalog_pins(self):
        contract = json.loads((REPO / "contracts/forever-distribution-contract-v1.json").read_bytes())
        self.assertEqual(contract["source"]["profile"], m.SOURCE_PROFILE)
        self.assertEqual(contract["source"]["contract_markdown_exception"], m.CONTRACT_MARKDOWN)
        self.assertTrue(set(m.PRODUCT_DB2_CATALOG_SOURCE) <= set(contract["source"]["required"]))
        self.assertIn("forever-seven-crate-linux-product-collision-disk-source-v9", contract["source"]["prior_profiles"])
        self.assertFalse(contract["source"]["product_db2_catalog_acceptance"]["bundle_built"])
        attribution = (REPO / m.CONTRACT_MARKDOWN).read_text()
        for build in (70205, 70338):
            catalog = json.loads((REPO / f"contracts/wowdbdefs-{build}-v1.json").read_bytes())
            self.assertIn(catalog["source"]["commit"], attribution)
            self.assertIn(str(build), attribution)
            self.assertEqual(catalog["source"]["license"], "CC-BY-SA-4.0")

    def test_v10_does_not_admit_historical_v9_profile(self):
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        manifest["source_snapshot"]["profile"] = "forever-seven-crate-linux-product-collision-disk-source-v9"
        self.assertFailure("manifest_snapshot", m.validate_manifest, manifest)

    def test_v12_requires_and_ships_exact_v23_source_closure(self):
        self.assertEqual(
            m.SOURCE_PROFILE,
            "forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-source-v12",
        )
        self.assertTrue(set(m.PRODUCT_V23_SOURCE) <= set(m.REQUIRED))
        self.build()
        manifest = json.loads((self.root / "output/distribution-manifest.json").read_bytes())
        self.assertEqual(manifest["source_snapshot"]["profile"], m.SOURCE_PROFILE)
        for name in m.PRODUCT_V23_SOURCE:
            with self.subTest(name=name):
                self.assertTrue(m.bundle_path_allowed("source/" + name))
                self.assertIn(name, manifest["source_snapshot"]["files"])
                self.assertEqual(
                    (self.root / "output/source" / name).read_bytes(),
                    (self.source / name).read_bytes(),
                )

        # The verifier requires the named v23 closure, rather than trusting the
        # broad directory scans used to populate a normal current snapshot.
        for name in (
            "crates/forever-extractor/src/driver/navigation_v23.rs",
            "contracts/forever-product-contract-v23.json",
            "tests/contracts/fixtures/forever-product-v23/map48.tsv",
            "docs/forever-product-v23.md",
            "docs/acceptance/forever-v23-selected-navigation-70450-20261020.json",
            "docs/acceptance/forever-v23-selected-navigation-ground-70450-20261020.json",
            "docs/forever-navigation.md",
        ):
            with self.subTest(missing_manifest=name):
                incomplete = copy.deepcopy(manifest)
                incomplete["inventory"].pop("source/" + name)
                self.assertFailure("manifest_missing", m.validate_manifest, incomplete)

        contract = json.loads((REPO / "contracts/forever-distribution-contract-v1.json").read_bytes())
        self.assertEqual(contract["source"]["profile"], m.SOURCE_PROFILE)
        self.assertTrue(set(m.PRODUCT_V23_SOURCE) <= set(contract["source"]["required"]))
        self.assertFalse(contract["source"]["product_v23_selected_navigation_acceptance"]["bundle_built"])

    def test_missing_v23_source_contract_fixture_and_link_docs_reject_before_compile(self):
        missing = (
            "crates/forever-extractor/src/driver/navigation_v23.rs",
            "contracts/forever-product-contract-v23.json",
            "tests/contracts/fixtures/forever-product-v23/map48.tsv",
            "docs/forever-product-v23.md",
            "docs/acceptance/forever-v23-selected-navigation-70450-20261020.json",
            "docs/acceptance/forever-v23-selected-navigation-ground-70450-20261020.json",
            "docs/forever-navigation.md",
        )
        for index, name in enumerate(missing):
            with self.subTest(name=name):
                path = self.source / name
                original = path.read_bytes()
                path.unlink()
                try:
                    with patch.object(
                        m,
                        "compile_source",
                        side_effect=AssertionError("missing v23 closure must reject before compilation"),
                    ):
                        self.assertFailure("input_io", self.build, "missing-v23-" + str(index))
                finally:
                    path.write_bytes(original)

    def test_environment_drops_inherited_injection(self):
        with patch.dict(
            m.os.environ,
            {
                "RUSTFLAGS": "secret",
                "RUSTC_WRAPPER": "evil",
                "CARGO_BUILD_JOBS": "64",
                "RCE_FOREVER_CASC_REFERENCE": "client",
                "HTTP_PROXY": "secret",
            },
        ):
            env = m.build_env(self.root, {"rustc": "/installed/rustc", "cargo": "/installed/cargo"}, 1234)
        self.assertNotIn("RUSTC_WRAPPER", env)
        self.assertNotIn("RCE_FOREVER_CASC_REFERENCE", env)
        self.assertNotIn("HTTP_PROXY", env)
        self.assertEqual(env["CARGO_BUILD_JOBS"], "1")
        self.assertNotIn("secret", " ".join(env.values()))
        self.assertEqual(env["CARGO_NET_OFFLINE"], "true")
        self.assertEqual(m.build_description()["cargo_jobs"], 1)


if __name__ == "__main__":
    unittest.main()
