# Forever product v46: AssemblyExpanded material-filtered WMO geometry

V46 defines a separately versioned Linux build-70450 DB2 and available-VMap
geometry profile (component mask **17**). It preserves the v45 acquisition,
model, and staged-placement limits and adds a distinct **268,435,456-byte (256
MiB) assembled-domain limit**. The coverage artifact uses RCVGEO1 v19. This is
an implementation contract whose real acceptance is scoped to available
assembly-expanded placement geometry, not full extraction. The separate software
qualification passed 97 workspace suites (4,689 passed, 0 failed, 26 ignored),
the forever-extractor library (811/811), v46 CLI (16/16), v46 PLAN (4/4), CASC
resource identity (1/1), and repository contracts (579). Formatting, Clippy,
diff, Windows GNU, and release-build checks passed. A later real target run is
accepted only for its recorded available-geometry scope; it does not test the
assembled-domain boundary or establish capacity sufficiency. The machine-readable
contract records the historical software qualification and final real acceptance
separately, together with the unconfirmed v45 failure.

## Product identity

V46 binds these identities:

- publication: `Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-assembly-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-assembly-expanded-material-filtered-wmo-coverage-v19`;
- codec/source: `linux70450-assembly-expanded-material-filtered-wmo-v1`;
- domain: `assembly-expanded-material-filtered-wmo-v1`;
- resources: `linux-collision-assembly-expanded-resources-v1`;
- completion: `linux70450-available-assembly-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-assembly-expanded-material-filtered-wmo-verification-v1`;
- coverage artifact:
  `vmaps/available-assembly-expanded-material-filtered-wmo-geometry-coverage-v19.bin`.

The canonical v46 PLAN fixture is `tests/contracts/fixtures/forever-product-v46/all.tsv`.
CLI tests derive a temporary v46 plan from the v45 fixture, changing only the
version and profile identities. PLAN budgets and client/package fixtures used by
tests are synthetic; they contain no client data and do not constitute real
extraction evidence.

## Resource contract

`AssemblyExpandedV1` keeps the v45 resource capacities and changes the assembled
domain guard independently:

- assembled-domain bytes: **268,435,456** (256 MiB), capped by the configured
  PLAN `collision-map-bytes` budget;
- staged placements: **268,435,456** bytes (256 MiB), capped by the PLAN
  `collision-placement-bytes` budget;
- per-map placement bytes: **67,108,864** (64 MiB);
- source bytes: **2,147,483,648** (2 GiB);
- model identities: **65,536**;
- reads: **1,000,000**;
- request and dependency calls: **16,000,000** each;
- raw bytes: **268,435,456** (256 MiB);
- staged model output and PLAN VMO: **536,870,912** bytes (512 MiB).

The assembled-domain check accounts for proof bytes, tree/index/tile outputs,
and retained parent payload data while assembling all admitted maps. Its
separate diagnostic is `product_selected_geometry_assembled_domain_limit`;
placement aggregate, per-map, and domain checks retain their own diagnostics.
The value is a configured software limit, not evidence that a real 256 MiB
domain fits available memory or that this limit is sufficient for the target.
`DiskWorldV1` remains explicit and is not implied.

## Coverage and omissions

RCVGEO1 v19 has the v18 field sequence with its own layout version and v46
identities. It keeps the ordered lists `missing_m2`, `missing_wmo`, `empty_wmo`,
`unselected_wmo`, and `material_filtered_wmo`; each is ordered, unique,
nonzero, bounded to 4,096 IDs, and all five lists are pairwise disjoint. V45 and
v46 PLAN, package, and coverage identities are separate, and each verifier
requires its matching coverage version/profile.

Coverage independently authenticates the admitted Map/VMap domain, while WMO
omissions remain producer-reported. The profile does not claim full-world
coverage, complete VMap geometry, runtime acceptance, or format parity.
Global GameObjects, LiquidObject, terrain, and MMaps remain outside this
product's scope.

## Relationship to v45 and acceptance status

The retained v45 real attempt failed in `vmaps` with the generic
`product_selected_geometry_limit` error. Its cause was not confirmed. The
128 MiB `MAX_ASSEMBLED_DOMAIN_BYTES` value was a hypothesis only; it does not
prove the source of the failure or that 256 MiB is sufficient. V46 is a separate
profile with a versioned PLAN/coverage identity and an independent 256 MiB
guard; it does not change v45 retroactively.

The final real run `v46-assembly-expanded-wmo-20261027T180034Z` passed in 1,107
seconds: inspect, extract, missing-M2 probe, verify, and verify-links all exited 0.
It records `target_status=accepted`, `real_target_attempted=true`, and
`real_extraction_accepted=true` only for
`available_geometry_with_assembly_expanded_material_filtered_wmo`; the fresh
extraction matches the promoted audit. Network was not attempted, and client
metadata and inputs were unchanged. No prefix was accepted and
`full_extractor_complete=false`.

The accepted package has 11,237 files / 473,186,807 bytes; 71 active maps (69
present, two root-absent), 1,158,879 placements / 90,392,562 bytes, and 4,990
model artifacts. It records 81 declared direct rows and 73 direct/materialized
rows. Its exact omissions are `missing_m2=[192262,194853,243074,243154,243158,243166,243169,243172,603672,970793]`, `missing_wmo=[]`, `empty_wmo=[]`, `unselected_wmo=[]`, and `material_filtered_wmo=[462695,7679500]`. Inventory, manifest, coverage, and plan SHA-256 values are retained in the structured contract. Report SHA-256 is
`a7732a62448a321399994de309d723683cda7a21d0d3df696af303fe4380b4a6`; audit SHA-256 is
`3fcbc9b8bb82f3b08dcc3e5945ba2a3f41b8b6dd3e7afacc6d71311b9fc542fb`.

This acceptance does not establish a real 256 MiB assembled-domain boundary or
source/model/placement/domain capacity sufficiency. WMO 462695 admission, GODI /
global GameObjects, LiquidObject, full-world coverage, complete VMap geometry,
MMaps, RustyCore runtime acceptance, distribution, and release authorization
remain unaccepted. Software qualification of the final run records 97 suites
(4,690 passed, 0 failed, 26 ignored) and 579 repository contracts; the prior
software-only release inspect remains historical evidence for its own binary.
See the [structured contract](../contracts/forever-product-contract-v46.json)
for hashes, exact package counters, prior retained attempts, and remaining
boundaries.

Only synthetic fixtures and structured metadata belong in this repository. No
client assets, keys, caches, extracted data, or client-derived geometry belong
in Git, CI, reports, or releases. This contract does not enable network access.
