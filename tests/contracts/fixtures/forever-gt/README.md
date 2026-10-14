# Authored GT manifest vectors

GPL-3.0-or-later. Synthetic vectors authored 2026-10-14, not client data.
Each numeric payload is `id`, N tab-separated `v` headers, LF, `x`, N
tab-separated `-0` values, LF. Raw-only payload is the five bytes
`72 61 77 00 ff`. Public identities/order come from GT catalog v1.
SHA-256/inventory fields are independently calculated from those literals.
Rust roundtrip/extraction tests and formal JSON Schema tests share these vectors.
Legacy vectors freeze v1/v2/v3 ordering and bytes for build 70205.
