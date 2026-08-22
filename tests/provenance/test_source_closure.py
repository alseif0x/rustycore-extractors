#!/usr/bin/env python3
# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

"""Reject source-closure drift and loss of modification provenance."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
VENDOR = ROOT / "vendor" / "trinitycore-legacy"


def load_json(path: Path):
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


allowlist = load_json(ROOT / "licensing" / "source-files.json")
manifest = load_json(ROOT / "licensing" / "import-manifest.json")

expected = {entry["path"] for entry in allowlist["files"]}
actual = {
    path.relative_to(VENDOR).as_posix()
    for path in VENDOR.rglob("*")
    if path.is_file()
}

if len(expected) != manifest["verification"]["imported_path_count"]:
    raise SystemExit("manifest path count does not match the audited allowlist")
if actual != expected:
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    raise SystemExit(f"source closure drift: missing={missing}, extra={extra}")

modified = {entry["path"] for entry in manifest["modified_paths"]}
for relative in modified:
    text = (VENDOR / relative).read_text(encoding="utf-8", errors="replace")
    if "Modified by rustycore-extractors contributors on 2026-08-22" not in text:
        raise SystemExit(f"missing dated modification notice: {relative}")

blocked = (
    "dep/CascLib/src/hashes/md5.cpp",
    "dep/CascLib/src/hashes/sha1.cpp",
    "dep/CascLib/src/overwatch/apm.cpp",
    "dep/CascLib/src/overwatch/cmf.cpp",
    "dep/g3dlite/source/g3dfnmatch.cpp",
    "dep/g3dlite/include/G3D/g3dfnmatch.h",
)
for relative in blocked:
    if (VENDOR / relative).exists():
        raise SystemExit(f"blocked source was imported: {relative}")

casc_handles = (VENDOR / "src/tools/extractor_common/CascHandles.cpp").read_text(
    encoding="utf-8"
)
for forbidden in ("LoadTactKeys", "DownloadFile", "boost/asio", "boost::asio"):
    if forbidden in casc_handles:
        raise SystemExit(f"network implementation reintroduced: {forbidden}")
if "ERROR_NOT_SUPPORTED" not in casc_handles:
    raise SystemExit("remote CASC rejection is missing")

cmake = (ROOT / "CMakeLists.txt").read_text(encoding="utf-8")
if 'EXCLUDE REGEX "/Sockets\\\\.cpp$"' not in cmake:
    raise SystemExit("CascLib socket transport is not excluded from the build")
if "src/compat/CascSocketsDisabled.cpp" not in cmake:
    raise SystemExit("offline CascLib socket replacement is missing")

print(f"verified {len(actual)} imported paths and {len(modified)} modification notices")
