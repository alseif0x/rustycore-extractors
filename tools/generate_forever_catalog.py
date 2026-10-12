#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate factual schema declarations from one verified local Git object.

No upstream implementation or source file is copied into this repository.
Only table/field facts are emitted, with attribution and immutable provenance.
The parser accepts the pinned declaration grammar in full or fails closed.
It never reads a client, uses the network, or infers runtime acceptance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

PIN = "02245dcd245e7433e524577656177723d3e4992e"
SOURCE_PATH = "src/server/game/DataStores/DB2Metadata.h"
SOURCE_BLOB = "d06b97bad72ca2aecad3987ce9c348ac6055e0db"
SOURCE_SHA256 = "897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def"
TABLE_COUNT = 1126
ROOT = Path(__file__).resolve().parents[1]
FACTS_PATH = ROOT / "contracts/forever-metadata-v1.json"
RUST_PATH = ROOT / "crates/forever-db2/src/records/catalog_generated.rs"
TYPES = {
    "FT_STRING": "String", "FT_STRING_NOT_LOCALIZED": "NonLocalizedString",
    "FT_BYTE": "Byte", "FT_SHORT": "Short", "FT_INT": "Int",
    "FT_LONG": "Long", "FT_FLOAT": "Float",
}
BLOCKERS = {
    "signed_64_bit": "Signed64Bit",
    "string_array": "StringArray",
    "unsigned_appended_parent": "UnsignedAppendedParent",
}
NUM = r"(?:0x[0-9a-fA-F]+|[0-9]+)"
FIELD = re.compile(
    r"\{\s*\.Type\s*=\s*(FT_[A-Z_]+)\s*,\s*\.ArraySize\s*=\s*(\d+)\s*,"
    r"\s*\.IsSigned\s*=\s*(true|false)\s*\}\s*,"
)
TABLE = re.compile(
    r"struct\s+([A-Za-z_][A-Za-z0-9_]*)Meta\s*\{\s*"
    r"static\s+constexpr\s+DB2MetaField\s+Fields\[(\d+)\]\s*=\s*\{(.*?)\}\s*;\s*"
    r"static\s+constexpr\s+DB2Meta\s+Instance\s*=\s*\{(.*?)\}\s*;\s*\}\s*;",
    re.S,
)
KEYS = ("FileDataId", "IndexField", "ParentIndexField", "FieldCount", "FileFieldCount", "LayoutHash")


class CatalogError(ValueError):
    pass


def _integer(value):
    return int(value, 16 if value.lstrip("-").startswith("0x") else 10)


def blockers(table):
    # Every declaration admitted by the strict pinned grammar now has a value
    # representation. File variants and compression remain decoder gates.
    # Keep this explicit hook for a future, separately versioned source pin.
    return []


def parse_metadata(source):
    """Strict declaration parser, also usable with small synthetic inputs."""
    text = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)
    prefix = re.match(
        r'\s*#ifndef\s+TRINITYCORE_DB2_METADATA_H\s*'
        r'#define\s+TRINITYCORE_DB2_METADATA_H\s*#include\s+"DB2Meta.h"\s*', text,
    )
    suffix = re.search(r"#endif\s*$", text)
    if prefix is None or suffix is None or suffix.start() < prefix.end():
        raise CatalogError("invalid_metadata_envelope")
    body = text[prefix.end():suffix.start()]
    tables, names, file_ids = [], set(), set()
    position = 0
    for match in TABLE.finditer(body):
        if body[position:match.start()].strip():
            raise CatalogError("unparsed_metadata_remainder")
        position = match.end()
        name, declared, field_text, instance = match.groups()
        fields, field_position = [], 0
        for field in FIELD.finditer(field_text):
            if field_text[field_position:field.start()].strip():
                raise CatalogError("unparsed_field_remainder")
            field_position = field.end()
            kind, count, signed = field.groups()
            if kind not in TYPES or not 1 <= int(count) <= 255:
                raise CatalogError("invalid_field_type_or_array")
            fields.append({"type": kind, "array_size": int(count), "signed": signed == "true"})
        if field_text[field_position:].strip():
            raise CatalogError("unparsed_field_remainder")
        pattern = r"\s*" + "".join(
            rf"\.{key}\s*=\s*({'-?' if key in ('IndexField', 'ParentIndexField') else ''}{NUM})\s*,\s*"
            for key in KEYS
        ) + r"\.Fields\s*=\s*Fields\s*"
        values = re.fullmatch(pattern, instance)
        if values is None:
            raise CatalogError("invalid_instance_declaration")
        file_id, index, parent, total, physical, layout = map(_integer, values.groups())
        if name in names or file_id in file_ids:
            raise CatalogError("duplicate_table_name_or_id")
        if not (0 < file_id <= 0xFFFFFFFF and 0 <= layout <= 0xFFFFFFFF):
            raise CatalogError("invalid_table_identity")
        if len(fields) != int(declared) or len(fields) != total or not 0 < physical <= total <= 0x7FFF:
            raise CatalogError("invalid_field_counts")
        if not (-1 <= index < physical and -1 <= parent < total):
            raise CatalogError("invalid_field_index")
        if total != physical and not (total == physical + 1 and parent == physical):
            raise CatalogError("unsupported_extra_field_shape")
        if index >= 0:
            field = fields[index]
            if field["type"] not in ("FT_BYTE", "FT_SHORT", "FT_INT") or field["signed"] or field["array_size"] != 1:
                raise CatalogError("invalid_inline_index_shape")
        if parent >= 0:
            field = fields[parent]
            if field["type"] not in ("FT_BYTE", "FT_SHORT", "FT_INT") or field["array_size"] != 1:
                raise CatalogError("invalid_parent_shape")
        table = {
            "name": name, "file_data_id": file_id, "layout_hash": layout,
            "index": index, "parent_index": parent, "field_count": total,
            "file_field_count": physical, "fields": fields,
        }
        table["decode_blockers"] = blockers(table)
        # Native GetRecordSize and FillParentLookup store every appended
        # BYTE/SHORT/INT parent in 32 bits, regardless of its wire declaration.
        table["appended_parent_storage_bits"] = 32 if parent >= physical else None
        tables.append(table)
        names.add(name)
        file_ids.add(file_id)
    if body[position:].strip() or not tables:
        raise CatalogError("unparsed_metadata_remainder")
    return sorted(tables, key=lambda table: table["name"])


def read_pinned(reference):
    selector = PIN + ":" + SOURCE_PATH
    blob = subprocess.check_output(["git", "-C", str(reference), "rev-parse", selector], stderr=subprocess.DEVNULL).decode().strip()
    raw = subprocess.check_output(["git", "-C", str(reference), "show", selector], stderr=subprocess.DEVNULL)
    if blob != SOURCE_BLOB or hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise CatalogError("pinned_source_mismatch")
    return raw


def document(raw):
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise CatalogError("pinned_source_mismatch")
    tables = parse_metadata(raw.decode("utf-8"))
    if len(tables) != TABLE_COUNT:
        raise CatalogError("pinned_table_count_mismatch")
    return {
        "catalog_version": 1,
        "catalog_id": "pinned-metadata-v1",
        "source": {"repository": "TrinityCore/TrinityCore", "commit": PIN,
                   "path": SOURCE_PATH, "git_blob": SOURCE_BLOB, "sha256": SOURCE_SHA256,
                   "bytes": len(raw), "license": "GPL-2.0-or-later",
                   "attribution": "TrinityCore contributors; see upstream AUTHORS"},
        "scope": "Metadata facts only; shape support is not extraction parity or runtime acceptance.",
        "tables": tables,
    }


def _kind(field):
    kind, count = field["type"], field["array_size"]
    if kind.startswith("FT_STRING"):
        return "Kind::String" if count == 1 else f"Kind::StringArray({count})"
    if kind == "FT_FLOAT":
        return f"Kind::Float({count})"
    width = {"FT_BYTE": 8, "FT_SHORT": 16, "FT_INT": 32, "FT_LONG": 64}[kind]
    return f"Kind::{'Signed' if field['signed'] else 'Unsigned'}({width}, {count})"


def render_rust(doc):
    lines = [
        "// SPDX-License-Identifier: GPL-3.0-or-later",
        "// Generated by tools/generate_forever_catalog.py; do not edit by hand.",
        "// Factual declarations from TrinityCore contributors, GPL-2.0-or-later.",
        f"// Source pin {PIN}; blob {SOURCE_BLOB}.",
        f"// Source SHA-256 {SOURCE_SHA256}.",
        "// No upstream implementation or client-derived data is included.",
        "use super::super::schema::Kind;",
        "use super::{FieldMetadata, FieldType, TableMetadata};",
        "#[rustfmt::skip]",
        "pub(super) static TABLES: &[TableMetadata] = &[",
    ]
    for table in doc["tables"]:
        fields = ", ".join(
            f"FieldMetadata {{ kind: FieldType::{TYPES[f['type']]}, array_size: {f['array_size']}, signed: {str(f['signed']).lower()} }}"
            for f in table["fields"]
        )
        denied = ", ".join("CatalogBlocker::" + BLOCKERS[reason] for reason in table["decode_blockers"])
        decoded = "None" if denied else "Some(&[" + ", ".join(
            _kind(field) for field in table["fields"][:table["file_field_count"]]
        ) + "])"
        lines.append(
            f'    TableMetadata {{ name: "{table["name"]}", file_data_id: {table["file_data_id"]}, '
            f'table_hash: None, other_layout_hashes: &[], '
            f'layout_hash: 0x{table["layout_hash"]:08X}, index: {table["index"]}, '
            f'parent_index: {table["parent_index"]}, field_count: {table["field_count"]}, '
            f'file_field_count: {table["file_field_count"]}, fields: &[{fields}], '
            f'blockers: &[{denied}], decode_fields: {decoded} }},'
        )
    return "\n".join(lines + ["];", ""])


def render_json(doc):
    return json.dumps(doc, ensure_ascii=True, indent=2) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="compare deterministic outputs without writing")
    args = parser.parse_args(argv)
    try:
        doc = document(read_pinned(args.reference))
        products = ((FACTS_PATH, render_json(doc)), (RUST_PATH, render_rust(doc)))
        if args.check:
            if any(not path.exists() or path.read_text() != content for path, content in products):
                raise CatalogError("generated_catalog_out_of_date")
        else:
            for path, content in products:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
        print(json.dumps({"catalog_id": doc["catalog_id"], "tables": len(doc["tables"]),
                          "shape_compatible": sum(not table["decode_blockers"] for table in doc["tables"]),
                          "checked": args.check, "runtime_acceptance_claimed": False}))
        return 0
    except (CatalogError, OSError, UnicodeError, subprocess.SubprocessError) as error:
        print(str(error) if isinstance(error, CatalogError) else "catalog_source_io", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
