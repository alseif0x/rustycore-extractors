# Private typed WDT and own-tile cache inventory

`linux-private-wdt-cache-inventory-v1` is an optional Linux library primitive
with scoped synthetic acceptance. It consumes an empty private `Spool`, stores complete
supplied WDT and normalized own-tile dir_bin records, and retains only sorted
compact key descriptors. It does not acquire inputs or replace the current
world runner's resident WDT/cache collections yet.

WDT keys bind declared map/FDID pairs. Tile keys bind map and both 0..63
coordinates. Private envelopes bind kind, map, source/cell, record count and
complete payload length; ordinal repository handles never leave the owner.
WDTs are admitted and reloaded through the existing complete planner/parser.
Caches use the existing complete dir_bin reader and `Cached` normalization:
only parent bit2 is cleared on storage and restored on replay to another map.
Order, IDs, names and every other serialized bit survive. Empty own-tile caches
are present records, distinct from genuine absence. Map/FDID acquisition and
OWN-tile provenance are declared by the caller, not authenticated by this API.

All errors poison the inventory; no partial completion token is exposed. Typed
callback causes survive independent repository/source failures, with observed
change taking diagnostic priority. A refused callback capture is destroyed
before the final checks; returned callback values are destroyed before returning
an error. Repository source/protected-path and immutable-file guards run again
before success. Checks are metadata plus per-reload content SHA256/EOF, without
an atomic filesystem snapshot or publisher authentication claim.

Downward ceilings are 4,096 entries, 512 MiB logical heap and 256 MiB admitted
active storage, plus the original parser/cache and private repository ceilings.
The ledger reserves twice the repository descriptor limit to cover old/new
allocation overlap, 512 KiB for bounded guard/path scratch, and actual typed
vector capacities/growth overlap. Complete reload, parser and callback/replay
allowances are admitted together before I/O. Parser admission can deliberately
exceed actual allocation. Borrowed inputs and successful returned/callback
storage are caller-owned and separately charged by integration. These are
conservative logical reservations, not exact allocator/RSS or all CPU work.
Source-root directory anchoring validates object identity; it does not authenticate
unread sibling contents or directory membership. Declared key files retain full
metadata checks without key reads. Actual acquired-source content/LocalStore
verification belongs to the later world integration.
The underlying same-mount, explicit protected-input, private path, immutable
record, disk/reload counters and no-cleanup policy remain unchanged.

This step has no product manifests, resume, publication, native C++ execution,
real acquisition, scalable full-map or runtime acceptance. Shared-world
integration, versioned full-world read/spool policies, selected VMO/map/proof
staging and product integration remain required. The existing 4,096 cumulative
source-read limit and same-mount spool boundary remain separate gates.

## Retained acceptance — 2026-10-12

The exact supplied-input implementation passes 25 focused tests and 412 complete
collision tests per Linux debug/release profile. CASC library/test Clippy, three
Rust formatting checks, 171 source/state contracts and 104 distribution/sealer
checks also pass. The eight retained traces admit only local IPC; no real client,
original native comparison, Windows execution or product bundle is added here.

The retained `collision-inventory-spool-20261012T0049Z` attempt-3 source freeze is
SHA256 `839c8849ad84d9b3959b6b8249e4087ffa76ac7e15e14f2a3029164cb68a8723`;
its completed report is
`0b66fbaedec1dacb0b1709feb4d6c921d1885131a9b9f5409007c0b062ce7225`,
and root evidence replay is
`6c5758dbe8e28ead1bb6e34a23bac2f18ea9219e60f2f1c196a853749c92129b`.
The observer retained the exact loaded service through successful terminal exit;
later unit collection is not used as exit evidence. The first two failed fixture
attempts, logs and their exact reconstructed owned sources remain preserved.
This acceptance section was added after the frozen checks; the three Rust files
are imported byte-for-byte. Source acquisition and the world runner's existing
resident inventories remain unchanged.
