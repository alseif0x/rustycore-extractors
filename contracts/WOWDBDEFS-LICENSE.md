# WoWDBDefs data attribution

Modification notice — 2026-10-18: this attribution now covers the separately
pinned build 70338 catalog alongside the existing build 70205 catalog, and adds
the separately pinned build 70374, 70425, and 70450 catalogs.

The generated schema facts below are adapted from the `definitions/` data and
`manifest.json` in [wowdev/WoWDBDefs](https://github.com/wowdev/WoWDBDefs):

| Client build | Facts and generated Rust source | Original source commit |
| --- | --- | --- |
| 70205 | `contracts/wowdbdefs-70205-v1.json`, `crates/forever-db2/src/records/wowdbdefs_70205_generated.rs` | `674636522d51c010f250ad1b5fe549ef6bc8eb0a` |
| 70338 | `contracts/wowdbdefs-70338-v1.json`, `crates/forever-db2/src/records/wowdbdefs_70338_generated.rs` | `d6f50d4f351193e84226aa62a36faa0f58528afa` |
| 70374 | `contracts/wowdbdefs-70374-v1.json`, `crates/forever-db2/src/records/wowdbdefs_70374_generated.rs` | `464a25b3a7c02b0c74726be7a53a0b0ccf1a2d10` |
| 70425 | `contracts/wowdbdefs-70425-v1.json`, `crates/forever-db2/src/records/wowdbdefs_70425_generated.rs` | `996e53f029c0213b223aecb94d754f6b4b9efb49` |
| 70450 | `contracts/wowdbdefs-70450-v1.json`, `crates/forever-db2/src/records/wowdbdefs_70450_generated.rs` | `bfebcbc5181f47b954e7886862f92c5577dbbe4a` |

Attribution: WoWDBDefs contributors. The source data and these adapted facts are
licensed under [Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/).
Each generated catalog records its own source commit and manifest digest. Schema
fields are normalized for this extractor; no client files or client record
values are included. The original generator is retained as
`tools/generate_wowdbdefs_catalog.py`.

The offline distribution also retains the complete CC-BY-SA-4.0 license text at
`toolchain-notices/licenses/CC-BY-SA-4.0.txt`. Retaining this existing notice text
does not add a compiled registry dependency or transfer either catalog's build
or acquisition acceptance to the other.
