# Forever product v50: complete LiquidObject source evidence

V50 is a separately versioned, offline Linux build-70450 profile for DB2 and
available-map geometry (component mask **17**). It adds a complete physical
`LiquidObject` source declaration and sealed source facts to the v49 profile.
Its geometry basis and geometry resource limits remain v49's; this version does
not add liquid or terrain geometry. Synthetic validation passed, and the scoped
real target was accepted for this versioned source-and-available-map scope only.
This is not full real-target acceptance, full-world coverage or a complete VMap
component.

## Product identity and scope

V50 uses distinct PLAN, coverage, LiquidObject codec/domain and verification
identities:

- publication, source and PLAN:
  `linux70450-liquid-object-complete-source-vmap-product-v1`;
- coverage: `linux70450-liquid-object-complete-source-coverage-v23`;
- LiquidObject source codec: `linux70450-liquid-object-complete-source-v1`;
- LiquidObject domain: `liquid-object-complete-source-v1`;
- verification: `linux70450-liquid-object-complete-source-verification-v1`;
- resource profile: `linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1`,
  based on `GameObjectRootAndEncryptedUnavailableExpandedV1`;
- source DB2: `dbc/esES/LiquidObject.db2` (FDID 1308058);
- sealed evidence: `vmaps/liquid-object-complete-source-v1.bin`.

The canonical synthetic PLAN is
`tests/contracts/fixtures/forever-product-v50/all.tsv`. It has 32 lines,
build 70450, component mask 17 and ordered DB2 entries
`GameObjectDisplayInfo=1266277`, `Map=1349477` and `LiquidObject=1308058`.
The fixture contains no client data.

The source capability is limited to complete physical LiquidObject rows and
ordered copy records. Signed 16-bit `LiquidTypeID` values preserve their bits
with `GetUInt16` semantics. Direct records and duplicate IDs retain physical
input order. Copy replay preserves order and overwrite behavior, creates a
missing source with default fields before assigning its destination, and
inserts self-copies at most once. Partial, malformed or trailing source data is
fatal; no partial LiquidObject declaration is admitted. The 16 MiB source and
300,000 materialized-row bounds are independent LiquidObject limits. No network
or key fetch is performed, and no consumer output is produced.

## Geometry basis and limits

V50 retains the existing v49 available-map ROOT-absent/encrypted-unavailable
material-filtered-WMO geometry behavior. The geometry resource profile is
shared, not replaced: its geometry limits do not change versus v49. The
coverage codec advances to RCVGEO1 layout 22 with the v50 coverage identity,
while PLAN, coverage and verification identities reject cross-version v49/v50
packages. Synthetic parity verifies that shared geometry artifacts match v49
byte-for-byte apart from versioned coverage and v50-only LiquidObject
DB2/evidence artifacts.

The ten emitted geometry limits remain explicit: source 2 GiB; model 512 MiB;
placement 256 MiB (64 MiB per map); 65,536 models; 1,000,000 reads;
16,000,000 request calls and dependency requests; and 256 MiB each for raw and
assembled-domain bytes. PLAN storage budgets can only reduce their admitted
ceilings. The additional LiquidObject source-byte and materialized-row bounds
are 16 MiB and 300,000 respectively.

Offline verification authenticates the sealed LiquidObject DB2 bytes, derives
complete-source facts again, re-encodes the 112-byte evidence artifact and
compares it with the published artifact. The geometry verifier retains the
v49 available-map scope with v50 coverage identity. These checks do not turn
source facts into a liquid geometry output or a terrain consumer result.

## Validation and scoped real acceptance

The focused synthetic CLI suite covers inspect, extract, verify, verify-links,
resume and deterministic package contents; exact numeric capabilities; ordered
direct rows and copy replay; signed-16 bit preservation; missing/partial/
trailing source rejection; v49/v50 profile isolation; and byte parity for the
shared geometry. PLAN admission and the v49 CLI regression are separately
scoped checks. Their results and the final evidence are recorded in the
[machine-readable contract](../contracts/forever-product-contract-v50.json) and
[progress log](forever-progress-log.md).

The final scoped real run `v50-liquid-object-complete-source-20261029T135738Z`
was accepted in 1,088 seconds. Inspect, extract, probe, verify and verify-links
all exited 0; network was not attempted, client metadata and inputs were
unchanged, and fresh extraction matched the promoted audit. The package has
12,052 files / 493,068,435 bytes (3 DB2 and 12,048 VMap), 71 active maps (69
present and two ROOT-absent), and complete LiquidObject source facts: 57 direct
rows, 1,213 ordered copy records and 1,270 materialized rows, with no partial
source declaration. The existing v49 GODI outcome policy and available-map
geometry scope remain; this does not claim all-world output or expand its
omission evidence.

The report SHA-256 is
`f368b1e715f86502c85d80675668cce3fbcf680336a752f1259990de13f38bde`, audit
SHA-256 `9286430073704beef9a7b1c272f6dc79baf3d697b94e5fc94aced00ff822d8c6`,
and binary SHA-256
`295e780066351503f4321334cb17c3550d5979d254058367f8b9d3c6bade77d9`. Earlier
failed audit/closeout attempts are not the basis for this final accepted run.
The separate final software closeout passed with 94 suites (4,774 passed, 0
failed, 26 ignored), 592 contracts, formatting, Clippy and release build; its
log path and SHA-256 are in the structured contract. Software closeout is not
extraction acceptance.

`real_target_attempted=true` and `real_extraction_accepted=true` describe only
this v50 scope. `accepted_prefix=false`, `full_real_target_acceptance=false` and
`full_extractor_complete=false` remain. No full-world coverage, complete VMap
component, LiquidObject geometry or terrain output, MMaps, RustyCore runtime
acceptance, Windows support, distribution, release or publication is established. Only synthetic
fixtures and structured metadata belong in Git; no client assets, keys, caches
or extraction outputs may be committed.

## Recorded checks

The synthetic v50 PLAN/CLI cases and v49 regression remain recorded separately
in the contract. The final contract test and common-execution contract were
rerun after documenting the accepted result. The final software closeout passed
94 suites (4,774 passed, 0 failed, 26 ignored), 592 contracts, formatting,
Clippy and release build. Its report is software qualification evidence, not the
real extraction acceptance record. Earlier audit/closeout attempts had failures;
they remain distinct and do not override the final passing closeout or accepted
real run. Exact closeout path and SHA-256 are in the machine-readable contract.

See also the [v50 contract](../contracts/forever-product-contract-v50.json),
the [v49 geometry basis](forever-product-v49.md), and the
[completion gates](forever-completion.md).
