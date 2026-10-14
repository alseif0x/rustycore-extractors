# Authored camera manifest vectors

GPL-3.0-or-later. Synthetic WDC5 vectors authored 2026-10-14.
Direct IDs [12,11,12] and copy(2,1) deliberately preserve original dependency11
after its row is overwritten. Source fields/five-column wire follow the
existing camera toy writer; target table hash is C2CEFDF9, legacy is zero.
Camera payloads are `not parsed as M2`, NUL, ff, little-endian u32 ID.
All payload/inventory hashes are independently calculated in Python.
These are not client data or historical real acceptance.
