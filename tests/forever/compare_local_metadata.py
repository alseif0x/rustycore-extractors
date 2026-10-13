"""Read-only comparison of Rust reports with independently unpacked local headers.

This checks wire metadata, not record decoding or C++ runtime parity. No client
paths/bytes are emitted. It is separate from synthetic unittest discovery.
"""
import argparse
import json
from pathlib import Path
import struct
import subprocess

HEADER_FIELDS = (
    "record_count", "field_count", "record_size", "string_table_size",
    "table_hash", "layout_hash", "min_id", "max_id", "locale_mask",
    "flags", "index_field", "total_field_count", "packed_data_offset",
    "parent_lookup_count", "column_meta_size", "common_data_size",
    "pallet_data_size", "section_count",
)
SECTION_FIELDS = (
    "file_offset", "record_count", "string_table_size", "catalog_data_offset",
    "id_table_size", "parent_lookup_data_size", "catalog_data_count", "copy_table_count",
)


def compare(binary, path):
    with Path(path).open("rb") as file:
        header = file.read(204)
        if len(header) != 204 or header[:8] != b"WDC5\x05\0\0\0":
            raise ValueError("header")
        # Source packed DB2Header: 9 uint32 + uint16 + int16 + 7 uint32.
        expected = dict(zip(HEADER_FIELDS, struct.unpack_from("<9IHh7I", header, 136)))
        if expected["section_count"] > 1024:
            raise ValueError("limit")
        sections = []
        for _ in range(expected["section_count"]):
            section = file.read(40)
            tact, *words = struct.unpack("<Q8I", section)
            sections.append({"tact_declared": tact != 0, **dict(zip(SECTION_FIELDS, words))})
    result = subprocess.run([binary, "--file", str(path), "--json"],
                            check=False, capture_output=True, text=True)
    if result.returncode or result.stderr:
        raise ValueError("command")
    report = json.loads(result.stdout)
    expected.update(magic="WDC5", version=5)
    if report["header"] != expected or report["sections"] != sections:
        raise ValueError("metadata")
    if report["record_data_validated"] or report["client_build_verified"]:
        raise ValueError("scope")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--file", action="append", required=True)
    args = parser.parse_args()
    checked = 0
    try:
        for path in args.file:
            compare(args.binary, path)
            checked += 1
    except (OSError, ValueError, KeyError, struct.error):
        print(json.dumps({"status": "failed", "files_checked": checked,
                          "scope": "metadata_only", "paths_redacted": True}))
        return 1
    print(json.dumps({"status": "passed", "files_checked": checked,
                      "scope": "metadata_only", "paths_redacted": True}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
