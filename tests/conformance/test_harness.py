# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

import json
import shutil
import struct
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.conformance.compare_outputs import compare, load_profile  # noqa: E402


def representative_map() -> bytes:
    area = struct.pack("<4sHH", b"AREA", 1, 42)
    heights = bytes(index % 256 for index in range(129 * 129 + 128 * 128))
    height = struct.pack("<4sIff", b"MHGT", 4, -12.5, 243.5) + heights
    liquid_types = b"".join(struct.pack("<H", index % 4) for index in range(16 * 16))
    liquid_cell_flags = bytes([1]) * (16 * 16)
    liquid_heights = struct.pack("<4f", 1.0, 2.0, 3.0, 4.0)
    liquid = struct.pack("<4sBBHBBBBf", b"MLIQ", 0, 1, 7, 3, 4, 2, 2, 4.5)
    liquid += liquid_types + liquid_cell_flags + liquid_heights
    holes = bytes([1]) + bytes(16 * 16 * 8 - 1)
    area_offset = 44
    height_offset = area_offset + len(area)
    liquid_offset = height_offset + len(height)
    holes_offset = liquid_offset + len(liquid)
    header = struct.pack(
        "<4s10I",
        b"MAPS", 10, 51943,
        area_offset, len(area), height_offset, len(height),
        liquid_offset, len(liquid), holes_offset, len(holes),
    )
    return header + area + height + liquid + holes


def empty_wdc4(suffix: bytes = b"") -> bytes:
    values = [0] * 17
    return struct.pack("<4s17I", b"WDC4", *values) + suffix


def empty_bih() -> bytes:
    return struct.pack("<6fII", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0, 0)


def empty_vmtree() -> bytes:
    return b"VMAP_4.B" + b"NODE" + empty_bih() + b"SIDX" + struct.pack("<I", 0)


def representative_vmo() -> bytes:
    vertices = struct.pack("<9f", 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
    triangles = struct.pack("<3I", 0, 1, 2)
    liquid = struct.pack("<II3fIf", 0, 0, 0.0, 0.0, 0.0, 7, 1.25)
    group = struct.pack("<6fII", 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0, 456)
    group += b"VERT" + struct.pack("<II", 4 + len(vertices), 3) + vertices
    group += b"TRIM" + struct.pack("<II", 4 + len(triangles), 1) + triangles
    group += b"MBIH" + empty_bih()
    group += b"LIQU" + struct.pack("<I", len(liquid)) + liquid
    return (
        b"VMAP_4.B" + b"WMOD" + struct.pack("<II", 8, 123)
        + b"GMOD" + struct.pack("<I", 1) + group + b"GBIH" + empty_bih()
    )


def representative_vmtile() -> bytes:
    m2_name = b"synthetic-model.m2"
    m2 = struct.pack("<BBI3f3ffI", 1, 9, 1001, 1.0, 2.0, 3.0, 0.0, 0.5, 1.0, 1.0, len(m2_name)) + m2_name
    wmo_name = b"synthetic-model.wmo"
    wmo = struct.pack(
        "<BBI3f3ff6fI", 2, 10, 1002, 4.0, 5.0, 6.0, 0.0, 0.0, 0.0, 1.0,
        -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, len(wmo_name),
    ) + wmo_name
    return b"VMAP_4.B" + struct.pack("<I", 2) + m2 + wmo


def mmap_params() -> bytes:
    return struct.pack("<5f2i", -17066.666, 0.0, -17066.666, 533.3333, 533.3333, 64, 32768)


def mmap_tile() -> bytes:
    mesh = struct.pack("<15i10f", 0x444E4156, 7, 32, 32, 0, 0, *([0] * 9), *([0.0] * 10))
    return struct.pack("<4IB3s", 0x4D4D4150, 7, 15, len(mesh), 1, b"\0\0\0") + mesh


def write(root: Path, relative: str, data: bytes) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def populate(root: Path, wdc_suffix: bytes = b"") -> None:
    write(root, "dbc/enUS/Map.db2", empty_wdc4(wdc_suffix))
    write(root, "dbc/esES/Map.db2", empty_wdc4(wdc_suffix))
    write(root, "maps/0000_32_32.map", representative_map())
    write(root, "vmaps/0000.vmtree", empty_vmtree())
    write(root, "vmaps/0000_32_32.vmtile", representative_vmtile())
    write(root, "vmaps/synthetic.vmo", representative_vmo())
    write(root, "vmaps/GameObjectModels.dtree", b"VMAP_4.B")
    write(root, "mmaps/0000.mmap", mmap_params())
    write(root, "mmaps/00003232.mmtile", mmap_tile())


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.reference = self.root / "reference"
        self.candidate = self.root / "candidate"
        populate(self.reference)
        shutil.copytree(self.reference, self.candidate)
        self.profile = load_profile(ROOT / "contracts" / "conformance-profile-v1.json")
        self.versions = {"reference": {"metadata": None}, "candidate": {"metadata": None}}

    def tearDown(self):
        self.temp.cleanup()

    def run_compare(self, profile=None):
        return compare(self.reference, self.candidate, profile or self.profile, self.versions)

    def test_identical_synthetic_outputs_pass_all_format_parsers(self):
        report = self.run_compare()
        self.assertEqual("pass", report["status"])
        self.assertEqual(9, report["summary"]["exact_match_count"])
        self.assertEqual(0, report["summary"]["mismatch_count"])
        self.assertFalse(report["privacy"]["absolute_paths_embedded"])
        self.assertIsNone(report["reader_acceptance"])

    def test_altered_header_is_reported(self):
        path = self.candidate / "maps/0000_32_32.map"
        data = bytearray(path.read_bytes())
        data[4:8] = (11).to_bytes(4, "little")
        path.write_bytes(data)
        report = self.run_compare()
        self.assertEqual("fail", report["status"])
        self.assertIn("FORMAT_INVALID", {item["code"] for item in report["mismatches"]})
        byte_mismatch = next(item for item in report["mismatches"] if item["code"] == "BYTE_MISMATCH")
        self.assertEqual(4, byte_mismatch["first_differing_byte"])

    def test_wrong_client_build_is_reported(self):
        path = self.candidate / "maps/0000_32_32.map"
        data = bytearray(path.read_bytes())
        data[8:12] = (54261).to_bytes(4, "little")
        path.write_bytes(data)
        report = self.run_compare()
        invalid = [item for item in report["mismatches"] if item["code"] == "FORMAT_INVALID"]
        self.assertTrue(any("expected 51943" in item["message"] for item in invalid))

    def test_field_ordering_change_is_reported(self):
        path = self.candidate / "maps/0000_32_32.map"
        data = bytearray(path.read_bytes())
        data[20:28] = struct.pack("<II", 16, 44)
        path.write_bytes(data)
        report = self.run_compare()
        self.assertIn("FORMAT_INVALID", {item["code"] for item in report["mismatches"]})

    def test_file_omission_is_reported(self):
        (self.candidate / "vmaps/0000.vmtree").unlink()
        report = self.run_compare()
        self.assertIn(
            {"code": "FILE_MISSING", "path": "vmaps/0000.vmtree"},
            report["mismatches"],
        )

    def test_unknown_artifact_is_rejected_even_when_present_on_both_sides(self):
        write(self.reference, "Buildings/raw-intermediate.bin", b"synthetic")
        write(self.candidate, "Buildings/raw-intermediate.bin", b"synthetic")
        report = self.run_compare()
        self.assertIn("UNSUPPORTED_ARTIFACT", {item["code"] for item in report["mismatches"]})

    def test_corrupt_and_incomplete_output_is_reported(self):
        path = self.candidate / "mmaps/00003232.mmtile"
        path.write_bytes(path.read_bytes()[:24])
        write(self.candidate, ".rustycore-extractors-incomplete.json", b"{}")
        report = self.run_compare()
        codes = {item["code"] for item in report["mismatches"]}
        self.assertIn("FORMAT_INVALID", codes)
        self.assertIn("INCOMPLETE_OUTPUT", codes)

    def test_semantic_exception_must_be_explicit_and_format_aware(self):
        shutil.rmtree(self.reference)
        shutil.rmtree(self.candidate)
        populate(self.reference, b"A")
        populate(self.candidate, b"B")
        strict = self.run_compare()
        self.assertEqual("fail", strict["status"])
        relaxed = self.run_compare({"schema_version": 1, "semantic_only": ["dbc/**/*.db2"]})
        self.assertEqual("pass", relaxed["status"])
        self.assertEqual(2, relaxed["summary"]["allowed_semantic_match_count"])

    def test_report_shape_contains_stable_target_and_no_input_roots(self):
        report = self.run_compare()
        encoded = json.dumps(report, sort_keys=True)
        self.assertEqual(51943, report["compatibility_target"]["build"])
        self.assertEqual(15, report["compatibility_target"]["formats"]["mmap"]["version"])
        self.assertNotIn(str(self.root), encoded)


if __name__ == "__main__":
    unittest.main()
