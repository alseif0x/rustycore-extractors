# Forever product v42: model-output-expanded material-filtered WMO geometry

Product v42 implements the Linux build-70450 profile
`Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1` for DB2 and available
VMap geometry (component mask **17**). Its PLAN, extract, resume, verify and
verify-links paths are integrated, as are separate model-count and model-output-byte
error codes. This is partial synthetic implementation evidence, **not real-target
acceptance**. The v42 real build-70450 attempt failed at the converted model-output
byte limit; this is not target acceptance. See the
[machine-readable contract](../contracts/forever-product-contract-v42.json).

## Identity and plan shape

V42 binds these identities:

- publication: `Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-model-output-expanded-material-filtered-wmo-coverage-v15`;
- codec/source: `linux70450-model-output-expanded-material-filtered-wmo-v1`;
- domain: `model-output-expanded-material-filtered-wmo-v1`;
- resources: `linux-collision-model-output-expanded-resources-v1`;
- completion: `linux70450-available-model-output-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-model-output-expanded-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_model_output_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-model-output-expanded-material-filtered-wmo-geometry-coverage-v15.bin`.

The synthetic CLI and PLAN-admission tests use the canonical v42 PLAN fixture
`tests/contracts/fixtures/forever-product-v42/all.tsv`. It contains no client data
and is not real extraction evidence. The plan retains build 70450,
`wowdbdefs-70450-v1`, disabled decryption, `partial_encrypted_sections=declared-skip`,
seven downward-only synthetic 1 MiB budgets, Map FDID 1349477, component mask 17,
all-map selection, and no terrain or navigation. These small budgets do not test
the nominal source, model-count, or model-output-byte boundaries.

## Resource contract

The acquisition resource profile is exactly SourceExpandedV1:

- source bytes: **1,073,741,824**;
- model identities: **65,536**;
- reads: **1,000,000**;
- requests and dependency requests: **16,000,000** each;
- raw bytes: **268,435,456**.

The implemented v42 delta raises the model-output inventory capacity coherently
to 65,536 identities across applicable output staging and package inventory
routes. This identity count is distinct from the 4,096 WDT/cache inventory entries
and 4,096 callbacks per map/tile. The acquisition values above match
SourceExpandedV1; model-output-byte, memory, storage, parser, geometry and other
limits are not represented as expanded beyond their existing ceilings.
`DiskWorldV1` remains opt-in and is not implied.

Count exhaustion (`product_selected_geometry_model_count_limit`) and converted
model-output-byte exhaustion (`product_selected_geometry_model_bytes_limit`) have
separate integrated codes, validated by direct Sink unit tests in
`crates/forever-extractor/src/driver/collision_geometry_v22/tests.rs` (not CLI
checks). The sink converts each model with the default VMO conversion first,
then checks the accumulated converted model-output byte budget;
the byte-budget diagnostic is verified at that point. A direct `stage_model`
test accepts 4097 output-model identities in v42 and rejects 4097 in v41. This is
a staging-unit test, not a CLI extraction or a package containing 4097 artifacts.
The configured 65,536 identity capacity is not a proven 65,536/65,537 boundary,
and neither test establishes target sufficiency. The v41 potential 4096 guard
was not proven to have caused its historical failure.

For inspect output compatibility, `model_output_inventory_max` is independent
metadata and is not a required stdout field. The six inspect mutations target
`source_bytes_max`, `models_max` and `reads_max`, removing each field once and
supplying a wrong value once (three fields × two mutations).

## Coverage and omission semantics

RCVGEO1 v15 changes the serialized version and v42 profile identities while
preserving the v14 field layout and five ordered lists:
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`. Each remains ordered, unique, nonzero and limited to
4,096 IDs; the lists remain pairwise disjoint. WMO omission predicates do not
change. V41 v14 and v42 v15 plans, packages and coverage wrappers must reject
cross-version use.

Offline verification covers package-visible artifacts and independently-derived
coverage but cannot reproduce source-side acquisition or omission classification.
WMO absence/omission evidence remains producer-reported. No target omission list
is asserted as observed output.

## Prior v41 evidence and boundary

Real run `v41-source-expanded-wmo-20261027T020021Z` failed after 115 seconds:
inspect exited 0; extract exited 9 in `vmaps` with the generic
`product_selected_geometry_limit`; probes and verifiers exited -1. The report
SHA-256 is
`d3c2bef14736c0d7523ccf3e7469a2db9c18b92665283a2b9db9e93d753cad72` at
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v41-source-expanded-wmo-20261027T020021Z-acceptance.json`.
Network was not attempted, inputs and client metadata were unchanged,
`accepted_prefix=false`, and audit/geometry were null. The diagnostic is absent;
no final CASC counters are available. Source bytes, models and reads at failure are
unknown, and no cause can be inferred from configured ceilings.

The driver guard `self.models.len() >= 4096` at
`crates/forever-extractor/src/driver/collision_geometry_v22.rs:2555` is below the
65,536 identity ceiling and is a potential model-output inventory blocker. It was
not proven to have triggered during the v41 run. Since count and byte failures
share the generic error, the observed failure cannot be attributed to either
condition. V41 failure evidence transfers no acceptance to v42.

## Synthetic validation and open boundaries

Current synthetic evidence is CLI **12/12** for v42 and **11/11** for the v41
regression suite, forever-extractor library **782/782**, plan admission **6/6**,
and current contract tests **6/6**. The CLI suite covers the synthetic inspect,
extract/resume/verify/verify-links lifecycle, deterministic output, v41↔v42
crossing rejection, resealed version/profile/resource attacks and all ten
pairwise omission overlaps. The five coverage omission lists accept 4096 and
reject 4097 through the verifiers; this is not the model-output inventory limit.

The direct `stage_model` test accepts 4097 output-model identities in v42 and
rejects 4097 in v41; it does not perform an extractor CLI run or create a
4097-artifact package. No nominal 65536 identity boundary is established. The
count/byte diagnostics are direct Sink unit tests, not CLI cases. The
converted-byte diagnostic test verifies `product_selected_geometry_model_bytes_limit`
after default VMO conversion and the subsequent accumulated-byte-budget check.
The RCVGEO1 v15 golden bytes match SHA-256
`433d78046326fdc9a2c40eee6b55c139e86ea844695bda1201546fadbba01b41`.

The auditor passed **50 cases / 142 reports**. Real inspect compatibility also
passed with six inspect mutations: missing and wrong values for each of
`source_bytes_max`, `models_max` and `reads_max`. This is inspect-only evidence,
not an extraction acceptance claim. `model_output_inventory_max` is independent
metadata and is not required in stdout.

## Real v42 attempt and v43 follow-up

Real run `v42-model-output-expanded-wmo-20261027T044414Z` failed after 169 seconds.
`inspect` exited 0; `extract` exited 9 with
`product_selected_geometry_model_bytes_limit`; probe, verify and verify-links
did not run (exit -1). Network access was false, inputs and client metadata were
unchanged, no prefix was accepted, and audit/geometry were null. The diagnostic
is absent, so final source/model/read counters are unavailable and remain
unknown. The byte-limit error establishes that the failing model passed v42's
configured model-count check (maximum 65536) before the converted-byte budget
check. It does not establish the total model count or that more than 4096 models
were present.

PLAN `collision-vmo-bytes` was capped at 268435456 bytes (256 MiB), while
`MAX_STAGED_MODEL_BYTES` was 402653184 bytes (384 MiB). The effective
model-output byte budget is the minimum of those separate limits:
`min(268435456, 402653184) = 268435456` bytes (256 MiB). This effective budget is
not the staging ceiling; there is no evidence that the 384 MiB staging ceiling
was reached. Report SHA-256 is
`347548262472d679b08efa89b4a988dfa3e05c9cb5307ff4586fe5af24ccc651`;
software SHA-256 is `4cb7b734d0f92a1a99566f959a5f44ad6ac8ce8ef71fdd73eb0cbb7de09a9c54`;
binary SHA-256 is `c5eebbeec9e2e4fbd556c0b35f8cead8389465dc1f558047e8b29078a8ff8f7e`.
Software qualification recorded 86 suites, 4589 passed, 26 ignored and 558
contracts. The report is at
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v42-model-output-expanded-wmo-20261027T044414Z-acceptance.json`.

Follow-up v43 `ModelBytesExpandedV1` is **design-only; target-not-run**: raise
the `MAX_STAGED_MODEL_BYTES` staging ceiling from 402653184 bytes (384 MiB) to
536870912 bytes (512 MiB), and separately raise PLAN `collision-vmo-bytes` from
268435456 bytes (256 MiB) to 536870912 bytes (512 MiB). The proposed effective
budget is the minimum of the new limits, 536870912 bytes (512 MiB). Preserve
source=1073741824, models=65536, reads=1000000, requests and dependency
requests=16000000 each, raw=268435456, and every other limit; plan coverage v16.
It does not select DiskWorldV1 or establish source/model-output sufficiency,
admission of WMO 462695, full-world coverage or runtime acceptance.

The v42 real attempt failed and was not accepted. Source and inventory sufficiency, the
nominal 1 GiB source boundary, WMO 462695 admission, full-world coverage,
complete geometry and runtime acceptance remain unproven. `full_world_coverage`,
`runtime_claim`, `real_source_1gib_boundary`, `model_output_inventory_sufficiency`,
and `full_extractor_complete` remain false. Windows, MMaps, format parity,
reproducible distribution and publication remain open.

## Filesystem/license and data boundary

Only synthetic fixtures and structured metadata are suitable for this contract.
No client assets, keys, caches, extracted data, or client-derived geometry belong
in Git, CI, reports, or a release. Offline/local acquisition remains the default;
this design does not enable network access.
