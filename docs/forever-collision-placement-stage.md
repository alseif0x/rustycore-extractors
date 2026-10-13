# Authenticated placement disk stage

The optional Linux `collision::assemble::staged` phase consumes authenticated
GameObject startup, fresh WDT/cache inventories and a fresh private placement
repository. Callers cannot provide placements or forge a completed stage.
This is an intermediate library phase; final VMO/map/proof assembly and the
product command are still resident and remain implementation gates.

## Contract

Every complete world callback is retained, including empty callbacks and all
pre-deduplication model names and duplicate IDs. Private `RCEPLC1` envelopes bind
map, original map, global/tile coordinates, replay flag, callback sequence,
record count and complete extent. Sorted descriptors reload one complete map in
global then x/y order. Parent flags must match the context; inherited callbacks
bind to the actual effective immediate parent retained by the closed world.
This is extractor-cache provenance, not final first-present-ancestor proof.

One full stage heap reservation is partitioned before world I/O. The collector
ceiling only decreases and survives consuming finish, so later RAW reloads cannot
reuse the stage allowance. WDT/cache ceilings, model/placement/source scratch and
stage descriptors remain disjoint. Twice the repository descriptor ceiling and
512 KiB guard allowance are reserved; entry growth overlap, complete map output,
largest envelope reload and the declared callback allowance are admitted before
reload. Actual Vec capacities are checked. Borrowed callback bytes remain with
the authenticated collector. These are conservative logical budgets, not RSS,
allocator overhead or all CPU work. The private borrowed Sink itself allocates
nothing; its external stage owner retains the disjoint reservation throughout.

Failed access or callback poisons the phase. Owned map buffers, callback captures
and rejected return values drop before final independent placement/model guards;
typed callback causes survive observed changes. Complete source authentication
belongs to the world acquisition; after finish the source facts and verified
private RAW repository survive, not an atomic client snapshot. Protected root
anchors check object identity, not unread directory membership/content.

The existing same-mount spool profile, 4096 records, 512 MiB disk, 16384 reloads,
1 GiB cumulative rereads and 4096 source-call limits are not enlarged. No public
ordinal escapes. No publication, resume, cleanup, product output or real-client
acceptance is implied. Final selected-model union, one-model VMO conversion,
one-map assembly, retained ancestor/proof state and product integration remain.

## Evidence

The isolated final Rust source passes 28 focused and 491 complete collision
cases in each Linux debug/release profile, CASC library/test Clippy, four scoped
formatting checks, 171 source/state contracts and 104 distribution/sealer checks.
All eight traces pass the strict IPC-only audit. The exact persistent service
was observed live through loaded/inactive/dead with ExecMainCode 1 / Status 0;
no not-found/default service properties are used as acceptance.

Only authored toy CASC inputs run. Comparisons cover complete placement bytes,
contexts, IDs, table source facts, startup lists and topology against the
unchanged resident Rust world. A separate supplied-input case retains 24 map
bodies exceeding twice its 3 MiB stage heap, while complete one-map reloads stay
inside that budget. This does not establish full-world or allocator/RSS bounds.
No new original C++ or real-client comparison is claimed by this phase.

Retained local evidence (private outputs/binaries are outside Git):

| Evidence | SHA256 |
| --- | --- |
| Source freeze, 985 entries / five owned paths | `086c55b8a3abd14d8328481434e3a61d9426649ab35ab5548ae7329a5db36310` |
| Eight-command check report | `fd0d79456d8ca1ae71b8542e9fcafa5577b569a59cc4a9f6bbb1ff82e5260f2f` |
| Root saved-evidence acceptance | `8dcf3209e2f3c20b1c1e707746f46c56a924f3f84b8681e463518089b6f9d96c` |

The candidate directory is
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate/collision-placement-stage-20261012T0206Z`.
The accepted checker is `checks-attempt6.py`; source freeze is
`source-freeze-attempt6.json`. The checked source retains the pre-acceptance
status paragraph; this publication updates documentation only, with the four
Rust files byte-identical to the accepted freeze. No full workspace/Wine run or
new distribution bundle was executed locally for this increment.

Attempt 1 retained 21 passing cases and two fixture failures (a prematurely
removed toy source and an extra expectation omitting a materialized Map copy).
Attempt 3 retained 26 passing cases and the one expected-array correction that
the earlier scripted edit missed. These do not alter production behavior.
Attempts 2 and 4 were checker-construction failures before any command; their
reports/observations and all build targets remain retained. Attempt 5 passed 27 focused and 490 complete collision cases per profile, then
failed Clippy for duplicate error/guard storage. The final source compacts these
fields without heap allocation or loss of typed causes/guards, adds an explicit
size/cause test, and reruns the full plan in attempt 6. No failed result is
relabeled accepted.

The preceding canonical `3629e8d` independently passes hosted
[Rust CI 38193903259](https://github.com/alseif0x/rustycore-extractors/actions/runs/38193903259)
and [C++ CI 38193903235](https://github.com/alseif0x/rustycore-extractors/actions/runs/38193903235).
The Rust run includes Windows debug: 2,116 passed, zero failed, 20 ignored,
41 targets; retained full Windows log SHA256
`afb1ca4c68e1a618f06d8c1066da6e77eecef52fcc3743a003ce9099f9f679b0`.
Hosted downloads are separate from local IPC-only traces and precede this
source increment. General NTFS, Windows release/product, real geometry/runtime,
mandatory encrypted inputs and full extraction remain open gates.
