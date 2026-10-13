#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate pinned WoWDBDefs schema catalogs for supported Forever builds.

The parser is deliberately strict and dependency-free. It reads only the
pinned definition and manifest files from a local WoWDBDefs checkout. It does
not read client data or use the network.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

PIN = "674636522d51c010f250ad1b5fe549ef6bc8eb0a"
MANIFEST_PATH = "manifest.json"
MANIFEST_BLOB = "8a5d8122f312ae6a9564e8286fdc491831fecc22"
MANIFEST_SHA256 = "4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265"
TARGET_BUILD = (1, 60, 1, 70205)
EXPECTED_TABLES = 1161
ROOT = Path(__file__).resolve().parents[1]
FACTS_PATH = ROOT / "contracts/wowdbdefs-70205-v1.json"
RUST_PATH = ROOT / "crates/forever-db2/src/records/wowdbdefs_70205_generated.rs"
CATALOG_ID = "wowdbdefs-70205-v1"
TARGETS = {
    70205: {
        "pin": PIN,
        "catalog_id": "wowdbdefs-70205-v1",
        "facts_path": FACTS_PATH,
        "rust_path": RUST_PATH,
    },
    70338: {
        "pin": "d6f50d4f351193e84226aa62a36faa0f58528afa",
        "catalog_id": "wowdbdefs-70338-v1",
        "facts_path": ROOT / "contracts/wowdbdefs-70338-v1.json",
        "rust_path": ROOT / "crates/forever-db2/src/records/wowdbdefs_70338_generated.rs",
    },
}

COLUMN = re.compile(
    r"^(int|uint|float|string|locstring)(?:<[^<>]+>)?\s+"
    r"([A-Za-z_][A-Za-z0-9_]*)\??$"
)
ENTRY = re.compile(
    r"^(?:\$(?P<annotations>[A-Za-z_][A-Za-z0-9_]*(?:,[A-Za-z_][A-Za-z0-9_]*)*)\$)?"
    r"(?P<column>[A-Za-z_][A-Za-z0-9_]*)"
    r"(?:<(?P<unsigned>u?)(?P<width>[0-9]+)>)?"
    r"(?:\[(?P<array>[0-9]+)\])?$"
)
LAYOUT = re.compile(r"^LAYOUT (.+)$")
VERSION = re.compile(
    r"^([0-9]+)\.([0-9]+)\.([0-9]+)\.([0-9]+)"
    r"(?:-([0-9]+)\.([0-9]+)\.([0-9]+)\.([0-9]+))?$"
)
KINDS = {
    "int": "Int",
    "uint": "Int",
    "float": "Float",
    "string": "NonLocalizedString",
    "locstring": "String",
}
BLOCKERS = {
    "signed_64_bit": "Signed64Bit",
    "string_array": "StringArray",
    "unsigned_appended_parent": "UnsignedAppendedParent",
}


class CatalogError(ValueError):
    pass


def _integer_version(text):
    match = VERSION.fullmatch(text)
    if match is None:
        raise CatalogError("invalid_build_version")
    values = [int(value) for value in match.groups()[:4]]
    return tuple(values)


def _build_range(text):
    left, separator, right = text.partition("-")
    start = _integer_version(left)
    end = _integer_version(right) if separator else start
    if end < start:
        raise CatalogError("invalid_build_range")
    return start, end


def _covers_target(build):
    return build[0] <= TARGET_BUILD <= build[1]


def _without_comment(line):
    return line.split("//", 1)[0].strip()


def _parse_columns(lines, path):
    if not lines or lines[0].strip() != "COLUMNS":
        raise CatalogError("invalid_columns_header:" + path)
    columns = {}
    index = 1
    while index < len(lines):
        line = _without_comment(lines[index])
        if not line:
            index += 1
            continue
        if line.startswith(("LAYOUT ", "BUILD ", "COMMENT ")):
            break
        match = COLUMN.fullmatch(line)
        if match is None:
            raise CatalogError("invalid_column_declaration:" + path)
        kind, name = match.groups()
        if name in columns:
            raise CatalogError("duplicate_column:" + path)
        columns[name] = kind
        index += 1
    return columns, index


def parse_dbd(source, path):
    try:
        lines = source.decode("utf-8").splitlines()
    except UnicodeError as error:
        raise CatalogError("invalid_dbd_utf8:" + path) from error
    columns, index = _parse_columns(lines, path)
    definitions = []
    current = None

    def flush():
        nonlocal current
        if current is not None:
            definitions.append(current)
            current = None

    while index < len(lines):
        line = _without_comment(lines[index])
        index += 1
        if not line:
            continue
        if line.startswith(("LAYOUT ", "BUILD ", "COMMENT ")):
            if current is not None and current["entries"]:
                flush()
            if current is None:
                current = {"layouts": [], "builds": [], "entries": []}
            if line.startswith("LAYOUT "):
                match = LAYOUT.fullmatch(line)
                if match is None:
                    raise CatalogError("invalid_layout:" + path)
                values = [value.strip() for value in match.group(1).split(",")]
                if not values or any(re.fullmatch(r"[0-9a-fA-F]{8}", value) is None for value in values):
                    raise CatalogError("invalid_layout:" + path)
                current["layouts"].extend(int(value, 16) for value in values)
            elif line.startswith("BUILD "):
                values = [value.strip() for value in line[6:].split(",")]
                if not values or any(not value for value in values):
                    raise CatalogError("invalid_build_list:" + path)
                current["builds"].extend(_build_range(value) for value in values)
            continue
        if current is None:
            raise CatalogError("entry_without_definition:" + path)
        match = ENTRY.fullmatch(line)
        if match is None:
            raise CatalogError("invalid_layout_entry:" + path)
        annotations = match.group("annotations")
        annotations = annotations.split(",") if annotations else []
        column = match.group("column")
        if column not in columns:
            raise CatalogError("entry_without_column:" + path)
        width = int(match.group("width")) if match.group("width") else None
        array = int(match.group("array")) if match.group("array") else 1
        if array < 1 or array > 255:
            raise CatalogError("invalid_array_size:" + path)
        if width is not None and width not in (8, 16, 32, 64):
            raise CatalogError("invalid_integer_width:" + path)
        current["entries"].append({
            "column": column,
            "annotations": annotations,
            "width": width,
            "unsigned_width": match.group("unsigned") == "u",
            "array_size": array,
        })
    flush()
    return columns, definitions


def _field_kind(column_type, width, array_size, unsigned):
    if column_type in ("string", "locstring"):
        return "String" if array_size == 1 else f"StringArray({array_size})"
    if column_type == "float":
        return f"Float({array_size})"
    if column_type not in ("int", "uint"):
        raise CatalogError("unsupported_column_type")
    bits = width or 32
    signed = column_type == "int" and not unsigned
    return f"{'Signed' if signed else 'Unsigned'}({bits}, {array_size})"


def _metadata(name, definitions, columns, manifest_item):
    matches = [
        definition for definition in definitions
        if any(_covers_target(build) for build in definition["builds"])
    ]
    if len(matches) != 1:
        raise CatalogError("target_definition_count:" + name)
    definition = matches[0]
    if not definition["layouts"] or not definition["entries"]:
        raise CatalogError("target_layout_shape:" + name)
    if manifest_item is None or not manifest_item.get("db2FileDataID"):
        raise CatalogError("target_manifest_identity:" + name)
    table_hash = manifest_item.get("tableHash")
    if not isinstance(table_hash, str) or re.fullmatch(r"[0-9a-fA-F]{8}", table_hash) is None:
        raise CatalogError("target_manifest_table_hash:" + name)

    fields = []
    decode_kinds = []
    index_field = -1
    parent_index = -1
    file_field_count = 0
    seen_id = False
    seen_relation = False
    appended_relation = None
    for entry in definition["entries"]:
        annotations = set(entry["annotations"])
        kind = columns[entry["column"]]
        noninline = "noninline" in annotations
        is_id = "id" in annotations
        is_relation = "relation" in annotations
        if is_id and seen_id:
            raise CatalogError("duplicate_id_annotation:" + name)
        if is_relation and seen_relation:
            raise CatalogError("duplicate_relation_annotation:" + name)
        seen_id |= is_id
        seen_relation |= is_relation

        unsigned = kind == "uint" or entry["unsigned_width"]
        if is_id:
            unsigned = True
        field_kind = KINDS[kind]
        field = {
            "kind": field_kind,
            "array_size": entry["array_size"],
            "signed": kind in ("int", "float", "string", "locstring") and not unsigned,
        }
        if not noninline:
            if is_id:
                if kind not in ("int", "uint") or entry["array_size"] != 1:
                    raise CatalogError("invalid_inline_id:" + name)
                if (entry["width"] or 32) > 32:
                    raise CatalogError("invalid_inline_id_width:" + name)
                index_field = file_field_count
            if is_relation:
                if kind not in ("int", "uint") or entry["array_size"] != 1:
                    raise CatalogError("invalid_inline_relation:" + name)
                if (entry["width"] or 32) > 32:
                    raise CatalogError("invalid_inline_relation_width:" + name)
                parent_index = file_field_count
            fields.append(field)
            decode_kinds.append(_field_kind(
                kind, entry["width"], entry["array_size"], unsigned
            ))
            file_field_count += 1
        elif is_relation:
            if kind not in ("int", "uint") or entry["array_size"] != 1:
                raise CatalogError("invalid_appended_relation:" + name)
            if (entry["width"] or 32) > 32:
                raise CatalogError("invalid_appended_relation_width:" + name)
            appended_relation = field
        elif is_id:
            if kind not in ("int", "uint") or entry["array_size"] != 1:
                raise CatalogError("invalid_noninline_id:" + name)

    if appended_relation is not None:
        parent_index = file_field_count
        fields.append(appended_relation)
    total_field_count = len(fields)
    if file_field_count == 0 or total_field_count < file_field_count:
        raise CatalogError("invalid_target_field_count:" + name)
    if not seen_id:
        raise CatalogError("missing_id_annotation:" + name)
    if parent_index >= 0 and parent_index >= total_field_count:
        raise CatalogError("invalid_parent_index:" + name)
    return {
        "name": name,
        "file_data_id": int(manifest_item["db2FileDataID"]),
        "table_hash": int(table_hash, 16),
        "layout_hash": definition["layouts"][0],
        "other_layout_hashes": definition["layouts"][1:],
        "index": index_field,
        "parent_index": parent_index,
        "field_count": total_field_count,
        "file_field_count": file_field_count,
        "fields": fields,
        "decode_kinds": decode_kinds,
        "blockers": [],
    }


def pinned_source(reference):
    commit = subprocess.check_output(
        ["git", "-C", str(reference), "rev-parse", "HEAD"],
        stderr=subprocess.DEVNULL,
    ).decode().strip()
    if commit != PIN:
        raise CatalogError("pinned_source_mismatch")
    changed = subprocess.check_output(
        ["git", "-C", str(reference), "status", "--porcelain", "--untracked-files=all",
         "--", "manifest.json", "definitions"],
        stderr=subprocess.DEVNULL,
    )
    if changed:
        raise CatalogError("pinned_source_worktree_modified")
    blob = subprocess.check_output(
        ["git", "-C", str(reference), "rev-parse", PIN + ":" + MANIFEST_PATH],
        stderr=subprocess.DEVNULL,
    ).decode().strip()
    raw_manifest = (reference / MANIFEST_PATH).read_bytes()
    if blob != MANIFEST_BLOB or hashlib.sha256(raw_manifest).hexdigest() != MANIFEST_SHA256:
        raise CatalogError("pinned_manifest_mismatch")
    return raw_manifest


def document(reference, raw_manifest):
    try:
        manifest_items = json.loads(raw_manifest)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise CatalogError("invalid_source_manifest") from error
    manifest = {item["tableName"]: item for item in manifest_items}
    tables = []
    paths = sorted((reference / "definitions").glob("*.dbd"))
    for path in paths:
        relative = "definitions/" + path.name
        blob = subprocess.check_output(
            ["git", "-C", str(reference), "rev-parse", PIN + ":" + relative],
            stderr=subprocess.DEVNULL,
        ).decode().strip()
        raw = path.read_bytes()
        if subprocess.check_output(
            ["git", "-C", str(reference), "hash-object", "--stdin"],
            input=raw,
            stderr=subprocess.DEVNULL,
        ).decode().strip() != blob:
            raise CatalogError("pinned_definition_mismatch")
        columns, definitions = parse_dbd(raw, relative)
        target_definitions = [
            definition for definition in definitions
            if any(_covers_target(build) for build in definition["builds"])
        ]
        if not target_definitions:
            continue
        tables.append(_metadata(path.stem, definitions, columns, manifest.get(path.stem)))
    tables.sort(key=lambda table: table["name"])
    if len(tables) != EXPECTED_TABLES:
        raise CatalogError("pinned_table_count_mismatch")
    names = [table["name"] for table in tables]
    ids = [table["file_data_id"] for table in tables]
    if len(set(names)) != len(names) or len(set(ids)) != len(ids):
        raise CatalogError("duplicate_target_identity")
    return {
        "catalog_version": 1,
        "catalog_id": CATALOG_ID,
        "target": {
            "product": "wow_classic_beta",
            "version": ".".join(str(value) for value in TARGET_BUILD),
            "build": TARGET_BUILD[-1],
        },
        "source": {
            "repository": "wowdev/WoWDBDefs",
            "commit": PIN,
            "manifest_path": MANIFEST_PATH,
            "manifest_blob": MANIFEST_BLOB,
            "manifest_sha256": MANIFEST_SHA256,
            "license": "CC-BY-SA-4.0",
            "attribution": "WoWDBDefs contributors",
        },
        "scope": "Build-specific DB2 schema facts; does not establish acquisition or runtime acceptance.",
        "tables": tables,
    }


def _rust_kind(kind):
    return "Kind::" + kind


def render_rust(doc):
    lines = [
        "// SPDX-License-Identifier: CC-BY-SA-4.0",
        "// Generated by tools/generate_wowdbdefs_catalog.py; do not edit by hand.",
        "// Adapted schema facts from WoWDBDefs contributors; see contracts/WOWDBDEFS-LICENSE.md.",
        f"// Source pin {PIN}; manifest blob {MANIFEST_BLOB}.",
        f"// Source manifest SHA-256 {MANIFEST_SHA256}.",
        "use super::super::schema::Kind;",
        "use super::{FieldMetadata, FieldType, TableMetadata};",
        "#[rustfmt::skip]",
        "pub(super) static TABLES: &[TableMetadata] = &[",
    ]
    for table in doc["tables"]:
        fields = ", ".join(
            "FieldMetadata { kind: FieldType::" + field["kind"]
            + f', array_size: {field["array_size"]}, signed: {str(field["signed"]).lower()} }}'
            for field in table["fields"]
        )
        blockers = ", ".join(
            "CatalogBlocker::" + BLOCKERS[blocker] for blocker in table["blockers"]
        )
        decode = ", ".join(_rust_kind(kind) for kind in table["decode_kinds"])
        lines.append(
            f'    TableMetadata {{ name: "{table["name"]}", file_data_id: {table["file_data_id"]}, '
            f'table_hash: Some(0x{table["table_hash"]:08X}), '
            f'other_layout_hashes: &[{", ".join(f"0x{value:08X}" for value in table["other_layout_hashes"])}], '
            f'layout_hash: 0x{table["layout_hash"]:08X}, index: {table["index"]}, '
            f'parent_index: {table["parent_index"]}, field_count: {table["field_count"]}, '
            f'file_field_count: {table["file_field_count"]}, fields: &[{fields}], '
            f'blockers: &[{blockers}], decode_fields: Some(&[{decode}]) }},'
        )
    return "\n".join(lines + ["];", ""])


def render_json(doc):
    return json.dumps(doc, ensure_ascii=True, indent=2) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--build", type=int, choices=tuple(TARGETS), default=70205)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    global PIN, TARGET_BUILD, CATALOG_ID, FACTS_PATH, RUST_PATH
    config = TARGETS[args.build]
    PIN = config["pin"]
    TARGET_BUILD = (1, 60, 1, args.build)
    CATALOG_ID = config["catalog_id"]
    FACTS_PATH = config["facts_path"]
    RUST_PATH = config["rust_path"]
    try:
        raw_manifest = pinned_source(args.reference)
        doc = document(args.reference, raw_manifest)
        products = ((FACTS_PATH, render_json(doc)), (RUST_PATH, render_rust(doc)))
        if args.check:
            if any(not path.exists() or path.read_text() != content for path, content in products):
                raise CatalogError("generated_catalog_out_of_date")
        else:
            for path, content in products:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
        print(json.dumps({
            "catalog_id": doc["catalog_id"],
            "tables": len(doc["tables"]),
            "build": TARGET_BUILD[-1],
            "checked": args.check,
            "runtime_acceptance_claimed": False,
        }))
        return 0
    except (CatalogError, OSError, UnicodeError, subprocess.SubprocessError) as error:
        message = str(error) if isinstance(error, CatalogError) else "catalog_source_io"
        print(message, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
