#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Create and inspect local, offline Linux source/binary distribution candidates.

Only authored source and the two pinned registry archives are admitted. This is
an integrity inventory, not a signature or a claim of full extractor acceptance.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tomllib
import zlib

SCHEMA = "forever-distribution-v1"
SOURCE_PROFILE = "forever-seven-crate-windows-navigation-readers-source-v7"
CRATES = ("forever-casc", "forever-db2", "forever-extractor", "forever-navigation", "forever-platform", "forever-terrain", "forever-vmap")
DEPENDENCIES = {
    "adler2-2.0.1": "320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa",
    "miniz_oxide-0.8.9": "1fa76a2c86f704bdb222d66965fb3d63269ce38518b83cb0575fca855ebb6316",
}
# Original notice documents shipped with the installed Rust1.98.0 Linux x86_64
# toolchain. Retaining the broad license directory does not assert that every
# component listed in its notices is linked into this product.
TOOLCHAIN_NOTICES = {
    "COPYRIGHT-library.html": "68129500b616d5838629e68f55ff3aed5e096dacf60ce9eb41bbe599a563afa6",
    "licenses/Apache-2.0.txt": "074e6e32c86a4c0ef8b3ed25b721ca23aca83df277cd88106ef7177c354615ff",
    "licenses/BSD-2-Clause.txt": "f32fb3b417a194167cfad068223fc975ba96c5960513a10f66a3c28720aec1df",
    "licenses/CC-BY-SA-4.0.txt": "23ee78c8bae49cf08ea2f0c84945c66b987ebe4520881fb51b3dad4fb43d07c2",
    "licenses/GCC-exception-3.1.txt": "a3966c9410ef94a1efbfa9ec04e9c1994026a70623fbf0885ce36737bfba3b54",
    "licenses/GPL-2.0-only.txt": "54b212ba1f97f6948fa5486239ab505dd820b02e50cbfd296b4062c8609a70af",
    "licenses/GPL-3.0-or-later.txt": "05315fce6c35f79b8a35454828995fa26336f235895ac64557abc028f4ffcea2",
    "licenses/ISC.txt": "0ff61769b238634e042ab759dddfd104a8e69d091e3600ae3410e401a6164381",
    "licenses/LLVM-exception.txt": "e34c58338bd89d43e709e226610d8f32b3e3c47f4ad9a99a8dc1d4ac7842488e",
    "licenses/MIT.txt": "b85dcd3e453d05982552c52b5fc9e0bdd6d23c6f8e844b984a88af32570b0cc0",
    "licenses/NCSA.txt": "39cb0234065c92b9e6cee3830cbb52b1aa289aa8fe9ba2e66730fec5cf05c630",
    "licenses/OFL-1.1.txt": "8eea8287e5876b539670cadb82e99f9a7afddec6f6730811be1daf25d2e9bcfd",
    "licenses/Unicode-3.0.txt": "f5062c9a188d81dfe66b56db4182dcf9e4b17c0d9b0d311a8e20b3a1b075c443",
}
TEST_SUFFIXES = {".py", ".rs", ".cpp", ".h", ".hpp", ".tsv", ".json", ".md"}
DOCS = ("README.md", "docs/forever-product.md", "docs/forever-completion.md", "docs/forever-distribution.md", "docs/forever-ci.md", "docs/forever-windows-producer.md")
WINDOWS_INPUT_SOURCE = ("crates/forever-platform/README.md", "crates/forever-platform/src/path.rs", "crates/forever-platform/src/windows.rs", "crates/forever-platform/src/windows/ffi.rs", "crates/forever-platform/src/windows/tests.rs", "crates/forever-casc/src/admission.rs", "crates/forever-casc/src/local_store.rs", "crates/forever-casc/src/local_store/io.rs", "crates/forever-casc/src/local_store/io/windows.rs", "crates/forever-casc/src/local_store/io/windows/tests.rs", "crates/forever-extractor/src/input.rs", "crates/forever-extractor/src/input/windows.rs", "crates/forever-extractor/src/input/windows_tests.rs", "contracts/forever-windows-consumers-contract-v1.json", "tests/contracts/test_forever_windows_consumers_contract.py")
WINDOWS_PRODUCER_SOURCE = ("crates/forever-platform/src/windows/producer.rs", "crates/forever-platform/src/windows/producer/ffi.rs", "crates/forever-platform/src/windows/producer/tests.rs", "crates/forever-extractor/src/producer.rs", "crates/forever-extractor/src/producer/tests.rs", "crates/forever-extractor/examples/windows_producer_identity.rs", "tools/seal_forever_windows.py", "tests/distribution/test_windows_seal.py", "contracts/forever-windows-producer-contract-v1.json", "tests/contracts/test_forever_windows_producer_contract.py")
WINDOWS_VMAP_SOURCE = ('crates/forever-vmap/Cargo.toml', 'crates/forever-vmap/src/store.rs', 'crates/forever-vmap/src/store/windows.rs', 'crates/forever-vmap/src/store/windows/tests.rs', 'contracts/forever-windows-vmap-contract-v1.json', 'tests/contracts/test_forever_windows_vmap_contract.py', 'docs/forever-windows-vmap.md')
INPUT_DIAGNOSTICS_SOURCE = ('crates/forever-casc/src/blte.rs', 'crates/forever-casc/src/blte/framing.rs', 'crates/forever-casc/src/blte/framing/tests.rs', 'crates/forever-casc/src/local_store.rs', 'crates/forever-casc/src/local_store/io.rs', 'crates/forever-casc/src/local_store/io/route.rs', 'crates/forever-casc/src/local_store/owned.rs', 'crates/forever-casc/src/local_store/routes.rs', 'crates/forever-casc/src/local_store/route_tests.rs', 'crates/forever-casc/src/local_store/tests.rs', 'contracts/forever-input-diagnostics-contract-v1.json', 'tests/contracts/test_forever_input_diagnostics_contract.py', 'docs/forever-blte-framing.md', 'docs/forever-required-routes.md')
REQUIRED_FRAMING_SOURCE = ('crates/forever-casc/src/local_store.rs', 'crates/forever-casc/src/local_store/io.rs', 'crates/forever-casc/src/local_store/tests.rs', 'crates/forever-casc/src/local_store/framing.rs', 'crates/forever-casc/src/local_store/framing_tests.rs', 'crates/forever-casc/src/local_store/io/framing.rs', 'docs/forever-required-framing.md', 'contracts/forever-required-framing-contract-v1.json', 'tests/contracts/test_forever_required_framing_contract.py', 'docs/forever-required-framing-acceptance.md')
WINDOWS_NAVIGATION_SOURCE = ('crates/forever-navigation/Cargo.toml', 'crates/forever-navigation/src/inventory.rs', 'crates/forever-navigation/src/inventory/windows.rs', 'crates/forever-navigation/src/inventory/windows/guard.rs', 'crates/forever-navigation/src/inventory/windows/run.rs', 'crates/forever-navigation/src/inventory/windows/run/tests.rs', 'crates/forever-navigation/src/inventory/windows/tests.rs', 'crates/forever-navigation/src/terrain/load.rs', 'crates/forever-navigation/src/terrain/load/windows.rs', 'crates/forever-navigation/src/terrain/load/windows/tests.rs', 'docs/forever-windows-navigation-readers.md', 'contracts/forever-windows-navigation-contract-v1.json', 'tests/contracts/test_forever_windows_navigation_contract.py')
REQUIRED = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "AUTHORS", "tools/package_forever.py", "tests/distribution/test_package_forever.py", "docs/licensing/forever-rust-casc.md", "licensing/import-manifest.json", "licensing/inventory.json", "licensing/source-files.json", ".github/workflows/forever-rust.yml", "tests/contracts/test_forever_ci_contract.py") + DOCS + WINDOWS_INPUT_SOURCE + WINDOWS_PRODUCER_SOURCE + WINDOWS_VMAP_SOURCE + INPUT_DIAGNOSTICS_SOURCE + REQUIRED_FRAMING_SOURCE + WINDOWS_NAVIGATION_SOURCE
MAX_FILES = 8192
MAX_FILE = 32 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024
MAX_MANIFEST = 4 * 1024 * 1024
VENDOR_CONFIG = b'[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "vendor"\n[net]\noffline = true\n'
MANIFEST_KEYS = {"schema", "platform", "target", "epoch", "toolchain", "build", "source_snapshot", "dependencies", "toolchain_notices", "inventory", "full_extractor_complete", "publication"}


class Failure(Exception):
    pass


def require(condition, code):
    if not condition:
        raise Failure(code)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode()


def safe_name(name):
    require(isinstance(name, str) and name and "\\" not in name and "\x00" not in name, "unsafe_path")
    require(not name.startswith("/") and all(p not in ("", ".", "..") for p in name.split("/")), "unsafe_path")
    require(all(ord(c) >= 32 and ord(c) < 127 for c in name), "unsafe_path")
    return PurePosixPath(name)


def regular_bytes(path, limit=None):
    """No-follow, bounded read with descriptor/name identities checked twice."""
    if limit is None:
        limit = MAX_FILE
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
        with os.fdopen(fd, "rb") as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode), "input_type")
            require(before.st_size <= limit, "input_budget")
            data = stream.read(limit + 1)
            after = os.fstat(stream.fileno())
        named = os.stat(path, follow_symlinks=False)
    except OSError as exc:
        raise Failure("input_io") from exc
    identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_mode)
    require(len(data) == before.st_size and identity(before) == identity(after) == identity(named), "input_changed")
    return data


def tree_files(root, skip_pycache=True, allowed_directories=None):
    require(root.is_dir() and not root.is_symlink(), "input_directory")
    for base, dirs, files in os.walk(root, followlinks=False):
        dirs.sort()
        files.sort()
        for name in dirs[:]:
            p = Path(base) / name
            require(not p.is_symlink(), "input_symlink")
            if skip_pycache and name == "__pycache__":
                dirs.remove(name)
            elif allowed_directories is not None:
                require(p.relative_to(root).as_posix() in allowed_directories, "bundle_topology")
        for name in files:
            p = Path(base) / name
            require(stat.S_ISREG(p.lstat().st_mode), "input_type")
            yield p


def selected_paths(source):
    result = set(REQUIRED)
    for name in DOCS:
        if (source / name).exists():
            result.add(name)
    for crate in CRATES:
        root = source / "crates" / crate
        require((root / "Cargo.toml").is_file(), "source_crate_missing")
        for p in tree_files(root):
            relative = p.relative_to(source).as_posix()
            require(p.suffix == ".rs" or p.name in ("Cargo.toml", "README.md"), "source_extension")
            result.add(relative)
    for directory, suffixes in (("contracts", {".json"}), ("schemas", {".json"}), ("tests/forever", TEST_SUFFIXES), ("tests/contracts", TEST_SUFFIXES), ("LICENSES", {".txt"}), ("docs/licensing", {".md"})):
        for p in tree_files(source / directory):
            require(p.suffix in suffixes, "source_extension")
            result.add(p.relative_to(source).as_posix())
    return sorted(result)


def source_snapshot(source):
    files = {}
    total = 0
    for name in selected_paths(source):
        safe_name(name)
        # Reject symlink ancestors too: resolve must preserve the lexical path.
        path = source / name
        require(path.resolve() == path, "input_symlink")
        data = regular_bytes(path)
        total += len(data)
        require(total <= MAX_TOTAL and len(files) < MAX_FILES, "source_budget")
        files[name] = {"bytes": len(data), "sha256": digest(data), "mode": 0o644}
    return files


def validate_lock(data):
    try:
        lock = tomllib.loads(data.decode("utf-8"))
        require(set(lock) == {"version", "package"} and lock["version"] == 4, "lock_shape")
        packages = lock["package"]
        require(len(packages) == 9, "lock_closure")
        names = set()
        registry = {}
        expected = {"forever-casc": ["forever-db2", "forever-platform", "forever-terrain", "forever-vmap", "miniz_oxide"], "forever-db2": [], "forever-extractor": ["forever-casc", "forever-db2", "forever-navigation", "forever-platform", "forever-terrain", "forever-vmap"], "forever-navigation": ["forever-db2", "forever-platform", "forever-terrain", "forever-vmap"], "forever-platform": [], "forever-terrain": [], "forever-vmap": ["forever-platform"], "adler2": [], "miniz_oxide": ["adler2"]}
        for package in packages:
            name = package["name"]
            require(name not in names and name in expected, "lock_closure")
            names.add(name)
            require(package.get("dependencies", []) == expected[name], "lock_closure")
            if name in CRATES:
                require(set(package) <= {"name", "version", "dependencies"} and package["version"] == "0.1.0", "lock_closure")
            else:
                require(set(package) <= {"name", "version", "dependencies", "source", "checksum"}, "lock_closure")
                key = name + "-" + package["version"]
                require(package["source"] == "registry+https://github.com/rust-lang/crates.io-index" and DEPENDENCIES.get(key) == package["checksum"], "lock_checksum")
                registry[key] = package["checksum"]
        require(registry == DEPENDENCIES and names == set(expected), "lock_closure")
    except (ValueError, UnicodeError, KeyError, TypeError) as exc:
        raise Failure("lock_shape") from exc


def safe_crate_entries(data, key):
    """Return original crate files; never ask tarfile to extract paths."""
    entries = {}
    seen = set()
    total = 0
    try:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            for member in archive:
                name = member.name.rstrip("/") if member.isdir() else member.name
                safe_name(name)
                require(name not in seen, "archive_duplicate")
                seen.add(name)
                require(name == key or name.startswith(key + "/"), "crate_root")
                require(member.isfile() or member.isdir(), "archive_type")
                require(len(seen) <= MAX_FILES and 0 <= member.size <= MAX_FILE, "archive_budget")
                if member.isdir():
                    require(member.size == 0, "archive_type")
                    continue
                require(name != key, "crate_root")
                total += member.size
                require(total <= MAX_TOTAL, "archive_budget")
                payload = archive.extractfile(member).read(MAX_FILE + 1)
                require(len(payload) == member.size, "archive_truncated")
                relative = name[len(key) + 1:]
                entries[relative] = payload
    except (tarfile.TarError, OSError, EOFError) as exc:
        raise Failure("crate_archive") from exc
    require("Cargo.toml" in entries and any("license" in n.lower() for n in entries), "crate_notice_missing")
    return entries


def find_crate(cache, key):
    # Accept a flat explicit archive cache or the standard registry/cache layout.
    candidates = [cache / (key + ".crate")]
    registry = cache / "registry" / "cache"
    if registry.is_dir() and not registry.is_symlink():
        candidates += sorted(registry.glob("*/" + key + ".crate"))
    found = []
    for path in candidates:
        if path.exists() or path.is_symlink():
            require(path.resolve() == path.absolute(), "input_symlink")
            data = regular_bytes(path)
            require(digest(data) == DEPENDENCIES[key], "dependency_checksum")
            found.append(data)
    require(found and all(data == found[0] for data in found), "dependency_missing")
    return found[0]


def write_new(path, data, mode=0o644):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(data)
    path.chmod(mode)


def vendor(source, cache):
    for key in sorted(DEPENDENCIES):
        data = find_crate(cache, key)
        write_new(source / "dependency-archives" / (key + ".crate"), data)
        entries = safe_crate_entries(data, key)
        for name, payload in entries.items():
            write_new(source / "vendor" / key / name, payload)
        checksum = {"package": DEPENDENCIES[key], "files": {n: digest(b) for n, b in sorted(entries.items())}}
        write_new(source / "vendor" / key / ".cargo-checksum.json", canonical(checksum))
    write_new(source / ".cargo" / "config.toml", VENDOR_CONFIG)


def new_output(output, source=None):
    output = output.absolute()
    require(all(ord(char) >= 32 and ord(char) != 127 for char in str(output)), "unsafe_output_path")
    require(not output.exists() and not output.is_symlink(), "output_exists")
    parent = output.parent.resolve()
    require(parent == output.parent, "output_symlink")
    if source is not None:
        require(not output.is_relative_to(source), "output_inside_source")
    for ancestor in (parent, *parent.parents):
        require(not (ancestor / ".git").exists(), "output_inside_git")
    output.mkdir(mode=0o700)
    return output


def installed_toolchain():
    require(sys.platform.startswith("linux"), "platform_unsupported")
    rustup = shutil.which("rustup")
    require(rustup is not None, "toolchain_missing")
    # `rustup which` only locates installed programs; never installs a toolchain.
    programs = {}
    for name in ("rustc", "cargo"):
        process = subprocess.run([rustup, "which", "--toolchain", "1.98.0", name], capture_output=True, text=True, check=False)
        require(process.returncode == 0, "toolchain_missing")
        path = Path(process.stdout.strip())
        require(path.is_file(), "toolchain_missing")
        programs[name] = str(path)
    return programs


def notice_documents(root):
    """Read exact original notice bytes; admit no new toolchain source code."""
    require(root.resolve() == root, "input_symlink")
    licenses = root / "licenses"
    require(licenses.is_dir() and not licenses.is_symlink(), "toolchain_notice_missing")
    paths = [root / "COPYRIGHT-library.html", *sorted(licenses.glob("*.txt"))]
    require(all(path.exists() for path in paths), "toolchain_notice_missing")
    require(all(stat.S_ISREG(path.lstat().st_mode) for path in paths), "toolchain_notice_type")
    names = {path.relative_to(root).as_posix() for path in paths}
    require(names == set(TOOLCHAIN_NOTICES), "toolchain_notice_closure")
    result = {}
    for path in paths:
        name = path.relative_to(root).as_posix()
        data = regular_bytes(path)
        require(digest(data) == TOOLCHAIN_NOTICES[name], "toolchain_notice_checksum")
        result[name] = data
    return result


def read_toolchain_notices():
    programs = installed_toolchain()
    env = {"PATH": str(Path(programs["rustc"]).parent) + ":/usr/bin:/bin", "LC_ALL": "C"}
    process = subprocess.run([programs["rustc"], "--print", "sysroot"], env=env, capture_output=True, text=True, check=False)
    require(process.returncode == 0, "toolchain_notice_missing")
    sysroot = Path(process.stdout.strip())
    require(sysroot.is_absolute() and sysroot.resolve() == sysroot and sysroot == Path(programs["rustc"]).parent.parent, "toolchain_sysroot")
    return notice_documents(sysroot / "share" / "doc" / "rust")


def build_env(output, programs, epoch):
    env = {"PATH": str(Path(programs["rustc"]).parent) + ":/usr/bin:/bin", "HOME": str(output / "home"), "CARGO_HOME": str(output / "cargo-home"), "CARGO_TARGET_DIR": str(output / "target"), "RUSTC": programs["rustc"], "LANG": "C", "LC_ALL": "C", "TZ": "UTC", "SOURCE_DATE_EPOCH": str(epoch), "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_RELEASE_DEBUG": "0", "CARGO_PROFILE_RELEASE_STRIP": "debuginfo", "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "1", "CARGO_NET_OFFLINE": "true"}
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(["--remap-path-prefix=" + str(output) + "=/forever-distribution", "-C", "debuginfo=0", "-C", "strip=debuginfo", "-C", "codegen-units=1"])
    return env


def compile_source(output, epoch):
    # Cargo searches ancestor configuration even with an isolated CARGO_HOME.
    # Refuse that ambient authority rather than silently inherit build settings.
    for ancestor in (output, *output.parents):
        require(not any((ancestor / ".cargo" / name).exists() or (ancestor / ".cargo" / name).is_symlink() for name in ("config", "config.toml")), "ambient_cargo_config")
    programs = installed_toolchain()
    env = build_env(output, programs, epoch)
    for directory in ("home", "cargo-home", "target"):
        (output / directory).mkdir()
    rustc = subprocess.run([programs["rustc"], "-vV"], env=env, capture_output=True, text=True, check=False)
    cargo = subprocess.run([programs["cargo"], "-V"], env=env, capture_output=True, text=True, check=False)
    require(rustc.returncode == cargo.returncode == 0 and rustc.stdout.startswith("rustc 1.98.0 ") and cargo.stdout.startswith("cargo 1.98.0 "), "toolchain_version")
    host = next((line[6:] for line in rustc.stdout.splitlines() if line.startswith("host: ")), "")
    require(host == "x86_64-unknown-linux-gnu", "target_unsupported")
    command = [programs["cargo"], "build", "--offline", "--locked", "--release", "-p", "forever-extractor", "--no-default-features"]
    with (output / "build.log").open("xb") as log:
        process = subprocess.run(command, cwd=output / "source", env=env, stdout=log, stderr=subprocess.STDOUT, check=False)
    require(process.returncode == 0, "build_failed")
    binary = regular_bytes(output / "target" / "release" / "forever-extractor", 64 * 1024 * 1024)
    require(binary.startswith(b"\x7fELF"), "binary_type")
    write_new(output / "bin" / "forever-extractor", binary, 0o755)
    return {"rustc": rustc.stdout.strip(), "cargo": cargo.stdout.strip()}, host


def bundle_path_allowed(name):
    safe_name(name)
    if name in ("BUILD.md", "toolchain.json", "bin/forever-extractor"):
        return True
    if name.startswith("toolchain-notices/"):
        return name[len("toolchain-notices/"):] in TOOLCHAIN_NOTICES
    if not name.startswith("source/"):
        return False
    relative = name[7:]
    if relative in REQUIRED or relative in DOCS or relative == ".cargo/config.toml":
        return True
    pieces = relative.split("/")
    if pieces[0] == "dependency-archives":
        return len(pieces) == 2 and pieces[1] in {k + ".crate" for k in DEPENDENCIES}
    if pieces[0] == "vendor":
        return len(pieces) >= 3 and pieces[1] in DEPENDENCIES
    if pieces[0] == "crates":
        return len(pieces) >= 3 and pieces[1] in CRATES and (relative.endswith(".rs") or pieces[-1] in ("Cargo.toml", "README.md"))
    if pieces[0] in ("contracts", "schemas"):
        return len(pieces) >= 2 and relative.endswith(".json")
    if pieces[0] == "LICENSES":
        return len(pieces) == 2 and relative.endswith(".txt")
    if relative.startswith("docs/licensing/"):
        return relative.endswith(".md")
    if relative.startswith(("tests/forever/", "tests/contracts/")):
        return PurePosixPath(relative).suffix in TEST_SUFFIXES
    return False


def inventory_for(output):
    result = {}
    for root in (output / "source", output / "bin", output / "toolchain-notices"):
        for path in tree_files(root):
            name = path.relative_to(output).as_posix()
            require(bundle_path_allowed(name), "bundle_topology")
            data = regular_bytes(path, 64 * 1024 * 1024)
            result[name] = {"bytes": len(data), "sha256": digest(data), "mode": 0o755 if name == "bin/forever-extractor" else 0o644}
    for name in ("BUILD.md", "toolchain.json"):
        data = regular_bytes(output / name)
        result[name] = {"bytes": len(data), "sha256": digest(data), "mode": 0o644}
    return dict(sorted(result.items()))


def file_inventory(root):
    return {p.relative_to(root).as_posix(): digest(regular_bytes(p)) for p in tree_files(root, skip_pycache=False)}


def read_json(data):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "json_duplicate")
            result[key] = value
        return result
    try:
        return json.loads(data, object_pairs_hook=pairs)
    except (ValueError, UnicodeError) as exc:
        raise Failure("manifest_json") from exc


def validate_manifest(manifest):
    require(isinstance(manifest, dict) and set(manifest) == MANIFEST_KEYS, "manifest_shape")
    require(manifest["schema"] == SCHEMA and manifest["platform"] == "linux" and manifest["target"] == "x86_64-unknown-linux-gnu" and manifest["publication"] == "local-candidate" and manifest["full_extractor_complete"] is False, "manifest_profile")
    require(type(manifest["epoch"]) is int and 0 <= manifest["epoch"] <= 0xffffffff, "epoch")
    require(manifest["dependencies"] == DEPENDENCIES, "manifest_dependencies")
    require(manifest["toolchain_notices"] == TOOLCHAIN_NOTICES, "manifest_toolchain_notices")
    require(isinstance(manifest["inventory"], dict) and 0 < len(manifest["inventory"]) < MAX_FILES, "manifest_inventory")
    names = set(manifest["inventory"])
    for name in names:
        safe_name(name)
        require(not any(str(parent) in names for parent in PurePosixPath(name).parents), "inventory_path_collision")
    total = 0
    for name, value in manifest["inventory"].items():
        require(bundle_path_allowed(name), "bundle_topology")
        require(isinstance(value, dict) and set(value) == {"bytes", "sha256", "mode"}, "manifest_inventory")
        require(type(value["bytes"]) is int and 0 <= value["bytes"] <= 64 * 1024 * 1024 and isinstance(value["sha256"], str) and re.fullmatch("[0-9a-f]{64}", value["sha256"]) and value["mode"] == (0o755 if name == "bin/forever-extractor" else 0o644), "manifest_inventory")
        total += value["bytes"]
    require(total <= MAX_TOTAL, "manifest_budget")
    mandatory = {"source/" + n for n in REQUIRED} | {"bin/forever-extractor", "BUILD.md", "toolchain.json", "source/.cargo/config.toml"}
    mandatory |= {"source/dependency-archives/" + k + ".crate" for k in DEPENDENCIES}
    mandatory |= {"source/crates/" + k + "/" + name for k in CRATES for name in ("Cargo.toml", "src/lib.rs")}
    mandatory.add("source/crates/forever-extractor/src/main.rs")
    mandatory.add("source/crates/forever-casc/build.rs")
    mandatory |= {"toolchain-notices/" + name for name in TOOLCHAIN_NOTICES}
    require(mandatory <= manifest["inventory"].keys(), "manifest_missing")
    for name, expected in TOOLCHAIN_NOTICES.items():
        require(manifest["inventory"]["toolchain-notices/" + name]["sha256"] == expected, "manifest_toolchain_notices")
    snapshot = manifest["source_snapshot"]
    require(isinstance(snapshot, dict) and set(snapshot) == {"kind", "profile", "files", "sha256"} and snapshot["kind"] == "current-working-tree-selected-source" and snapshot["profile"] == SOURCE_PROFILE, "manifest_snapshot")
    require(isinstance(snapshot["files"], dict) and digest(canonical(snapshot["files"])) == snapshot["sha256"], "manifest_snapshot")
    original = {n[7:]: e for n, e in manifest["inventory"].items() if n.startswith("source/") and not n.startswith(("source/vendor/", "source/dependency-archives/")) and n != "source/.cargo/config.toml"}
    require(snapshot["files"] == original, "manifest_snapshot")
    for name, entry in snapshot["files"].items():
        require(manifest["inventory"].get("source/" + name) == entry, "manifest_snapshot")
    require(isinstance(manifest["toolchain"], dict) and set(manifest["toolchain"]) == {"rustc", "cargo"}, "manifest_toolchain")
    require(all(isinstance(value, str) for value in manifest["toolchain"].values()), "manifest_toolchain")
    require(manifest["toolchain"]["rustc"].startswith("rustc 1.98.0 ") and manifest["toolchain"]["cargo"].startswith("cargo 1.98.0 "), "manifest_toolchain")
    require(manifest["build"] == build_description(), "manifest_build")


def build_description():
    return {"command": ["cargo", "build", "--offline", "--locked", "--release", "-p", "forever-extractor", "--no-default-features"], "native": False, "incremental": False, "debug": 0, "strip": "debuginfo", "codegen_units": 1, "remap_root": "/forever-distribution", "system_linkage": "x86_64 GNU/Linux system linker and runtime; libgcc_s.so.1, libm.so.6, libc.so.6, ld-linux-x86-64.so.2 are external; not static or all-Linux ABI compatibility"}


def validate_payloads(manifest, payloads):
    validate_manifest(manifest)
    require(set(payloads) == set(manifest["inventory"]), "inventory_mismatch")
    for name, entry in manifest["inventory"].items():
        data = payloads[name]
        require(len(data) == entry["bytes"] and digest(data) == entry["sha256"], "inventory_hash")
    validate_lock(payloads["source/Cargo.lock"])
    require(payloads["source/.cargo/config.toml"] == VENDOR_CONFIG, "vendor_config")
    require(tomllib.loads(payloads["source/rust-toolchain.toml"].decode())["toolchain"]["channel"] == "1.98.0", "toolchain_version")
    require(payloads["toolchain.json"] == canonical(manifest["toolchain"]), "toolchain_mismatch")
    for key, checksum in DEPENDENCIES.items():
        data = payloads["source/dependency-archives/" + key + ".crate"]
        require(digest(data) == checksum, "dependency_checksum")
        entries = safe_crate_entries(data, key)
        expected = {"source/vendor/" + key + "/" + n: b for n, b in entries.items()}
        expected["source/vendor/" + key + "/.cargo-checksum.json"] = canonical({"package": checksum, "files": {n: digest(b) for n, b in sorted(entries.items())}})
        actual = {n: b for n, b in payloads.items() if n.startswith("source/vendor/" + key + "/")}
        require(actual == expected, "vendor_mismatch")
    for name, expected in TOOLCHAIN_NOTICES.items():
        require(digest(payloads["toolchain-notices/" + name]) == expected, "toolchain_notice_checksum")
    require(payloads["bin/forever-extractor"].startswith(b"\x7fELF"), "binary_type")


def verify_archive(path):
    payloads = {}
    manifest = None
    total = 0
    try:
        compressed = regular_bytes(path, MAX_TOTAL)
        require(len(compressed) >= 18 and compressed[:4] == b"\x1f\x8b\x08\x00", "gzip_metadata")
        maximum = MAX_TOTAL + (MAX_FILES + 32) * 1024
        decoder = zlib.decompressobj(wbits=31)
        decoded = decoder.decompress(compressed, maximum + 1)
        require(len(decoded) <= maximum and decoder.eof, "archive_budget")
        require(not decoder.unused_data and not decoder.unconsumed_tail, "gzip_trailing")
        with tarfile.open(fileobj=io.BytesIO(decoded), mode="r:") as archive:
            for member in archive:
                safe_name(member.name)
                require(member.isfile() and not member.pax_headers, "archive_type")
                require(member.name not in payloads, "archive_duplicate")
                require(member.name == "distribution-manifest.json" or bundle_path_allowed(member.name), "bundle_topology")
                require(member.name != "distribution-manifest.json" or member.size <= MAX_MANIFEST, "manifest_budget")
                total += member.size
                require(0 <= member.size <= 64 * 1024 * 1024 and total <= MAX_TOTAL and len(payloads) < MAX_FILES, "archive_budget")
                require(member.uid == member.gid == 0 and member.uname == member.gname == "", "archive_metadata")
                data = archive.extractfile(member).read(64 * 1024 * 1024 + 1)
                require(len(data) == member.size, "archive_truncated")
                payloads[member.name] = data
            require("distribution-manifest.json" in payloads, "manifest_missing")
            manifest_data = payloads.pop("distribution-manifest.json")
            manifest = read_json(manifest_data)
            require(manifest_data == canonical(manifest), "manifest_encoding")
            validate_payloads(manifest, payloads)
            members = archive.getmembers()
            require([m.name for m in members] == sorted(m.name for m in members), "archive_order")
            end = members[-1].offset_data + ((members[-1].size + 511) // 512) * 512
            expected_length = ((end + 1024 + 10239) // 10240) * 10240
            require(len(decoded) == expected_length and not any(decoded[end:]), "archive_trailing")
            require(int.from_bytes(compressed[4:8], "little") == manifest["epoch"] and compressed[8:10] == b"\x02\xff", "gzip_metadata")
            offset = 0
            for member in members:
                mode = 0o644 if member.name == "distribution-manifest.json" else manifest["inventory"][member.name]["mode"]
                require(member.mtime == manifest["epoch"] and member.mode == mode, "archive_metadata")
                # Reconstruct the entire canonical USTAR header, including fields
                # tarfile otherwise ignores or folds away (GNU/PAX extensions).
                expected = tarfile.TarInfo(member.name)
                expected.size = member.size
                expected.mtime = manifest["epoch"]
                expected.mode = mode
                expected.uid = expected.gid = 0
                expected.uname = expected.gname = ""
                header = expected.tobuf(format=tarfile.USTAR_FORMAT)
                require(member.offset == offset and member.offset_data == offset + 512 and decoded[offset:offset + 512] == header, "archive_framing")
                offset = member.offset_data + ((member.size + 511) // 512) * 512
                require(not any(decoded[member.offset_data + member.size:offset]), "archive_padding")
    except (tarfile.TarError, OSError, EOFError, ValueError, UnicodeError, zlib.error) as exc:
        raise Failure("bundle_archive") from exc
    return manifest


def verify_bundle(bundle):
    manifest_path = bundle / "distribution-manifest.json"
    require(stat.S_IMODE(manifest_path.lstat().st_mode) == 0o644, "bundle_mode")
    manifest_data = regular_bytes(manifest_path, MAX_MANIFEST)
    manifest = read_json(manifest_data)
    require(manifest_data == canonical(manifest), "manifest_encoding")
    # Admit the complete declared closure and global budgets before retaining
    # any untrusted payload. Then reject undeclared files before their reads.
    validate_manifest(manifest)
    payloads = {}
    total = 0
    directories = {str(parent) for name in manifest["inventory"] for parent in PurePosixPath(name).parents if str(parent) != "."}
    for path in tree_files(bundle, skip_pycache=False, allowed_directories=directories):
        name = path.relative_to(bundle).as_posix()
        if name == "distribution-manifest.json":
            continue
        require(name in manifest["inventory"], "inventory_mismatch")
        entry = manifest["inventory"][name]
        metadata = path.lstat()
        require(stat.S_IMODE(metadata.st_mode) == entry["mode"], "bundle_mode")
        require(metadata.st_size == entry["bytes"], "inventory_hash")
        total += metadata.st_size
        require(total <= MAX_TOTAL and len(payloads) < MAX_FILES, "archive_budget")
        data = regular_bytes(path, entry["bytes"])
        require(digest(data) == entry["sha256"], "inventory_hash")
        payloads[name] = data
    validate_payloads(manifest, payloads)
    return manifest, payloads


def write_archive(output, manifest):
    archive_path = output / "forever-extractor-linux.tar.gz"
    with archive_path.open("xb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=manifest["epoch"], compresslevel=9) as zipped, tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        names = sorted([*manifest["inventory"], "distribution-manifest.json"])
        for name in names:
            data = regular_bytes(output / name, 64 * 1024 * 1024)
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mtime = manifest["epoch"]
            info.mode = 0o755 if name == "bin/forever-extractor" else 0o644
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    verify_archive(archive_path)
    return archive_path


def build_bundle(source, output, cache, epoch):
    source = source.absolute()
    require(source.resolve() == source, "input_symlink")
    require(type(epoch) is int and 0 <= epoch <= 0xffffffff, "epoch")
    snapshot = source_snapshot(source)
    validate_lock(regular_bytes(source / "Cargo.lock"))
    require(tomllib.loads(regular_bytes(source / "rust-toolchain.toml").decode())["toolchain"]["channel"] == "1.98.0", "toolchain_version")
    output = new_output(output, source)
    for name, entry in snapshot.items():
        data = regular_bytes(source / name)
        require(digest(data) == entry["sha256"], "input_changed")
        write_new(output / "source" / name, data)
    vendor(output / "source", cache.absolute())
    notices = read_toolchain_notices()
    require(set(notices) == set(TOOLCHAIN_NOTICES), "toolchain_notice_closure")
    for name, data in notices.items():
        require(digest(data) == TOOLCHAIN_NOTICES[name], "toolchain_notice_checksum")
        write_new(output / "toolchain-notices" / name, data)
    frozen = file_inventory(output / "source")
    toolchain, target = compile_source(output, epoch)
    require(file_inventory(output / "source") == frozen, "frozen_source_changed")
    notice_documents(output / "toolchain-notices")
    require(source_snapshot(source) == snapshot, "input_changed")
    # Include only stable relative commands; exact subprocess output is retained
    # separately in build.log, whose source paths depend on the local build root.
    instructions = """# Local Linux distribution candidate\n\nThis bundle is not a complete extraction or runtime acceptance claim.\nPrerequisites: Python 3.11+, installed Rust/Cargo 1.98.0 for\nx86_64-unknown-linux-gnu, and a compatible GNU/Linux linker/runtime.\nNo native C++ reference or client files are included. No downloads are needed.
Original project notices include source/THIRD_PARTY_NOTICES.md. Rust standard
library attribution is retained under toolchain-notices/. Its broad license
directory does not imply every component named there is linked into this binary.\n\nVerify the archive with the original trusted tool before extracting it.\nAfter unpacking, rebuild in a new directory outside every Git checkout:\n\n    python3 source/tools/package_forever.py rebuild --bundle . --output /absolute/new-directory\n\nThe helper verifies all inventory bytes first, copies the corresponding source,\nuses only bundled exact registry archives/vendor source, clears inherited build\nsettings, and records its exact build log. It checks the rebuilt binary SHA256\nagainst this bundle. Binary identity is scoped to the recorded Rust toolchain\nand compatible external system linker/runtime. The manifest is not a signature.\n"""
    write_new(output / "BUILD.md", instructions.encode())
    write_new(output / "toolchain.json", canonical(toolchain))
    manifest = {"schema": SCHEMA, "platform": "linux", "target": target, "epoch": epoch, "toolchain": toolchain, "build": build_description(), "source_snapshot": {"kind": "current-working-tree-selected-source", "profile": SOURCE_PROFILE, "files": snapshot, "sha256": digest(canonical(snapshot))}, "dependencies": DEPENDENCIES, "toolchain_notices": TOOLCHAIN_NOTICES, "inventory": inventory_for(output), "full_extractor_complete": False, "publication": "local-candidate"}
    validate_manifest(manifest)
    write_new(output / "distribution-manifest.json", canonical(manifest))
    archive = write_archive(output, manifest)
    return {"schema": SCHEMA, "status": "created", "archive_sha256": digest(regular_bytes(archive, MAX_TOTAL)), "binary_sha256": manifest["inventory"]["bin/forever-extractor"]["sha256"], "source_sha256": manifest["source_snapshot"]["sha256"], "files": len(manifest["inventory"]), "full_extractor_complete": False}


def rebuild_bundle(bundle, output):
    bundle = bundle.absolute()
    require(bundle.resolve() == bundle, "input_symlink")
    manifest, payloads = verify_bundle(bundle)
    output = new_output(output, bundle)
    for name, data in payloads.items():
        if name.startswith(("source/", "toolchain-notices/")):
            write_new(output / name, data)
    frozen = file_inventory(output / "source")
    toolchain, target = compile_source(output, manifest["epoch"])
    require(file_inventory(output / "source") == frozen, "frozen_source_changed")
    notice_documents(output / "toolchain-notices")
    require(toolchain == manifest["toolchain"] and target == manifest["target"], "toolchain_mismatch")
    actual = digest(regular_bytes(output / "bin" / "forever-extractor", 64 * 1024 * 1024))
    require(actual == manifest["inventory"]["bin/forever-extractor"]["sha256"], "rebuild_mismatch")
    return {"schema": SCHEMA, "status": "rebuilt", "binary_sha256": actual, "full_extractor_complete": False}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("build")
    build.add_argument("--source", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--cargo-cache", type=Path, required=True)
    build.add_argument("--epoch", type=int, required=True)
    verify = commands.add_parser("verify")
    verify.add_argument("--archive", type=Path, required=True)
    rebuild = commands.add_parser("rebuild")
    rebuild.add_argument("--bundle", type=Path, required=True)
    rebuild.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "build":
            result = build_bundle(args.source, args.output, args.cargo_cache, args.epoch)
        elif args.command == "rebuild":
            result = rebuild_bundle(args.bundle, args.output)
        else:
            manifest = verify_archive(args.archive)
            result = {"schema": SCHEMA, "status": "verified", "files": len(manifest["inventory"]), "binary_sha256": manifest["inventory"]["bin/forever-extractor"]["sha256"], "full_extractor_complete": False}
        print(json.dumps(result, sort_keys=True))
        return 0
    except (Failure, OSError, ValueError, KeyError, TypeError) as exc:
        code = str(exc) if isinstance(exc, Failure) else "distribution_input"
        print(json.dumps({"schema": SCHEMA, "status": "rejected", "code": code, "full_extractor_complete": False}, sort_keys=True))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
