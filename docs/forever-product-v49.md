# Forever product v49: GameObject ROOT-absent and encrypted-unavailable material-filtered WMO geometry

V49 is a separately versioned offline Linux build-70450 product for DB2 and
available-map geometry (component mask **17**). It carries forward the v48
missing-ROOT outcome and adds a disjoint outcome for direct GODI FDIDs whose raw
ROOT contains the FDID but whose local encoded payload is unavailable solely
because its frame is encrypted. Coverage uses RCVGEO1 version 22. Synthetic plan
and CLI lifecycle/verifier checks pass. A real run was subsequently accepted for
this versioned scope only; it does not establish full-world, complete-VMap or
runtime acceptance.

## Product identity and scope

V49 uses these distinct identities:

- publication: `Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1`;
- resource profile: `GameObjectRootAndEncryptedUnavailableExpandedV1`,
  `linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1`;
- PLAN: `linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22`;
- codec/source: `linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1`;
- domain: `gameobject-root-encrypted-unavailable-material-filtered-wmo-v1`;
- completion: `linux70450-available-gameobject-root-encrypted-unavailable-material-filtered-wmo-collision-v1`;
- verification: `linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1`;
- coverage artifact:
  `vmaps/available-gameobject-root-encrypted-unavailable-material-filtered-wmo-geometry-coverage-v22.bin`;
- GODI scope: `available-fdid-direct-root-absent-or-encrypted-unavailable-v1`;
- model scope: `placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1`.

The canonical synthetic PLAN is
`tests/contracts/fixtures/forever-product-v49/all.tsv`. It has 31 lines,
build 70450, component mask 17 and the ordered DB2 entries
`GameObjectDisplayInfo=1266277` followed by `Map=1349477`. The test fixture
contains no client data.

## Outcome rules and limits

The profile admits at most **4,096 direct nonzero GODI candidate rows**. It also
bounds each canonical outcome FDID list at 4,096. The lists are sorted, unique,
nonzero and pairwise disjoint. Row counters count outcomes per direct row; they
are not required to equal the deduplicated list lengths.

- **Missing ROOT:** a raw ROOT lookup finds no row for the FDID. The model is not
  published and the FDID is added to the missing-ROOT list.
- **Encrypted unavailable:** ROOT has a row, but local acquisition sees only
  encrypted-frame failures. This is not evidence of missing ROOT; the model is
  not published and the FDID is added to a separate encrypted-unavailable list.
- Any ROOT-present missing body for a non-encrypted reason, malformed payload,
  other acquisition error, or limit violation remains fatal. The tool performs
  no network request, key lookup or retry to fetch unavailable payloads.

The keyless encrypted-unavailable path requires the selected EKey in full:
framed BLTE authenticates its complete descriptor table and independently checks
all frame MD5s/extents, while headerless BLTE checks the full encoded object. It
checks exact framing and the decoded-size sum, validates the encrypted wrapper
(including the admitted Salsa20 selector and nonempty ciphertext), and decodes
observable N/Z frames before classifying encrypted frames as unavailable. This
cannot authenticate plaintext inside E frames or claim a complete CKey/plaintext
for a source with encrypted holes. Truncated wrappers, unsupported ciphers,
corrupt later frames and mixed encrypted/available frames have synthetic
regressions.

Both absence classifications are producer-reported. Offline verification binds
the outcome rows and FDID lists consistently to authenticated sealed GODI rows,
checks list canonicality/disjointness and rejects references to missing or
unavailable models in artifacts and placements. It does **not** independently
re-read the client's raw ROOT or prove the producer's encrypted-frame diagnosis.
Map/GODI source rows and the map domain are independently derived from sealed
DB2; bounds and transformed raw geometry remain producer-reported.

V49 keeps v48's resource and disk ceilings unchanged: 2 GiB raw and output,
256 MiB each for WDT/cache/placement/map and 512 MiB VMO. Every override may only
reduce its admitted ceiling. The new v22 codec adds the encrypted-unavailable
row count and FDID list to the existing missing-ROOT declarations. V47, v48 and
v49 plans/packages/coverage remain cross-rejected by their distinct identities.

The startup candidate ceiling is **4,096 nonzero direct rows**, not 4,096 unique
FDIDs: repeated requests count separately even when the unavailable-FDID list is
deduplicated. The v49 profile supplies enough read/request capacity for the table
read plus 4,096 candidate attempts; 4,097 is rejected. Each canonical outcome
list remains independently bounded at 4,096 unique FDIDs.

## Validation and scoped real acceptance

The v49 plan admission target passed **5/5** tests. The v49 synthetic CLI target
passed **10/10**, including inspect, extract, verify, verify-links, resume,
deterministic package bytes, bidirectional v47/v48 profile rejection, partial GODI
resume, simultaneous missing-ROOT and ROOT-present encrypted-unavailable rows,
repeated candidate rows at the 4,096 limit and rejection at 4,097, redacted failure
diagnostics, and rejection of crossed/noncanonical outcome lists. The v47 and v48
CLI regressions passed **7/7** and **11/11**, respectively. The final recorded
offline `forever-extractor --all-targets` run passed **1,310/1,310** tests across
49 targets. Contract tests passed **3/3**. The separate full workspace closeout
is recorded in the progress log; its broader software qualification is not real
extractor acceptance.

The real run `v49-gameobject-root-encrypted-wmo-20261029T033410Z` was accepted
for the v49 scope. Its redacted report is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v49-gameobject-root-encrypted-wmo-20261029T033410Z-acceptance.json`
(SHA-256 `04e63482a957c5361c3fa966d4b4efa3d296ed53b4d490bf48b9f9ea30449045`);
the audit SHA-256 is
`c3c9e2a81fe5048e369f5c66726cf27b73f46a8a59a1d27e61f16194844bb4a2`. Inspect,
extract, missing-M2 probe, verify and verify-links all exited 0 in 1,112 seconds;
network was not attempted, inputs and client metadata were unchanged, and fresh
output matched the audit. It published 12,050 files (2 DB2, 12,048 VMap),
493,057,703 bytes; the package records 71 active maps (69 present, two ROOT-absent)
and 73 materialized rows from 81 declared direct rows. It has 1,158,879 placements
(90,392,562 bytes) and 5,801 placement-model artifacts. The sealed Map contains
10,006 available bytes out of 10,581 declared across two partial frames (575 bytes);
its encoded key is authenticated but the full CKey is not verified. GODI dispositions
are producer-reported: 20 missing-ROOT rows and one encrypted-unavailable row
(FDID 7385605). Missing-root FDIDs are recorded in the structured contract.

This scoped result does **not** accept a prefix, complete VMap component, full-world
coverage, LiquidObject, MMaps, RustyCore runtime, full extractor, Windows support,
distribution or publication. Decryption was disabled and no key/network fetch was
performed. `real_extraction_accepted=true` describes only this versioned scope;
`full_real_target_acceptance=false` and `full_extractor_complete=false` remain.

Only synthetic fixtures and structured metadata belong in Git. No client assets,
keys, caches, extracted output or client-derived geometry may be added to the
repository, CI or releases. See the [machine-readable v49 contract](../contracts/forever-product-contract-v49.json).
