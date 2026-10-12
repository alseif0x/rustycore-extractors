# Selected VMO disk preparation

The optional Linux `collision::assemble::staged::models` intermediate consumes
an actual authenticated placement stage and a fresh private spool. Callers
cannot supply selected names, model bytes or association proofs. Selection is
all placement names before ID deduplication plus successful startup TempList
rows, preserving duplicates while converting each unique selected model once.
Acquired unused dependencies are excluded.

Every selected complete RAW is admitted and inspected before the first VMO
conversion/write. Aggregate geometry, M2 bounds and RAW visit policy are checked.
Conversion then reloads one complete verified RAW at a time, applies the existing
strict VMO converter and drops owned geometry/output after its private write.
RCEVMO1 envelopes bind class, scoped source handle, source FDID/content key/extent/
first magic, exact output name, M2 bounds and complete VMO extent. Read callbacks
recheck this association and the complete checked VMO reader, including class
flags and M2 group bounds. All native format bytes remain VMAP_4.E.

This phase retains the actual world/Data/topology and placement owners. Logical
admission includes recorded caller LocalStore residency, compact/model repository,
full placement-stage heap reservation, twice the new archive descriptor ceiling,
fixed guard allowance and actual catalog vector capacity. RAW source, conversion,
reader and declared caller callback allocations are admitted before I/O. New
crate-private completed RAW access also subtracts conversion allowance from the
unchanged collector memory budget; it never removes Data/topology reservations.
The active source-record ceiling remains separate. Growth/reader/component caps
only decrease. These are conservative logical counters, not RSS/allocator or
all CPU work. Caller callback allocations must obey the declared allowance.

Typed causes survive observed changes. Capture/buffer/reader/rejected-value drops
precede final independent RAW-model, placement and selected-VMO archive guards.
Every failure is terminal and returns no usable prepared owner. No final map
artifacts, ancestor proof resolution, publication/resume, cleanup or product
mode are implemented by this phase. The current source/read/disk/reload/same-mount
v1 ceilings remain unchanged. Actual client files are authenticated during the
closed acquisition; later reads authenticate the private complete RAW/VMO files,
not an atomic live-client filesystem snapshot.

## Evidence

Final isolated source passes 21 focused selected-VMO tests and all 28 parent
placement-stage tests in each Linux debug/release profile, CASC library/test
Clippy, four scoped formatting checks, 171 source/state contracts and 104
distribution/sealer checks. All eight traces pass the strict IPC-only audit.
The exact persistent service was observed live through loaded/inactive/dead,
ExecMainCode 1 / Status 0, retaining its InvocationID. Missing-unit/default
properties are not treated as process acceptance.

Only authored toy CASC is executed, against the existing resident Rust complete
assembler. Comparisons include selected exact names, complete VMO bytes, geometry
and output counts, model BIH work, logical RAW passes and source facts. Fixtures
cover all pre-ID-dedup names, duplicate list rows, omitted textures/NoCollision,
unused WMO doodad dependencies, startup-cached versus later-liquid WMO, inherited
placements and empty-parent barriers. Negative cases cover phase/component
budgets, aggregate geometry before any write, nonempty archives and changes in
each private RAW/placement/VMO repository. Typed callback errors, uninvoked
capture destruction and rejected-success-value destruction retain independent
final guards. Callback-memory refusal has zero RAW reload delta.

| Evidence | SHA256 |
| --- | --- |
| Final source freeze, 988 entries / five owned paths | `bb7e06698a71e297c484ad3c0f2ee360171841491db554bdacfcb0a271a5c0e0` |
| Eight-command final check report | `1269c5b5c7e8d4bb5acedc6e0bda2e127444fa266ad2ae3c6117b088aee65068` |
| Root saved-evidence acceptance | `70ef9a918e75232f60fd496768f885d9d2ca5cb927b566faba1767b14c97644e` |

Private retained evidence is under
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate/collision-vmo-stage-20261012T0243Z`.
The accepted checker is `checks-attempt6.py`, with
`source-freeze-attempt6.json`. The publication qualifies this documentation after
acceptance; the four Rust paths are byte-identical to that freeze. The parent
placement source separately passed 491 complete collision tests per profile;
this final increment does not claim a new full-collision/workspace/Wine run.
No new original C++ or real-client comparison or distribution bundle is claimed.

Attempt 1 stopped at Clippy for an unused import. Attempt 2 passed Clippy and
formatting, then retained 11 passing / seven failed fixture-opening cases.
Directory-chain descriptor pressure under the service's 1024-FD soft limit was
the working diagnosis; no observed EMFILE claim is made. Attempt 3's nested-trace
diagnostic was rejected by ptrace before tests, ended there, and is retained
without acceptance. The final fixture bounds only its five-repository lifetime
through a test-only mutex; the service limit, production guards and all
assertions remain unchanged. Attempt 4 passes all earlier 18 tests and 28 parent
cases per profile plus static checks; its source/report/binaries remain retained.
Three additional drop/admission regressions then exposed an invalid test-only
assert_eq on a type without Eq/Debug in attempt 5. The final matches assertion
preserves that production type, and attempt 6 reruns the entire final plan.
All failed reports and build targets remain retained without relabeling.

The preceding placement commit `8596820` independently passes hosted
[Rust CI 38197185256](https://github.com/alseif0x/rustycore-extractors/actions/runs/38197185256)
and [C++ CI 38197185258](https://github.com/alseif0x/rustycore-extractors/actions/runs/38197185258).
Windows debug executes 2,116 passed, zero failed and 20 ignored tests across
41 targets; retained full Windows log SHA256
`6c6d8ace7c56cb9e858fb946b4078d5733766367c8af508a52f5a2f449bcc858`.
That hosted result precedes this Linux VMO increment; hosted downloads are
separate from local IPC-only evidence.

Final per-map artifacts and first-present-ancestor proofs remain a separate disk
phase. The product driver still uses resident assembly. Scalable explicit
source/read/disk/reload and cross-volume policies, publication/resume, mandatory
encrypted real inputs, complete real geometry/runtime and reproducible Linux /
Windows distribution remain completion gates. The full extractor goal is active.
