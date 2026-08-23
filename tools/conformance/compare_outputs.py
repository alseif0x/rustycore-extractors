#!/usr/bin/env python3
# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

"""Compare extractor output trees without copying client-derived data.

The report contains relative paths, hashes, sizes and parsed structural facts. It
never embeds file payloads or absolute input paths.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import math
import os
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


REPORT_VERSION = 1
ORACLE_COMMIT = "92796557f9b0ba3d2d1c7c770f535153154cf83e"
MAX_COUNT = 100_000_000
VMAP_MAGIC = b"VMAP_4.B"


class FormatError(ValueError):
    """A bounded parser rejected an extractor artifact."""


@dataclass
class Cursor:
    data: bytes
    offset: int = 0

    def take(self, size: int, label: str) -> bytes:
        if size < 0 or self.offset + size > len(self.data):
            raise FormatError(f"truncated {label} at byte {self.offset}")
        value = self.data[self.offset : self.offset + size]
        self.offset += size
        return value

    def unpack(self, fmt: str, label: str) -> tuple[Any, ...]:
        size = struct.calcsize(fmt)
        return struct.unpack(fmt, self.take(size, label))

    def u32(self, label: str) -> int:
        return self.unpack("<I", label)[0]

    def finish(self, label: str) -> None:
        if self.offset != len(self.data):
            raise FormatError(f"unexpected trailing bytes in {label}: {len(self.data) - self.offset}")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def first_difference(left: Path, right: Path) -> int | None:
    offset = 0
    with left.open("rb") as left_file, right.open("rb") as right_file:
        while True:
            left_chunk = left_file.read(1024 * 1024)
            right_chunk = right_file.read(1024 * 1024)
            common = min(len(left_chunk), len(right_chunk))
            for index in range(common):
                if left_chunk[index] != right_chunk[index]:
                    return offset + index
            if len(left_chunk) != len(right_chunk):
                return offset + common
            if not left_chunk:
                return None
            offset += len(left_chunk)


def checked_count(count: int, label: str) -> int:
    if count > MAX_COUNT:
        raise FormatError(f"unreasonable {label}: {count}")
    return count


def finite(values: tuple[float, ...], label: str) -> list[float]:
    if not all(math.isfinite(value) for value in values):
        raise FormatError(f"non-finite value in {label}")
    return list(values)


def parse_bih(cursor: Cursor, label: str) -> dict[str, Any]:
    bounds = finite(cursor.unpack("<6f", f"{label} bounds"), f"{label} bounds")
    tree_size = checked_count(cursor.u32(f"{label} tree size"), f"{label} tree size")
    tree = cursor.take(tree_size * 4, f"{label} tree")
    object_count = checked_count(cursor.u32(f"{label} object count"), f"{label} object count")
    objects = cursor.take(object_count * 4, f"{label} objects")
    return {
        "bounds": bounds,
        "tree_word_count": tree_size,
        "object_count": object_count,
        "tree_sha256": sha256(tree),
        "objects_sha256": sha256(objects),
    }


def parse_map(data: bytes) -> dict[str, Any]:
    if len(data) < 44:
        raise FormatError("file too short for map_fileheader")
    values = struct.unpack_from("<4s10I", data)
    magic = values[0]
    version, build = values[1:3]
    if magic != b"MAPS":
        raise FormatError(f"map magic is {magic!r}, expected b'MAPS'")
    if version != 10:
        raise FormatError(f"map version is {version}, expected 10")
    if build != 51943:
        raise FormatError(f"map build is {build}, expected 51943")
    names = ("area", "height", "liquid", "holes")
    sections: dict[str, Any] = {}
    for index, name in enumerate(names):
        offset, size = values[3 + index * 2 : 5 + index * 2]
        if (offset == 0) != (size == 0):
            raise FormatError(f"map {name} offset/size must both be zero or nonzero")
        if offset + size > len(data):
            raise FormatError(f"map {name} section exceeds file size")
        if size:
            sections[name] = {
                "offset": offset,
                "size": size,
                "sha256": sha256(data[offset : offset + size]),
            }

    if "area" in sections:
        offset, size = sections["area"]["offset"], sections["area"]["size"]
        if size < 8:
            raise FormatError("truncated AREA section")
        area_magic, flags, grid_area = struct.unpack_from("<4sHH", data, offset)
        if area_magic != b"AREA":
            raise FormatError("AREA chunk is out of order or has bad magic")
        expected = 8 if flags & 1 else 8 + 16 * 16 * 2
        if size != expected:
            raise FormatError(f"AREA size is {size}, expected {expected}")
        sections["area"].update(flags=flags, grid_area=grid_area)

    if "height" in sections:
        offset, size = sections["height"]["offset"], sections["height"]["size"]
        if size < 16:
            raise FormatError("truncated MHGT section")
        height_magic, flags, grid_min, grid_max = struct.unpack_from("<4sIff", data, offset)
        if height_magic != b"MHGT":
            raise FormatError("MHGT chunk is out of order or has bad magic")
        finite((grid_min, grid_max), "MHGT bounds")
        encodings = int(bool(flags & 1)) + int(bool(flags & 2)) + int(bool(flags & 4))
        if encodings > 1:
            raise FormatError("MHGT has mutually exclusive encodings")
        encoding = "flat" if flags & 1 else "uint16" if flags & 2 else "uint8" if flags & 4 else "float32"
        samples = 129 * 129 + 128 * 128
        expected = 16 if encoding == "flat" else 16 + samples * {"uint16": 2, "uint8": 1, "float32": 4}[encoding]
        if flags & 8:
            expected += 9 * 2 * 2
        if size != expected:
            raise FormatError(f"MHGT size is {size}, expected {expected} for {encoding}")
        sections["height"].update(
            flags=flags, encoding=encoding, grid_min=grid_min, grid_max=grid_max
        )

    if "liquid" in sections:
        offset, size = sections["liquid"]["offset"], sections["liquid"]["size"]
        if size < 16:
            raise FormatError("truncated MLIQ section")
        liquid = struct.unpack_from("<4sBBHBBBBf", data, offset)
        if liquid[0] != b"MLIQ":
            raise FormatError("MLIQ chunk is out of order or has bad magic")
        flags, liquid_flags, liquid_type, x, y, width, height, level = liquid[1:]
        finite((level,), "MLIQ level")
        if width > 129 or height > 129 or x + width > 129 or y + height > 129:
            raise FormatError("MLIQ dimensions exceed the 129x129 tile grid")
        expected = 16 + (0 if flags & 1 else 16 * 16 * 2) + (0 if flags & 2 else width * height * 4)
        if size != expected:
            raise FormatError(f"MLIQ size is {size}, expected {expected}")
        sections["liquid"].update(
            flags=flags,
            liquid_flags=liquid_flags,
            liquid_type=liquid_type,
            offset=[x, y],
            dimensions=[width, height],
            level=level,
        )

    if "holes" in sections and sections["holes"]["size"] != 16 * 16 * 8:
        raise FormatError("holes section must contain exactly 2048 bytes")

    return {"format": "map", "magic": "MAPS", "version": version, "build": build, "sections": sections}


def parse_wdc4(data: bytes) -> dict[str, Any]:
    if len(data) < 72:
        raise FormatError("file too short for WDC4 header")
    header = struct.unpack_from("<4s17I", data)
    if header[0] != b"WDC4":
        raise FormatError(f"DB2 magic is {header[0]!r}, expected b'WDC4'")
    names = (
        "record_count", "field_count", "record_size", "string_table_size", "table_hash",
        "layout_hash", "min_id", "max_id", "locale_and_flags", "id_index_and_flags",
        "total_field_count", "packed_data_offset", "lookup_column_count",
        "field_storage_info_size", "common_data_size", "pallet_data_size", "section_count",
    )
    values = dict(zip(names, header[1:]))
    section_count = checked_count(values["section_count"], "WDC4 section count")
    total_fields = max(values["field_count"], values["total_field_count"])
    checked_count(total_fields, "WDC4 field count")
    if values["field_storage_info_size"] % 24:
        raise FormatError("WDC4 field storage info size is not a multiple of 24")
    metadata_end = 72 + section_count * 40 + total_fields * 4 + values["field_storage_info_size"]
    metadata_end += values["pallet_data_size"] + values["common_data_size"]
    if metadata_end > len(data):
        raise FormatError("WDC4 metadata exceeds file size")
    sections = []
    for index in range(section_count):
        fields = struct.unpack_from("<Q8I", data, 72 + index * 40)
        file_offset = fields[1]
        if file_offset > len(data):
            raise FormatError(f"WDC4 section {index} starts outside file")
        sections.append({
            "tact_key_hash": f"{fields[0]:016x}",
            "file_offset": file_offset,
            "record_count": fields[2],
            "string_table_size": fields[3],
            "offset_records_end": fields[4],
            "id_list_size": fields[5],
            "relationship_data_size": fields[6],
            "offset_map_id_count": fields[7],
            "copy_table_count": fields[8],
        })
    values["locale"] = values.pop("locale_and_flags")
    packed_id = values.pop("id_index_and_flags")
    values["flags"] = packed_id & 0xFFFF
    values["id_index"] = packed_id >> 16
    return {"format": "db2-wdc4", "magic": "WDC4", "header": values, "sections": sections}


def parse_model_spawn(cursor: Cursor) -> dict[str, Any]:
    flags, adt_id, model_id = cursor.unpack("<BBI", "model spawn prefix")
    position = finite(cursor.unpack("<3f", "model position"), "model position")
    rotation = finite(cursor.unpack("<3f", "model rotation"), "model rotation")
    scale = finite(cursor.unpack("<f", "model scale"), "model scale")[0]
    bounds = None
    if flags & 2:
        bounds = finite(cursor.unpack("<6f", "model bounds"), "model bounds")
    name_length = checked_count(cursor.u32("model name length"), "model name length")
    if name_length > 16_384:
        raise FormatError(f"model name is unreasonably long: {name_length}")
    name = cursor.take(name_length, "model name")
    return {
        "flags": flags,
        "adt_id": adt_id,
        "id": model_id,
        "position": position,
        "rotation": rotation,
        "scale": scale,
        "bounds": bounds,
        "name_sha256": sha256(name),
    }


def parse_vmtile(data: bytes) -> dict[str, Any]:
    cursor = Cursor(data)
    if cursor.take(8, "VMap tile magic") != VMAP_MAGIC:
        raise FormatError("bad VMap tile magic")
    count = checked_count(cursor.u32("VMap spawn count"), "VMap spawn count")
    spawns = [parse_model_spawn(cursor) for _ in range(count)]
    cursor.finish("VMap tile")
    canonical = json.dumps(spawns, sort_keys=True, separators=(",", ":")).encode()
    return {
        "format": "vmap-tile", "magic": VMAP_MAGIC.decode(), "spawn_count": count,
        "m2_count": sum(bool(item["flags"] & 1) for item in spawns),
        "bounded_count": sum(item["bounds"] is not None for item in spawns),
        "spawns_sha256": sha256(canonical),
    }


def parse_vmtree(data: bytes) -> dict[str, Any]:
    cursor = Cursor(data)
    if cursor.take(8, "VMap tree magic") != VMAP_MAGIC:
        raise FormatError("bad VMap tree magic")
    if cursor.take(4, "NODE chunk") != b"NODE":
        raise FormatError("missing NODE chunk")
    tree = parse_bih(cursor, "map BIH")
    if cursor.take(4, "SIDX chunk") != b"SIDX":
        raise FormatError("missing SIDX chunk")
    count = checked_count(cursor.u32("spawn index count"), "spawn index count")
    spawn_ids = cursor.take(count * 4, "spawn indices")
    cursor.finish("VMap tree")
    return {
        "format": "vmap-tree", "magic": VMAP_MAGIC.decode(), "tree": tree,
        "spawn_index_count": count, "spawn_indices_sha256": sha256(spawn_ids),
    }


def parse_vmo(data: bytes) -> dict[str, Any]:
    cursor = Cursor(data)
    if cursor.take(8, "VMap model magic") != VMAP_MAGIC:
        raise FormatError("bad VMap model magic")
    if cursor.take(4, "WMOD chunk") != b"WMOD":
        raise FormatError("missing WMOD chunk")
    if cursor.u32("WMOD size") != 8:
        raise FormatError("WMOD size must be 8")
    root_id = cursor.u32("root WMO id")
    group_count = vertex_count = triangle_count = liquid_count = 0
    if cursor.offset < len(data):
        if cursor.take(4, "GMOD chunk") != b"GMOD":
            raise FormatError("missing GMOD chunk")
        group_count = checked_count(cursor.u32("group count"), "group count")
        for group in range(group_count):
            finite(cursor.unpack("<6f", f"group {group} bounds"), f"group {group} bounds")
            cursor.unpack("<II", f"group {group} flags and id")
            if cursor.take(4, "VERT chunk") != b"VERT":
                raise FormatError("missing VERT chunk")
            vert_size = cursor.u32("VERT size")
            verts = checked_count(cursor.u32("vertex count"), "vertex count")
            if vert_size != 4 + verts * 12:
                raise FormatError("VERT chunk size/count disagree")
            cursor.take(verts * 12, "vertices")
            vertex_count += verts
            if verts == 0:
                continue
            if cursor.take(4, "TRIM chunk") != b"TRIM":
                raise FormatError("missing TRIM chunk")
            tri_size = cursor.u32("TRIM size")
            triangles = checked_count(cursor.u32("triangle count"), "triangle count")
            if tri_size != 4 + triangles * 12:
                raise FormatError("TRIM chunk size/count disagree")
            cursor.take(triangles * 12, "triangles")
            triangle_count += triangles
            if cursor.take(4, "MBIH chunk") != b"MBIH":
                raise FormatError("missing MBIH chunk")
            parse_bih(cursor, f"group {group} BIH")
            if cursor.take(4, "LIQU chunk") != b"LIQU":
                raise FormatError("missing LIQU chunk")
            liquid_size = cursor.u32("LIQU size")
            if liquid_size:
                liquid = Cursor(cursor.take(liquid_size, "liquid payload"))
                tiles_x, tiles_y = liquid.unpack("<II", "liquid dimensions")
                checked_count(tiles_x, "liquid width")
                checked_count(tiles_y, "liquid height")
                finite(liquid.unpack("<3f", "liquid corner"), "liquid corner")
                liquid.u32("liquid type")
                liquid_samples = (tiles_x + 1) * (tiles_y + 1) * 4 + tiles_x * tiles_y if tiles_x and tiles_y else 4
                liquid.take(liquid_samples, "liquid samples")
                liquid.finish("liquid payload")
                liquid_count += 1
        if group_count:
            if cursor.take(4, "GBIH chunk") != b"GBIH":
                raise FormatError("missing GBIH chunk")
            parse_bih(cursor, "group BIH")
    cursor.finish("VMap model")
    return {
        "format": "vmap-model", "magic": VMAP_MAGIC.decode(), "root_wmo_id": root_id,
        "group_count": group_count, "vertex_count": vertex_count,
        "triangle_count": triangle_count, "liquid_group_count": liquid_count,
    }


def parse_gameobject_models(data: bytes) -> dict[str, Any]:
    cursor = Cursor(data)
    if cursor.take(8, "game-object model magic") != VMAP_MAGIC:
        raise FormatError("bad game-object model magic")
    count = wmo_count = 0
    semantic = hashlib.sha256()
    while cursor.offset < len(data):
        display_id, is_wmo, name_length = cursor.unpack("<IBI", "game-object model record")
        checked_count(name_length, "game-object model name length")
        name = cursor.take(name_length, "game-object model name")
        bounds = cursor.take(24, "game-object model bounds")
        semantic.update(struct.pack("<IBI", display_id, is_wmo, name_length))
        semantic.update(hashlib.sha256(name).digest())
        semantic.update(bounds)
        count += 1
        wmo_count += bool(is_wmo)
    return {
        "format": "gameobject-model-index", "magic": VMAP_MAGIC.decode(),
        "model_count": count, "wmo_count": wmo_count, "records_sha256": semantic.hexdigest(),
    }


def parse_mmap(data: bytes) -> dict[str, Any]:
    if len(data) != 28:
        raise FormatError(f".mmap size is {len(data)}, expected 28")
    # Detour stores these as signed int, but the pinned 64-bit polygon-reference
    # build intentionally writes bit 31 for maxPolys. Interpret the serialized
    # bit pattern as unsigned so the required 2^31 capacity is not rejected.
    origin_x, origin_y, origin_z, width, height, max_tiles, max_polys = struct.unpack("<5f2I", data)
    values = finite((origin_x, origin_y, origin_z, width, height), "Detour navmesh parameters")
    if width <= 0 or height <= 0 or max_tiles <= 0 or max_polys <= 0:
        raise FormatError("invalid Detour navmesh parameters")
    return {
        "format": "mmap-params", "origin": values[:3], "tile_width": width,
        "tile_height": height, "max_tiles": max_tiles, "max_polys": max_polys,
        "poly_ref_bits": 64,
    }


def parse_mmtile(data: bytes) -> dict[str, Any]:
    if len(data) < 20:
        raise FormatError("file too short for MmapTileHeader")
    mmap_magic, dt_version, mmap_version, size, uses_liquids, padding = struct.unpack_from("<4IB3s", data)
    if mmap_magic != 0x4D4D4150:
        raise FormatError(f"MMap magic is 0x{mmap_magic:08x}, expected 0x4d4d4150")
    if dt_version != 7:
        raise FormatError(f"Detour version is {dt_version}, expected 7")
    if mmap_version != 15:
        raise FormatError(f"MMap version is {mmap_version}, expected 15")
    if padding != b"\0\0\0":
        raise FormatError("MMap padding is not initialized to zero")
    if size != len(data) - 20:
        raise FormatError(f"MMap payload size is {size}, available {len(data) - 20}")
    if size < 100:
        raise FormatError("Detour tile is too short for dtMeshHeader")
    mesh = struct.unpack_from("<15i10f", data, 20)
    if mesh[0] != 0x444E4156 or mesh[1] != 7:
        raise FormatError("Detour tile magic/version mismatch")
    counts = mesh[6:15]
    if any(value < 0 for value in counts):
        raise FormatError("negative count in Detour mesh header")
    finite(tuple(mesh[15:]), "Detour mesh header floats")
    expected_size = (
        100
        + mesh[7] * 12  # vertices
        + mesh[6] * 32  # dtPoly with 64-bit polygon references
        + mesh[8] * 16  # dtLink with 64-bit polygon references
        + mesh[9] * 12  # dtPolyDetail
        + mesh[10] * 12  # detail vertices
        + mesh[11] * 4  # detail triangles
        + mesh[12] * 16  # dtBVNode
        + mesh[13] * 36  # dtOffMeshConnection
    )
    if size != expected_size:
        raise FormatError(f"Detour payload size is {size}, expected {expected_size} from mesh counts")
    return {
        "format": "mmap-tile", "mmap_magic": "MMAP", "mmap_version": mmap_version,
        "detour_version": dt_version, "poly_ref_bits": 64, "payload_size": size,
        "uses_liquids": bool(uses_liquids), "tile": [mesh[2], mesh[3], mesh[4]],
        "poly_count": mesh[6], "vertex_count": mesh[7], "detail_triangle_count": mesh[11],
        "off_mesh_connection_count": mesh[13],
    }


PARSERS: list[tuple[Callable[[str], bool], str, Callable[[bytes], dict[str, Any]]]] = [
    (lambda path: path.endswith(".map"), "map", parse_map),
    (lambda path: path.endswith(".db2"), "db2", parse_wdc4),
    (lambda path: path.endswith(".vmtree"), "vmap-tree", parse_vmtree),
    (lambda path: path.endswith(".vmtile"), "vmap-tile", parse_vmtile),
    (lambda path: path.endswith(".vmo"), "vmap-model", parse_vmo),
    (lambda path: path == "vmaps/GameObjectModels.dtree", "vmap-gameobjects", parse_gameobject_models),
    (lambda path: path.endswith(".mmap"), "mmap-params", parse_mmap),
    (lambda path: path.endswith(".mmtile"), "mmap-tile", parse_mmtile),
]


def parser_for(path: str) -> tuple[str, Callable[[bytes], dict[str, Any]]] | None:
    for predicate, kind, parser in PARSERS:
        if predicate(path):
            return kind, parser
    return None


def allowed_opaque(path: str) -> bool:
    patterns = (
        "data-manifest.json",
        "gt/*.txt",
        "cameras/FILE????????.xxx",
        "maps/*.tilelist",
    )
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def inventory(root: Path, side: str) -> tuple[dict[str, dict[str, Any]], list[dict[str, Any]]]:
    files: dict[str, dict[str, Any]] = {}
    errors: list[dict[str, Any]] = []
    if not root.is_dir():
        return files, [{"code": "ROOT_INVALID", "side": side, "message": "input root is not a directory"}]
    for directory, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        filenames.sort()
        base = Path(directory)
        for dirname in list(dirnames):
            child = base / dirname
            if child.is_symlink():
                relative = child.relative_to(root).as_posix()
                errors.append({"code": "UNSAFE_SYMLINK", "side": side, "path": relative})
                dirnames.remove(dirname)
        for filename in filenames:
            full = base / filename
            relative = full.relative_to(root).as_posix()
            if full.is_symlink():
                errors.append({"code": "UNSAFE_SYMLINK", "side": side, "path": relative})
                continue
            if filename == ".rustycore-extractors-incomplete.json":
                errors.append({"code": "INCOMPLETE_OUTPUT", "side": side, "path": relative})
            data = full.read_bytes()
            selected = parser_for(relative)
            kind = selected[0] if selected else "opaque"
            semantic = None
            if selected is None and not allowed_opaque(relative) and filename != ".rustycore-extractors-incomplete.json":
                errors.append({"code": "UNSUPPORTED_ARTIFACT", "side": side, "path": relative})
            if selected:
                try:
                    semantic = selected[1](data)
                except (FormatError, struct.error, UnicodeError) as error:
                    errors.append({"code": "FORMAT_INVALID", "side": side, "path": relative, "message": str(error)})
            files[relative] = {"path": relative, "kind": kind, "size": len(data), "sha256": sha256(data), "semantic": semantic}
    return files, errors


def load_profile(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as source:
        profile = json.load(source)
    if profile.get("schema_version") != 1:
        raise ValueError("unsupported conformance profile version")
    patterns = profile.get("semantic_only", [])
    if not isinstance(patterns, list) or not all(isinstance(item, str) for item in patterns):
        raise ValueError("profile semantic_only must be a string array")
    return profile


def is_semantic_only(path: str, profile: dict[str, Any]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in profile.get("semantic_only", []))


def compare(
    reference_root: Path,
    candidate_root: Path,
    profile: dict[str, Any],
    versions: dict[str, Any],
    reader_acceptance: dict[str, Any] | None = None,
) -> dict[str, Any]:
    reference, errors = inventory(reference_root, "reference")
    candidate, candidate_errors = inventory(candidate_root, "candidate")
    errors.extend(candidate_errors)
    reference_paths = set(reference)
    candidate_paths = set(candidate)
    missing = sorted(reference_paths - candidate_paths)
    unexpected = sorted(candidate_paths - reference_paths)
    mismatches: list[dict[str, Any]] = []
    exact_count = semantic_count = 0
    for path in sorted(reference_paths & candidate_paths):
        left, right = reference[path], candidate[path]
        semantic_equal = left["semantic"] is not None and left["semantic"] == right["semantic"]
        exact_equal = left["sha256"] == right["sha256"] and left["size"] == right["size"]
        semantic_only = is_semantic_only(path, profile)
        passed = semantic_equal if semantic_only else exact_equal
        if exact_equal:
            exact_count += 1
        elif semantic_equal:
            semantic_count += 1
        if not passed:
            mismatch = {
                "code": "SEMANTIC_MISMATCH" if semantic_only else "BYTE_MISMATCH",
                "path": path,
                "comparison": "semantic" if semantic_only else "exact",
                "reference": left,
                "candidate": right,
            }
            if not exact_equal:
                mismatch["first_differing_byte"] = first_difference(reference_root / path, candidate_root / path)
            mismatches.append(mismatch)
    for path in missing:
        mismatches.append({"code": "FILE_MISSING", "path": path})
    for path in unexpected:
        mismatches.append({"code": "FILE_UNEXPECTED", "path": path})
    for error in errors:
        mismatches.append(error)
    status = "pass" if not mismatches else "fail"
    categories: dict[str, int] = {}
    for item in candidate.values():
        categories[item["kind"]] = categories.get(item["kind"], 0) + 1
    return {
        "schema_version": REPORT_VERSION,
        "status": status,
        "compatibility_target": {
            "product": "wow_classic", "version": "3.4.3", "build": 51943,
            "oracle_commit": ORACLE_COMMIT,
            "formats": {"map": {"magic": "MAPS", "version": 10}, "vmap": {"magic": "VMAP_4.B"}, "mmap": {"magic": "MMAP", "version": 15, "detour_version": 7, "poly_ref_bits": 64}},
        },
        "tools": versions,
        "reader_acceptance": reader_acceptance,
        "policy": {"semantic_only": profile.get("semantic_only", []), "default": "exact-bytes"},
        "summary": {
            "reference_file_count": len(reference), "candidate_file_count": len(candidate),
            "common_file_count": len(reference_paths & candidate_paths), "exact_match_count": exact_count,
            "allowed_semantic_match_count": semantic_count, "mismatch_count": len(mismatches),
            "candidate_categories": dict(sorted(categories.items())),
        },
        "mismatches": mismatches,
        "privacy": {"absolute_paths_embedded": False, "client_payloads_embedded": False},
    }


def parse_version(path: Path | None, role: str) -> dict[str, Any]:
    if path is None:
        return {"role": role, "metadata": None}
    with path.open(encoding="utf-8") as source:
        value = json.load(source)
    return {"role": role, "metadata": value}


def load_optional_json(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    with path.open(encoding="utf-8") as source:
        value = json.load(source)
    if not isinstance(value, dict):
        raise ValueError("reader evidence must be a JSON object")
    return value


def safe_report_path(path: Path, input_roots: tuple[Path, Path]) -> None:
    if path.exists() and path.is_symlink():
        raise ValueError("report path must not be a symlink")
    resolved = path.resolve()
    for root in input_roots:
        try:
            resolved.relative_to(root.resolve())
        except ValueError:
            continue
        raise ValueError("report path must be outside both read-only input roots")
    path.parent.mkdir(parents=True, exist_ok=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--profile", type=Path, default=Path(__file__).resolve().parents[2] / "contracts" / "conformance-profile-v1.json")
    parser.add_argument("--reference-version-json", type=Path)
    parser.add_argument("--candidate-version-json", type=Path)
    parser.add_argument("--reader-evidence-json", type=Path)
    arguments = parser.parse_args(argv)
    try:
        profile = load_profile(arguments.profile)
        report = compare(
            arguments.reference,
            arguments.candidate,
            profile,
            {
                "reference": parse_version(arguments.reference_version_json, "reference"),
                "candidate": parse_version(arguments.candidate_version_json, "candidate"),
            },
            load_optional_json(arguments.reader_evidence_json),
        )
        safe_report_path(arguments.report, (arguments.reference, arguments.candidate))
        arguments.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"conformance harness error: {error}", file=sys.stderr)
        return 2
    print(json.dumps({"status": report["status"], "mismatch_count": report["summary"]["mismatch_count"], "report": arguments.report.name}, sort_keys=True))
    return 0 if report["status"] == "pass" else 9


if __name__ == "__main__":
    raise SystemExit(main())
