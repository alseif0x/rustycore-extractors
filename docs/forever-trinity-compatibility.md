# Trinity-compatible available-map verification

`verify-trinity-compat` certifies the partial omission behavior already present
in accepted v51 terrain and v57 navigation packages. It does not reextract,
rewrite or manufacture resources and never claims full-world completion.

```
forever-extractor verify-trinity-compat \
  --terrain-package V51_PACKAGE --terrain-plan V51_PLAN \
  --navigation-package V57_PACKAGE --navigation-plan V57_PLAN
```

The command first performs complete v51 `verify-links` and complete v57
`verify-links` against the same source package and PLAN. It then requires the
unavailable-WDT set in terrain to equal the ROOT-absent set in navigation. Each
omitted map must have a 4,108-byte tilelist containing 4,096 absent coordinates
and no present coordinates, no MMap global, no tile and zero native fallback
counters. Different, missing or additional omissions reject.

The result is `compatible-partial`, matching TrinityCore's behavior of skipping
an unavailable WDT and writing an empty tilelist while retaining a successful
process result. Unlike an opaque success code, this profile records the exact
omissions and their scope. `full_world_complete=false`, `runtime_accepted=false`
and `full_extractor_complete=false` remain mandatory.

The real v51/v57 acceptance passed offline in 567.564 seconds with no network
attempt: 71 maps, 69 planned and two ROOT-absent. The redacted report is
[acceptance/trinity-compat-v1-20261101.json](../reports/acceptance/trinity-compat-v1-20261101.json),
SHA-256 `d3e013f2d14e77e05d61a1ab0decdb2eb87ffe6f2d433aa4a91709cdb5ff6c20`.
The timed-out debug attempt and the initial source-PLAN argument error were
retained and corrected before this accepted run. The final closeout passes 101
suites, 4,870 tests, zero failures, 31 ignores and 616 contracts; log SHA-256 is
`763c55a9b037f01ce729496efa320a101899049cc45a2eda8995402a4aeb33e3`.
No publication is authorized.
