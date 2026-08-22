# Notices and distribution policy

RustyCore Extractors is licensed as a combined work under
GPL-3.0-or-later. See `LICENSE`. Files copied or derived from the pinned
TrinityCoreLegacy source remain GPL-2.0-or-later and retain their original
copyright and license headers. New or materially changed derived files must
carry a prominent `Modified by the RustyCore Extractors project, YYYY-MM-DD`
notice without removing upstream attribution.

This repository does not contain and must never distribute Blizzard client
files or client-derived DBC/DB2, maps, VMaps, MMaps, models, cameras, game
tables, CASC caches, encryption-key downloads, or intermediate extraction
output. Users provide their own lawfully obtained client installation and
generate data locally. Hash-only and structured reports must not contain
reconstructable client content.

## Private work and distribution

Running, studying and modifying the software privately does not by itself
require publication. Distribution of source or binaries does trigger the
applicable license conditions. In particular, every binary release must:

1. accompany the binary with the complete corresponding source, or a GPLv3
   section 6 compliant source-delivery method, for the exact released build;
2. include build scripts, interface definition files, dependency pins,
   modifications and installation information when the GPL requires them;
3. include `LICENSE`, this notice, `THIRD_PARTY_NOTICES.md`, `AUTHORS`, the
   machine-readable inventory and every applicable text in `LICENSES/`;
4. preserve copyright, warranty, license and modification notices;
5. provide recipients the GPL rights to use, inspect, modify and redistribute
   the covered source; an NDA or private-repository rule must not remove those
   rights from a recipient of a distributed copy; and
6. contain no Blizzard client data or output derived from it.

Release evidence must identify the source commit, dependency versions,
checksums, build command and corresponding-source archive. A release is
blocked if its closure differs from `licensing/inventory.json`, if a required
notice is absent, or if any license is unknown, `NOASSERTION`, GPL-2.0-only,
or incompatible with GPL-3.0-or-later.

This is an engineering compliance record, not legal advice. Material license
ambiguity must be escalated to qualified counsel before distribution.
