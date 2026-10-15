# Functional Linux extraction

Issue: [#72](https://github.com/alseif0x/rustycore-extractors/issues/72).

## Delivery scope

On 2026-10-15 the operator selected stable, usable Linux extraction matching
the Forever TrinityCore tools as the next milestone. Broader coverage follows;
Windows delivery remains deferred. The current implementation is a source
candidate, not an accepted extraction CLI or release.

The upstream setup guide reports a working Linux server using data extracted
on Windows, with navigation disabled for that test. Its README declares build
70338; our current development input is 70374. Comparing upstream commits
`8c5325b37dfa155450a9e87f213965f6e6c00afe` and
`8a864593bb4d566153e89058b33793752ff81c57` finds identical blobs, modes and
types for all 125 relevant extractor, CascLib and DB2 paths. Upstream's DB2
reader permits zero-filled encrypted spans and skips sections without a key;
Map consumers omit the resulting unavailable records. These are source facts
and an upstream operational report, not our real-input acceptance.

References: [upstream setup](https://github.com/advocaite/TrinityCore/blob/8a864593bb4d566153e89058b33793752ff81c57/doc/LinuxServerSetup.md),
[DB2 source](https://github.com/advocaite/TrinityCore/blob/8a864593bb4d566153e89058b33793752ff81c57/src/tools/extractor_common/DB2CascFileSource.cpp).
The earlier review remains in `forever-completion.md`.

## Separate availability capability

Existing complete-file/CKey APIs, strict profiles and failed batches retain
their contracts. The new capability must keep unavailable spans inaccessible
and retain explicit coverage. It cannot convert a partial result into
`VerifiedFile` or claim complete plaintext authentication.

The initial CASC profile is Linux, build 70374 and disabled keys. A complete
read uses the existing strict path. An encrypted-only failure may use a
separate encoded-source check: the independently selected full EKey binds
the BLTE header, or the whole headerless object, and every frame hash, extent
and EOF must validate. N/Z ranges decode exactly; encrypted ranges remain
inaccessible. Different authenticated partial encodings are ambiguous and
reject. Candidate counts include the preceding strict pass; decoded extents
include unavailable spans. These are logical budgets, not RSS or total CPU
measurements.

The pure DB2 interface borrows only fully available ranges and proves no
cryptographic identity itself. Explicit current-catalog Map and
GameObjectDisplayInfo projections return a distinct snapshot with section
coverage. The initial disabled-key policy admits zero-Tact sections and omits
all nonzero-Tact sections without reading their payloads. Missing bytes in an
admitted section or its required string dependency are fatal. Declared rows
and copies in omitted sections still count against limits. Physical order,
direct IDs and raw copy facts remain distinct; terrain copy materialization
uses admitted direct sources only. GODI does not replay copies.

## Acceptance sequence

1. Qualify the new source and authored fixtures, then compare supplied positive
   inputs against the identified original C++ readers. Native malformed-input
   behavior is not a safety oracle.
2. Probe real available Map/GODI inputs under an explicit new admission and
   record actual admitted and omitted coverage. No private rows, key names,
   keys or asset correlations belong in Git or CI.
3. Prove complete requested terrain tiles using available Map rows and complete
   liquid/WDT/ADT inputs. Missing requested maps, required ancestors or present
   tiles reject that selection.
4. Integrate a versioned product CLI with sealed coverage evidence and matching
   verification/resume rules. Add collision and navigation after their own
   selected-input and output comparisons.

Global source coverage and requested output coverage are separate. A complete
selected terrain map may use a globally incomplete Map table when its required
row and all terrain inputs are available. That does not establish complete
global DB2 data, collision models, all-world extraction or runtime acceptance.
Existing v11 and v8 product promises are not widened by this candidate.
