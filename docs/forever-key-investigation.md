# Forever key investigation and REA procedure

This is an operational record of the local investigation, not extraction
acceptance. Keep key values, lookup identifiers, client bytes, cache payloads,
memory contexts and packet captures outside Git and outside model context.
Only disclose reviewed aggregate reports. Existing build and component gates
continue to apply.

## Checkpoint: 2026-10-19, build 70450

The selected product is `wow_classic_beta`, version `1.60.1.70450`, locale
`esES`. The observed executable SHA-256 is
`e87aaae6b4204cced22c05975d5378a7ea49e5560c171905c450fbc848e933c1`.
The three complete-file probes concern Map (FDID 1349477),
GameObjectDisplayInfo (FDID 1266277), and a model (FDID 7385605).
They encounter two distinct unavailable key names at their first failing
encrypted frames. This does not inventory every key needed by later frames
or by a complete world extraction.

### Results already established locally

| Source or method | Observation | Coverage limit |
| --- | --- | --- |
| Complete public TACTKeys list | 19,705 pairs; neither requested name present | Exact pinned snapshot, not future updates |
| Static TactKey / TactKeyLookup | 69 / 202 rows; 69 joined keys | Exact build 70450 |
| Current hotfix cache | 14 distinct valid direct key observations, all public | Cache produced by the operator's actual session |
| Combined static and current cache keys | 83 keys; complete-file probes remain blocked | No recovered requested key |
| All BroadcastText cache records | 92 records; 29 nonempty, all with the 46-byte base tail and no recognized key extension | Exact current cache; no server-wide delivery claim |
| ROOT variants | Map has ten locale rows; esES is encrypted and enUS encoding is not locally available | Only esES/enUS groups were read |
| GODI and selected model variants | Each has one ROOT row and one encoding, still encrypted | Exact installed ROOT and selected FDIDs |
| Latest inspected fork key catalog | Neither requested name present | Pinned fork source, not proof about server operators' private keys |
| Fourteen historical minidumps | No requested name found in declared captured memory | Older builds and limited captured ranges; no current heap coverage |

The current cache SHA-256 is
`c02fc57d9bfc26ae50e45ed0c046f39d88a1f2dc20d92f7bf8383273de020758`:
3,334,666 bytes, 32,917 strictly framed HTFX v9 records, declared build 70450.
Source identity and content were checked before and after the read.

## Public sources and pins

- [TACTKeys distribution methods](https://github.com/wowdev/TACTKeys#updates-wow).
  The inspected `WoW.txt` commit is
  `2394fc00248074b9624ebb6749934f7a41a5f09e`; blob
  `1ff43f9dcb6a22b342edfe8fa676d70e328e23aa`;
  SHA-256 `e5695072ed09e4e17462204da60cda7f22a71ef69d6cf2481f563b4d533b6d6a`.
- [wow.tools.local KeyController](https://github.com/Marlamin/wow.tools.local/blob/5b2f85db477bd42205fde912ed72b3adadda218a/Controllers/KeyController.cs).
  Also inspect `Utils/DBCacheParser.cs` at the same commit.
- [Fork CascDecrypt catalog](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/dep/CascLib/src/CascDecrypt.cpp).
- [Fork world-packet encryption](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/src/common/Cryptography/Authentication/WorldPacketCrypt.cpp).
- [Fork hotfix handling](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/src/server/game/Handlers/HotfixHandler.cpp).

Use the complete authenticated public dictionary for membership comparisons.
The smaller 83-key client ring is not a substitute for that comparison.
Select matching public entries privately for a bounded retry; do not send a
real lookup identifier to public search services.

## Local retained laboratory

These workstation paths are external evidence, not distributable product
inputs. Keep their private directories restricted to the operator.

- Initial private probe and frozen diagnostic source:
  `/home/joe/.local/share/rustycore-key-investigation-70450-20261018-m5ru_zy_`.
- Current-cache, public-source, and ROOT-variant investigation:
  `/home/joe/.local/share/rustycore-key-retry-70450-20261019-tvaa55c2`.
- REA, all-record cache audit, runtime preparation and tool-update receipts:
  `/home/joe/.local/share/rustycore-rea-keyflow-70450-20261019-mhqytzbc`.

The last directory contains `reports/` with redacted results, `private/` with
local-only material, and authored diagnostic scripts. Preserve earlier runs
and failures. Record script and binary hashes before another execution.
Do not copy a laboratory wholesale into this repository or an issue.

## Repeating cache acquisition correctly

1. Record dirty files and assign ownership before working alongside OpenCode.
   Use an isolated source copy and external build/output directories for probes.
2. Resolve the actual client root, selected product, locale, version and build.
   Do not invent a local remapping of the reference-server paths.
3. Hash the executable, `.build.info` metadata and selected cache locally.
   Check that the cache header declares the same admitted build.
4. Read static TactKey and TactKeyLookup through an already admitted local CASC
   reader. Join row IDs locally; never infer a key from its lookup alone.
5. For HTFX v9, validate the 44-byte file header and each 32-byte record header
   plus its declared payload. Reject out-of-bounds, unexpected-version or
   changing sources. Keep row payloads local.
6. Audit both the latest-record policy and all retained nonempty records.
   Do not silently equate latest-record selection with upstream's complete
   record iteration: an older observation might otherwise be omitted.
7. Parse two terminated CStrings before interpreting a BroadcastText tail.
   The admitted 70450 catalog derives a 46-byte base tail. A known optional
   TactKey extension is a table tag, 8-byte lookup and 16-byte key: 28 bytes.
   Require the tag at the end of the justified base layout and a complete
   extension. The upstream 44/45-byte special cases do not establish 70450's
   layout, and a coincidental tag inside a 46-byte base is not an extension.
8. Reject conflicting key observations. Recheck source identity and hash after
   reading. Candidate extraction and complete-file verification are separate.

The authored `audit_all_cache_records.py` implements the bounded all-record
70450 audit. Its report is `reports/all-cache-records.json`. It found no new
requested key. The previous `broadcast_keys.py` remains retained with its
latest-record and old-tail-heuristic limitations; do not describe it as the
full current-format audit.

## Using REA without repeating an unsuitable analysis

Start from the installed skill and the connected server's actual tool schemas.
Inspect the existing session and retained evidence before opening another
target. Bind the exact executable hash and provider; keep one MCP session open
between related queries so it can reuse its Ghidra database.

The 70450 attempt used REA 6.4.0 and Ghidra 12.1.4. Opening the target succeeded.
The first string query started automatic analysis, but that analysis was stopped
after about eleven minutes without completion or a handler finding. This was
an intentional stop, not a timeout, a successful decompilation or a key result.
Logs and the owned-process stop receipt are retained locally.

Independent byte measurements found:

- `.text`: 63,511,040 bytes, entropy 7.999825 bits per byte.
- The 64 KiB window beginning at the entry RVA: entropy 7.996655.
- The single searched TactKey table-tag occurrence is in `.rdata`, not code.
- Exact string starts, containing-string starts and relocated pointer cells
  were recorded as seeds, not mislabeled as code references or handlers.

High entropy is consistent with packed, encrypted or otherwise transformed
code. It does not identify a protector or demonstrate runtime unpacking.
Do not keep expanding disassembly claims over these disk bytes without first
checking the runtime image. Complementary linear-disassembly seed searches
were negative and are explicitly not REA Evidence.

The installed Codex registration already allowed 30 minutes:
`REA_GHIDRA_STARTUP_TIMEOUT_MS=1800000`, client tool timeout 1860 seconds,
`GHIDRA_HEADLESS_MAXMEM=4G`. REA's unconfigured default was 330000 ms. A new
environment setting does not extend an already running provider request.

CLI `--snapshot` retains matching Evidence responses; it does not preserve a
live Ghidra project for a different query. Repeating independent CLI xref
commands can therefore repeat import and automatic analysis. Use the active
MCP session for related native queries and export completed evidence before
closing. Close only resources owned by this investigation.

## Runtime comparison: current status and next step

A single owned user service was started with the pinned existing
GE-Proton11-9 runtime, updates disabled, and the explicit 70450 executable.
Its launch receipt is `reports/runtime-launch.json`; its log stays private.
No credentials were supplied, no packet capture was made and no successful
login or world entry is claimed for this launch.

`memory_key_observer.py` is an authored local diagnostic, not product code.
It scans at most 512 MiB over twenty seconds in private writable,
non-executable mappings of a selected existing process. It uses 83 known
name/key controls and requires three distinct controls to agree on a relative
key offset before proposing a candidate. Candidates still require actual CASC
decryption and complete content verification. It never writes target memory.

The first execution refused `client_mapping_missing` before scanning: the
Wine process had no file mapping matching the executable's exact path,
device and inode. This failure must remain a failure. The preferred PE base
was a reserved inaccessible range; reading its header did not match the file.
A bounded search of 641 readable private mapping starts found no exact COFF
header match. Consequently **no runtime image identity, unpacking, memory-key
candidate or key recovery has been established**.

Next, diagnose the owned launch and identify the real loaded Windows module.
Retain the process start identity and source hash. Establish an equally strong,
explicit image identity method for Wine before changing the observer's guard;
do not remove it merely to obtain scan output. Compare actual loaded code with
the exact file before deciding which artifact REA should analyze. Once a
handler or key-registration location is established, observe that path during
a normal authorized game session, keeping values local.

Packet capture is secondary: fork source shows world traffic encryption and
hotfix optional-data serialization. A raw capture does not by itself supply
decrypted hotfix payloads or TACT keys. Distinguish transport session keys
from the content keys needed by CASC. If runtime observation is needed, select
the game-side decrypted hotfix/key-registration boundary rather than assuming
the entire authentication exchange will reveal the missing values.

## Tool and client version changes

1. Preserve current tool versions, hashes, configuration backups and dirty
   files. Inspect a source checkout before any `pull`; never reset local work.
2. Verify the official `morluto/rea` source and the published npm package's
   repository, exact version and integrity. Keep a fresh source checkout
   outside this Git repository. A source checkout and the installed CLI are
   separate installations.
3. Update only the authorized REA package. The pinned 2026-10-19 update plan
   selects release 6.7.0, tag commit
   `27dc42d10249f0410904a75c0997842eb03b931c`, and npm installation with lifecycle
   scripts disabled. Do not silently update Ghidra, Java or client settings.
4. Check the fresh CLI version. An already connected MCP process can still run
   the old package; verify server identity after reconnection instead of
   assuming a file update reloaded it. Review any skill/setup maintenance plan
   and preserve timeouts and unrelated configuration.
5. For a new client build, create a new run directory and source identity
   record. Re-evaluate cache framing, table layout, ROOT selection and runtime
   addresses. Never reuse an old address or transfer an old build's acceptance
   by changing a version number.
6. Recovered candidates must pass the real encrypted-frame decoder and complete
   file authentication. A partial/available-range read, ignored encrypted
   section or zero-filled native result is not a recovered key or complete
   extraction.

### Executed REA update

The fresh CLI reports 6.7.0; the shared installed skill was updated from version
37 / 170 tools to version 42 / 207 tools using a skill-only setup plan and its
matching apply operation. The complete previous skill tree and configuration
were backed up privately. The Codex configuration remained byte-identical;
Ghidra and Java were not updated. The old connected MCP process was not
reconnected, so subsequent new-version work uses the fresh installed CLI.

The official source checkout is `/mnt/nvme2/datos/tools/rea/source`.
Its explicit `git pull --ff-only` advanced it to
`a80575c73652d8b43d7340654a48bd20a4eb966d`, with no local changes.
This development commit and the published 6.7.0 package are distinct pins.
Future pulls must start with a dirty-state check and retain the resulting
commit in a new receipt.

REA 6.7.0 exposes optional Frida instrumentation and recorded packet-capture
analysis. Neither establishes that the underlying native binding or capture
decoder is available. The first Frida enumeration failed because installing
REA with lifecycle scripts disabled had not materialized its optional binding.
The exact official Frida 17.23.2 Linux x64 / N-API 8 prebuild was then acquired
and checked against the release asset's SHA-256:
`9fa8f668ce91eb03713e8564de4f8fbde232233ee91d729c16773fde11dbae95`.
Only `build/frida_binding.node` was installed in REA's external optional
dependency. Its SHA-256 is
`5b2db5ae57c058ad0765e627d9d9c9c98e4e90c09746c6f727b7a80769eed7a5`.
No lifecycle scripts or unrelated package updates were executed. A fresh
`rea list-frida-processes --format json` succeeded and listed the owned process.

Use `rea instrument-with-frida --help` and the installed
`docs/frida-instrumentation.md` before instrumentation. Preserve full local
Evidence with `--full-output --format json`, keep script inputs and reports
outside Git, and disclose only the script's reviewed aggregate messages.
The authored `frida_runtime_inventory.js` is limited to process/module/range
inventory and observing `mprotect` calls intersecting the requested image
range. It neither authenticates a Windows module from that range alone nor
scans for keys. Authenticate the owned PID/start identity before attaching,
detach the script's own listener, and retain cleanup and coverage outcomes.

The first actual REA/Frida attach to the owned direct-launch Wine process did
not produce that inventory. It returned `capability_unavailable` with the
provider's alternative diagnosis that the process refused to load the agent
or terminated during injection. The process was absent afterwards and the
owned service was inactive. The controller's post-observation identity check
therefore also failed. Full private CLI output and a redacted failure receipt
are retained. Do not infer the cause of process exit, a successful attach,
successful login, runtime image recovery or any key result from this attempt.
Continue by diagnosing the ordinary Battle.net launch rather than repeating
the same attach to a stale PID.


### Windows Frida server inside the existing Wine container

The changed method uses the Windows backend rather than attaching the Linux
backend to Wine's host process. The official Frida 17.23.2 Windows x86_64
server archive is pinned by SHA-256
`230f62f528a3a58c1a765df14aa3108bb019d7d5c65f93030633e46d32f700a5`;
the extracted PE executable is pinned by
`a7b0e82c4efdd12f579914369b83b955dd49dbccb0170ec7bd4730c78cc19749`.
The archive, executable and generated authentication token remain private.
The server listens only on `127.0.0.1:34699` with token authentication.

Container identity matters as well as prefix identity. The initial Battle.net
service must enable `UMU_CONTAINER_NSENTER=1`, so its launcher service exists
for later helpers. Enabling this only for a helper does not create a missing
initial service. The owned launcher was closed and restarted normally with
that option; the subsequent helper log explicitly recorded re-entry into the
existing container. Preserve the already selected Proton version and disable
UMU runtime updates for this bounded investigation. Do not create a second
prefix or terminate unrelated Wine processes.

The retained launch receipts identify the owned user units
`rustycore-keyflow-battlenet-reentry-20261019` and
`rustycore-keyflow-wine-frida-20261019`. Keep the server's token in its existing
0600 file and read it inside the local controller; never print it or place it
in a checked-in command. Stop only these owned units when their work ends.

A fresh REA CLI remote process enumeration succeeded. Those PIDs are Windows
PIDs, not Linux `/proc` PIDs. A subsequent `instrument-with-frida spawn` of the
locally authored `win_ui.exe observe` helper executed a read-only Frida script
and reported `Process.platform == windows`, architecture x64. This establishes
that this local Windows Frida route can execute that script; it does not yet
establish successful instrumentation of WoW or key recovery.

The first exact-title `FindWindowW` call returned a valid but hidden Battle.net
window. The follow-up `EnumWindows` script checked all top-level windows locally
and disclosed only exact allowed titles. It found two Battle.net windows with
the same Windows PID: one hidden 640×480 window and a visible 1440×895 window.
All API error counters were zero. Do not use the first title match as proof
that the launcher is hidden or unauthenticated. Select the visible window and
revalidate its PID/title/geometry before any explicitly declared UI action.
Window handles and PIDs are per-run observations and must not be reused after
a restart.

Retained evidence includes `reports/wine-frida-process-inventory-00.json`,
`reports/windows-ui-frida-00.json` and
`reports/windows-ui-enumerate-00.json` under the current external laboratory.
Raw REA envelopes and screenshot/OCR data remain private. The authored
`run_windows_probe.py` captures private output, records script hashes and
emits only recognized reviewed aggregate payloads.

Sources for the route and its limits are the
[Frida Wine issue](https://github.com/frida/frida/issues/3339), the
[REA Frida guide](https://github.com/morluto/rea/blob/a80575c73652d8b43d7340654a48bd20a4eb966d/docs/frida-instrumentation.md)
and the locally inspected installed UMU launcher source. A community report
motivated the experiment; the successful local probes above provide its
actual acceptance scope here.


### Launch argument diagnosis and current runtime boundary

Do not copy the earlier direct-launch arguments into a new recipe. The actual
client-owned modal rejected the `launch` argument. Its text was obtained through
read-only control queries from the separate helper, saved privately and reduced
to a reviewed diagnostic. A process named `WowB.exe` in that error state was not
evidence of a normally initialized game.

A subsequent launch used the exact pinned executable, the selected product as
working directory, the existing UMU container and **no game arguments**. The
following Windows enumeration observed a visible window whose exact title was
`World of Warcraft`. The older failed-argument attempt continued after its
modal was closed, creating a duplicate. Only the owned older window was then
closed through its freshly validated HWND and Windows PID; the new window was
retained. Stopping the outer UMU user unit had not by itself retired the target
inside the already running container. Always re-enumerate the actual Windows
processes/windows before claiming cleanup or starting another instance.

The new Windows process also rejected the remote Frida agent load, while the
separate helper continued to accept and execute scripts. The game process
remained alive afterwards. This narrows the observed limitation to target
instrumentation in this configuration; it does not establish an anti-cheat
cause, successful attach or a recovered runtime image. Do not repeat an
identical failed injection.

A bounded read-only Linux header probe checked 512 MiB of approximately 2.69
GiB of eligible private readable mappings and found no exact 256-byte source
DOS header in that scanned prefix. The process and source hash stayed stable.
The report explicitly retains the omitted 2.19 GiB and does not claim that the
image is absent elsewhere. Shared mappings and mapping starts require their
own declared coverage. The original key observer's file-backed mapping guard
remains unchanged; a failed guard is not permission to scan a different PID.

Next diagnostics use the already functioning helper to query the exact client
process through read-only Windows process APIs, or a separately declared
mapping-start probe. Neither method authorizes raw bytes in model context.
Any recovered image identity must precede code/key analysis, and candidate
key values still need full local CASC authentication.


### Authenticated runtime image and executable-page sampling

A subsequent mapping-start probe included both readable private and shared
mappings. It examined 2,766 starts / 11,427,840 bytes within a 32 MiB / ten-second
budget, with eight inaccessible reads explicitly retained. It found two
structurally exact source header matches: a writable shared alias and a higher
image view. The source hash and process start identity stayed unchanged.
This was a mapping-start inventory, not an exhaustive heap search.

The stronger cross-process confirmation used REA instrumentation **only inside
the authored helper**. That helper called `OpenProcess` with query/read access,
checked `QueryFullProcessImageNameW` against the selected executable locally,
queried `ProcessBasicInformation`, and read the PEB image-base pointer. The
reported image base matched the higher view. DOS, PE/COFF, section table,
image/header sizes and entry-point RVA matched the source; activity and
creation time stayed stable and the handle was closed. No agent was injected
into WoW by this operation. The attempted first contiguous `.text` sample
failed because that range was not readable; the retained result distinguishes
that failure from the successfully completed identity checks.

The higher view contained approximately 7.85 MiB of readable executable pages
and 52.72 MiB of inaccessible regions, rather than one uniformly readable
`.text` range. The lower alias's first `.text` sample still matched the
high-entropy disk data. A further guarded read sampled sixteen 4 KiB pages from
the higher view, including the entry-point page. All differed from their disk
counterparts; runtime entropy was 5.0841–6.4785 bits/byte versus approximately
7.95 for the corresponding disk pages. This supports analysis of the selected
runtime code pages. It does not establish complete unpacking, execution of a
particular instruction, a named protection mechanism or a key-handling function.

The corresponding authored tools are `runtime_pe_mapping_starts.py`,
`windows_cross_process_identity.js` and
`sample_runtime_executable_pages.py`; reports are retained in the current
laboratory with the same stems and run ordinals. The original file-backed
`memory_key_observer.py` remains unchanged. The separate
`memory_key_observer_v2.py` declares its Windows/Wine scope, accepts an explicit
image base, and requires matching source/PE headers, executable `.text`
mappings and process/source stability before and after a bounded scan. It
considers both private and shared non-executable RW mappings, prioritizes the
identified image's data, and reports omitted coverage. It preserves the
83 known-key controls, three-control layout consensus, one-context-per-name
limitation and mandatory independent CASC candidate verification.


The first executed v2 scan retained valid identity guards before and after.
It inspected 130,060,288 bytes of 2,515,386,368 eligible RW bytes before its
reserved scan deadline. It found zero of 83 control names and zero of the two
requested names; there were no read errors, layout consensus or candidates.
The remaining 2,385,326,080 bytes and 503 incomplete/unvisited mappings are
reported. This is not evidence that the keys are absent from the process:
coverage was partial, the selected structure may differ, and even the positive
controls were not observed. The report is
`reports/memory-key-observer-v2-00.json`.


An optimized literal matcher was retained separately as
`memory_key_observer_v3.py`. It removes the regex lookahead and advances by one
byte after each match to retain overlapping names. Its executed run reached
the byte budget: 536,804,576 scanned bytes, stable guards, zero read errors,
zero control/target names and no candidates. It omitted 1,979,626,272 eligible
bytes. The source/layout/control limitations above still apply; a faster
negative scan is not a recovered key.

`runtime_instruction_seeds.py` then linearly decoded only the currently readable
executable `.text` spans, with runtime/source/process guards before and after.
That snapshot contained 435 spans / 4,049,404 bytes. The bounded scan completed
in 3.116 seconds with 1,064,779 decoded instructions, 2,358 bad decodes and
46,690 RIP-relative instructions. One decoded instruction referenced the known
TactKey containing-string range, at build-local RVA `0x1b8bd99`. This is a
static instruction-reference candidate in a runtime snapshot. Linear decoding
is not complete control-flow recovery; it does not establish a key handler or
observed execution. The raw decoder output was not disclosed, and reports
contain only reviewed counts, categories and instruction RVAs. Use the fixed
source exception-directory metadata to delimit a candidate function before
attempting focused decompilation; reject missing/inaccessible code rather than
filling it with zeros.


### First completed focused REA/Ghidra decompilation

The source exception-directory entry containing the reference delimits
`[0x1b8bc20, 0x1b8be3b)`: 539 bytes. The guarded capture required the complete
range to be readable/executable before and after reading. It succeeded without
zero filling. The retained raw artifact SHA-256 is
`e07b3e9889f7796d1a8686c5ad9d19cbb2f9af863f5e9e8caa2e5188d0bd1ef6`;
its raw bytes and manifest remain private. The capture recorded the source
exception entry and chained-unwind limitation; it did not expand an inferred
function beyond that entry.

REA 6.7.0 analyzed this raw function with Ghidra 12.1.4 in 10.888 seconds,
using an explicit x86_64 raw-binary interpretation, the captured function's
actual load/entry address and the existing pinned Java/Ghidra environment.
The invocation shape is:

```text
rea function <private-raw-function> <captured-entry-address>
  --target-format raw-binary --architecture x86_64
  --entry-point <captured-entry-address> --provider ghidra
  --snapshot <new-private-snapshot.json> --full-output --format json
```

Run it from the external private laboratory with a restrictive umask. Save
complete stdout/stderr privately; do not print an unreviewed function dossier.
The completed Evidence ID is
`ev_a881e75cf9a4fe17293f224be7b276b3dc3c9c7f793c8042536d026f439c28de`.
It returned pseudocode, assembly and function facets. Its default recovered
prototype/calling convention is not a verified Windows function signature;
confirm register use before assigning semantic parameter names. The isolated
raw artifact also omits external callees and data contents.

A provenance check confirmed that the instruction targets the **start** of a
30-byte containing string labeled TactKey, rather than merely a neighboring
BroadcastText seed. The initial general runtime scanner had assigned all seed
categories a TactKey label, including BroadcastText; preserve that limitation
in its original report. The separate `runtime_instruction_seeds_v2.py`
corrects per-marker attribution and records exact target RVAs. Do not infer a
TactKey relationship from the old category name alone.

The recovered function accesses a hash table and record states. A bounded
read of the observed global header found a stable, non-null table with 256
buckets. Its candidate node layout and possible 16-byte key field are a
hypothesis derived from code offsets, not verified keys. The dedicated
`inspect_tactkey_cache.py` must validate coverage, table shape, cycles, state,
known-name/value controls and pre/post identity before retaining any private
candidate. Independent full CASC authentication remains mandatory.

The separate v4 memory scan also tested both name byte orders and exact
16-byte known-key values. It reached its 512 MiB budget with stable guards,
no read errors, no target names and no control names/values in the selected
coverage. Its remaining approximately 1.98 billion bytes are explicit. This
result motivates the directed table walk; it does not establish process-wide
absence or that a logged-in world session was observed.


### Directed cache walk: positive controls and remaining blocker

The directed walk completed: 256 bucket slots and 202 nodes, zero cycles,
zero read errors, zero omitted nodes/slots, stable table header and stable
source/process/image guards. It read 12,560 bytes. It found all 83 selected
control names. **69 name/value pairs matched their known 16-byte keys exactly**;
these 69 records had state 2. The remaining 133 records had state 0 and none
had state 1 or another state. Both requested missing names were present, but
neither produced an available candidate. No key file was exported.

These positive controls support the observed lookup and 16-byte field offsets
for this exact runtime cache. The default raw decompiler's parameter prototype,
the complete handler role, and server-delivery behavior remain separate
questions. The result does not prove that entering the world was observed or
that the service will supply those two values to this account. It also does
not prove permanent/global absence. The exact complete walk is retained as
`reports/inspect-tactkey-cache-00.json`.

The next useful comparison is a guarded repeat after a confirmed normal
client login/world session or an observed key-delivery event, reporting the
two target states and field availability without disclosing identifiers or
values. Do not replace this comparison with another unchanged whole-heap
scan. A later available candidate must still pass the frozen local CASC probe
against the actual encrypted frames and full-file CKey before acceptance.


The refined directed report, `reports/inspect-tactkey-cache-v2-00.json`,
explicitly cross-tabulated the result: 69 control records in state 2 and 14
controls in other states; both target records in state 0 with all-zero
16-byte fields. Its layout status is `supported_by_known_controls`, while
calling convention and complete handler semantics remain unverified.

### Bounded observation across login

A persistent user service was actually started and its first successful round
was checked: `rustycore-keyflow-cache-watch-20261019`. The authored controller
is `<current-laboratory>/R/watch_tactkey_cache.py`. It calls the fixed v2
observer every five seconds for at most twenty minutes / 240 rounds. Its
manifest pins the observer SHA-256 and the controller checks that pin before
each invocation. A source/process/image/header inconsistency stops the watch;
a child invocation also has a ten-second timeout. It does not log in, change
the game, request keys, capture traffic or upload anything.

Each round gets new 0600 reports and, only when a target has an available
nonzero candidate, a new 0600 private keyring. One candidate is retained while
the watch continues; two exported candidates end capture. The label
`capture_complete` is explicitly **not** authenticated key recovery or goal
completion. Full CASC verification is a separate required operation. No
candidate had been exported at the initial checked round.

The exact launch command and tool hashes are retained in
`reports/cache-watch-service-launch-00.json`. Round manifests/reports are in
`reports/cache-watch-00/`; candidate material, if any, is under
`private/cache-watch-00/`. The private service log emits only changed aggregate
states. A final `summary.json` records the bounded outcome. The operator was
asked to distinguish a current world session from a login screen; no answer
was assumed.

To stop this observation, stop only its owned unit:

```bash
systemctl --user stop rustycore-keyflow-cache-watch-20261019
```

Do not reuse existing round output directories: they deliberately reject
overwrite. For another observation, select a fresh external run root and
explicitly select the retained observer path. Reauthenticate PID, creation
identity, executable hash and the runtime image base first. A new build also
requires deriving the function/global offsets again. Never treat these
per-run addresses or the 69-control result as acceptance for another build.

Full Linux extraction, complete geometry/world resources and RustyCore runtime
acceptance remain open. REA investigation does not waive those gates.

### Cache writers, code boundaries and the raw-import ABI

The next directed executable-page scan used separate typed seeds for the
bucket count, bucket-array pointer and neighboring cache headers. It retained
42 instruction references in the readable/executable coverage. Source `.pdata`
entries bounded five additional captures (318, 257, 1,315, 1,450 and 399 bytes).
All five byte ranges were fully readable/executable and passed the retained
identity/header guards. REA/Ghidra returned evidence for each in 9–12 seconds.
The corresponding aggregate receipts are
`reports/cache-function-carve-<rva>-00.json` and
`reports/rea-carved-safecache<rva>00.json` in the private laboratory.

The 318-byte range writes the observed cache node fields; the 257-byte range
checks state 2 and a 16-byte value length before copying the value. These are
code observations, not an observed delivery of either missing key. In
particular, no incoming network-handler role has been established for the
writer: a local cache loader is still a possibility.

Two larger captures produce bad-instruction/control-flow warnings and depend
on registers initialized outside their selected entry. A `.pdata` range is
not sufficient proof that an isolated artifact contains an entire logical
function. Follow its `UNWIND_INFO`, including supported chained records, and
inspect incoming control-flow references before assigning a complete role.
Retain the original warnings; do not hide missing ranges with zero-filled
client-code claims.

There is also a reproducible ABI limitation in the original raw imports.
REA 6.7.0's `src/ghidra/GhidraRawBinaryLanguage.ts` selects
`x86:LE:64:default` with compiler specification `gcc` for `x86_64` raw binary.
This mapping was checked against the retained
[official REA source](https://github.com/morluto/rea/blob/a80575c73652d8b43d7340654a48bd20a4eb966d/src/ghidra/GhidraRawBinaryLanguage.ts).
That generic convention does not establish the Windows x64 parameter mapping
of this PE client. A returned `__stdcall` label is not an independent ABI
attestation. Preserve addresses, field offsets and positive-control evidence,
but verify parameter assignments under the Windows compiler specification.
Any generated PE carrier used for that comparison must be labeled synthetic,
retain the exact captured-code hash/range and identify generated headers and
padding separately. It is a private analysis artifact, never a reconstructed
complete client or an executable to launch.

A separate helper sent only bounded `WM_NULL` messages to the authenticated
visible WoW and Battle.net windows. Both responded (33 ms and 1 ms). The
reviewed result is `reports/windows-message-responsiveness-reviewed-00.json`.
This establishes window-dispatcher responsiveness at that observation; it
does not establish rendered gameplay, a world session, successful account
authentication or the cause of any earlier injection failure.

### Windows-ABI comparison and completed watch

The two non-chained entries were reanalyzed using private synthetic PE32+
carriers and the explicit REA settings below. The original function bytes,
runtime addresses and hashes were preserved; generated carrier headers and
alignment padding were separately declared. Neither carrier was executed.

```text
REA_GHIDRA_LANGUAGE_ID=x86:LE:64:default
REA_GHIDRA_COMPILER_SPEC_ID=windows
```

The local Ghidra installation provides `x86-64-win.cspec`. REA returned
successful evidence with the Windows analysis profile and `__fastcall`
prototypes in approximately 12 seconds per function. The writer at RVA
`0x1b8b9d0` now has a coherent one-input prototype: two reads of 8 and 16
bytes into local buffers, then a lookup/insert and a state-2 value assignment.
The getter at RVA `0x1b8bb10` has three inferred arguments: lookup pointer,
output buffer and output size. Its available branch copies 16 bytes from
node offset `0x1c`; unavailable states follow a separate interface path.
External callees remain opaque, so the 8/16-byte reads do not by themselves
establish file, packet or server origin. Types and names remain inferred.

The authored tools are `make_windows_abi_carrier.py` and
`R/run_windows_carrier_rea.py` in the current laboratory. The first consumes
the **full private carve manifest**, not the shorter aggregate receipt. The
second consumes the new carrier receipt. Each invocation requires a fresh
tag, verifies hashes/permissions and retains the full dossier privately.
Use `rea function <carrier> <entry> --provider ghidra` without the raw-binary
options; the explicit language/compiler settings apply to the PE import.
Original raw-import evidence remains retained with its original limitation.

The bounded provenance scan subsequently confirmed `CHAININFO` for the two
larger suspect entries and found one direct getter call plus two callback
address references. It examined 3,682,304 mapped executable bytes and
16,426,976 mapped read-only data bytes with stable guards and no cap hit.
The scan is linear decoding of selected mappings, not complete image control
flow. Its 2,382 bad decodes remain explicit. No aligned writer/getter pointer
was found in that selected read-only data coverage; this is not global
absence. The exact result is `reports/cache-function-provenance-00.json`.

The twenty-minute cache watch has now ended normally at its deadline:
**220 complete walks, one unchanged aggregate state, zero candidate rounds**.
All walks retained the 69 exact controls and both requested entries in state
0. Its final result is `deadline_no_candidates`, not authenticated recovery.
The owned unit is inactive. Do not describe the observer as still running,
and do not renew an unchanged observation without a meaningful new input.

A passive selected-window OCR attempt also retained the actual UMU wrapper
class (`steam_app_default.steam_app_default`), exact game PID and stable
window identity. It produced no allowlisted session labels or public error
codes. This inability to read a session state does not prove a disconnected
client or a hung renderer. The operator's world/session confirmation remains
unanswered at this checkpoint.

Following a secondary unwind entry back to its primary is only half of the
boundary check: other entries may also chain to that primary, including a
later epilogue. The first three contiguous registration captures formed a
694-byte region and still produced a truncated-control-flow warning. A
bounded source-metadata census in the declared nearby RVA interval found two
additional adjacent members of the same unwind family. Retain the 694-byte
evidence as incomplete and authenticate each additional capture separately.
Do not change the old receipt into a complete-function claim.

The getter's non-null global interface was also checked without calling it.
A bounded pointer walk found an object, an in-image vtable and an in-image
code target for slot `+8`. The code target had no source `.pdata` entry. A
separate maximum-64-byte leaf sample stopped **before reading code** because
the complete selected range was not mapped readable/executable. No code
artifact or instruction classification was produced by that attempt. Its
early-exit post-guard fields are unobserved, not proof that the process or
source changed. These results establish neither a true/false interface return
value nor a connected/world state. Retained aggregate receipts are
`reports/key-interface-presence-00.json` and
`reports/interface-leaf-classification-00.json`.

### Verified callback registration matches the public key extension

Five separately authenticated contiguous captures now cover the observed
registration region from RVA `0x1b8dae0` through `0x1b8de17` (823 bytes).
Each secondary entry's source `UNWIND_INFO` path was checked against the same
primary. `make_linked_windows_carrier_v2.py` preserves all five hashes, rejects
gaps/overlap and declares generated PE metadata separately. The updated
`R/run_windows_carrier_rea.py` verifies each constituent hash against its
actual carrier byte range before analysis. Both older receipts remain intact.

REA/Ghidra returned the expanded Windows-ABI dossier in 12.133 seconds:
`ev_9de623a2e790ecbbe9b089b071d670ce83858dc73ef86d18a862035a7edbda92`.
This version has a normal return and no truncated-control-flow/bad-instruction
warning; its overlapping-global-symbol warning remains. This is acceptance
of the selected analysis region, not an all-image function-family census or
runtime extraction acceptance. Its receipt is
`reports/rea-windows-carrier-saferegistration01.json`.

The recovered code registers the exact getter address with another interface
and places the exact writer address into a callback registry. A separate local
comparison found that the writer's registry selector equals the public
`BroadcastText` key-extension tag recognized by the pinned
[wow.tools.local parser](https://github.com/Marlamin/wow.tools.local/blob/5b2f85db477bd42205fde912ed72b3adadda218a/Controllers/KeyController.cs).
The match occurs in the selector comparison and store, not just as an
unattributed nearby literal. The 8-byte lookup / 16-byte value reads also
match that parser's extension layout. The reviewed comparison is
`reports/registration-broadcast-extension-comparison-00.json`.

This links the observed runtime cache writer to a known key-extension format
and gives a concrete delivery path to investigate. It does **not** establish
that the current client received such an extension, that either missing
lookup was delivered, or that this account can obtain it on demand. The
earlier complete retained-cache audit found no recognized extension, and the
220-round live observation found no candidate. No key value has been
recovered in this checkpoint. A later candidate still requires the frozen
full-frame and full-file-CKey CASC authentication before being called usable.

### 2026-10-19 — Delivery conditions, live registration and execution counters

The next observations use the same exact build-70450 source hash and private
laboratory above. They do not recover either requested key. The public selector
is also the `TactKey` table tag; its match with the extension parser does not
establish an exclusive `BroadcastText` source, a particular NPC/dialogue, or a
specific server packet in this client build.

`inspect_key_extension_registry.py` walked the fixed registry at RVA
`0x52150c0`, with bounded reads and process/source/PE/header guards before and
after. The initialization flag was one. Eight buckets contained one node;
the public selector's stored callback target was exactly the authenticated
writer at RVA `0x1b8b9d0`. Its manager and invoker were in-image code pointers,
but neither had a source `.pdata` entry. The 1,895-byte walk completed in
0.645 seconds with stable guards. Receipt:
`reports/key-extension-registry-00.json`. Registration proves that the
delivery handler is installed, not that any key payload has arrived.

The Windows-ABI getter dossier and a separate instruction review establish
these local conditions for the observed query path:

1. The lookup must already have a node in the local cache. An absent node
   returns without taking this query path.
2. State 2 serves the stored 16-byte value. States 0 and 3 can reach the
   interface check; other observed state values return without that query.
3. The fixed global interface must be non-null and its virtual slot `+8`
   must return true. Its meaning is still unidentified: presence alone is
   not proof of a connected or in-world session.
4. The query callsite supplies the node's 32-bit row-ID field and a callback
   wrapper. A result of 2 marks state 1 and records a timestamp. This does not
   yet identify the callee's packet format or the server's delivery policy.

The two selected missing lookups previously had nodes in state 0, so their
absence alone does not explain the unobserved query. The callee and interface
leaf remain outside the authenticated executable captures; retain their
mapping rejections rather than substitute source bytes or guess semantics.
Reviewed callsite metadata: `reports/getter-query-call-reviewed-00.json`.

`observe_key_execution_counts.py` then used Linux `perf stat` hardware
execution breakpoints at the exact writer, getter and query-callsite addresses
for one bounded 20-second interval. All three counts were zero; `task-clock`
was 6,339.38 milliseconds and all counting percentages were 100. Process
creation identity, source hash and runtime PE guards remained stable. Receipt:
`reports/key-execution-counts-safedelivery00.json`. No registers, stack,
payloads, callchains or `perf.data` were captured, and no client code was
patched. The observer ended; no continuous monitoring is claimed. These
counts establish no executions at those addresses during that interval,
not lifetime absence, a disconnected state, or an impossible delivery.

A bounded RX cross-reference scan found initialization/unregistration code
and one caller, but no consumer in its selected coverage. It covered
3,387,392 executable bytes in 383 spans, with 2,054 undecodable positions;
this is not an all-image control-flow census. Receipt:
`reports/runtime-delivery-xrefs-00.json`.

The initializer's caller required three authenticated contiguous unwind
members, covering RVAs `0x2118c0` through `0x211bdc` (796 bytes). The first
single-member analysis remains retained as truncated. The generic
`make_linked_windows_carrier_v3.py` accepts 1–16 contiguous captures, verifies
each hash and common unwind primary, and declares generated PE metadata
separately. The reviewed runner now accepts that carrier version. Expanded
Windows-ABI evidence:
`ev_9058a75b849499acb9c211c1eb26ca40697133595e876c52f275e632d597b446`.
It has a normal return and retains the overlapping-global-symbol warning.

One callback installed by that caller, RVA `0x211730`, constructs a structured
object from a 32-bit input. Its 389-byte capture has a normal return:
`ev_9d0b3c8d17d6b6d71136044a9e19f0c826d1b76189ce247f10f3af74007135f6`.
Its relationship to the actual key query is unproven. In particular, do not
label it `DBQueryBulk`, `HotfixRequest`, or a network request from object
construction alone. A separate 63-byte adapter capture at RVA `0x1bf5d10`
still ends in truncated-control-flow evidence and has no complete-function
claim. Their receipts use tags `safeadapter21173000` and
`safeadapter1bf5d1000`.

Before the next live RTTI/session probes, PID 1184593 ceased to exist. The
RTTI attempt read zero candidate bytes; the repaired Lua probe records
`process_identity_unavailable` at `pre_identity`, with post guards explicitly
unobserved. The initial RTTI helper's generic internal failure is not a
permission-denial diagnosis. Retained receipts:
`reports/key-request-rtti-00.json` and
`reports/lua-session-api-discovery-v2-00.json`. There is no established exit
cause or confirmed world/session state, and no identical live retry was made.

Two separate source-only diagnostics continued without the exited process:

- `R/discover_lua_registry_static.py` scanned 12,942,900 source `.rdata`
  bytes and the complete 2,867,316-byte exception directory under stable
  pre/post source guards. It found the fixed public labels `IsLoggedIn` and
  `UnitExists`, but no qualifying aligned name/function-pointer pair. This
  says nothing about the client's live login/world state. Receipt:
  `reports/lua-registry-static-00.json`.
- `inspect_source_request_rtti.py` checked the two fixed object candidates
  using the PE preferred image base. One had no valid in-image RTTI locator;
  the other is in writable `.data`, outside this read-only RTTI hypothesis.
  All source guards passed; neither yielded a type identity. Receipt:
  `reports/source-key-request-rtti-00.json`. A negative static hypothesis
  does not rule out other runtime type metadata.

Repeatable continuation: first establish a current exact-build process and
creation identity, then resolve its actual image base. Recreate scoped live
probes with those verified inputs and new output names; never reuse the old
PID/base blindly. Resolve the interface predicate and actual query callee
before attributing a player action or packet. Correlate a bounded execution
observation with a confirmed session transition and recheck the two nodes.
Only a newly captured candidate warrants the frozen authenticated CASC check.
Authored diagnostic hashes are retained in
`reports/authored-diagnostics-manifest-v6.json`; client captures, dossiers,
raw diagnostics and all key material remain private outside Git.

### Same-day reentry — captured predicate, unresolved indicator meanings

The operator's continuing launch authorization was used to reopen the exact
client once through a separate, owned Windows helper with REA's fresh Frida
provider. The helper first enumerated exact game basenames and found none,
then used `CreateProcessW` with the exact executable and working directory,
no arguments and no inherited handles. It closed all returned handles and
did not attach to or inject into WoW. Receipt:
`reports/windows-reopen-pinned-client-00.json`.

The new process was Linux PID 1639188, creation ticks 89374969, Windows PID
2712. Its PEB base was **`0x6ffff0720000`**, different from the earlier base.
Exact process path, active/creation identity and source PE headers matched
before/after the helper inspection. That inspection's optional text sample
failed, so its overall status remains a retained native/parse failure, not
an entirely successful inspection. Its successful structural-header fields
were independently checked by the later probes. Receipt:
`reports/windows-cross-process-identity-2712-00.json`.

The new cache walk again found 202 nodes, the same 69 exact state-2 controls,
and both selected targets in state 0 with zero value fields. No candidate
file was created. Receipt: `reports/reopened-client-cache-00.json`.
Selected-window OCR with stable PID/window guards found the public labels
`Blizzard`, `Iniciar`, `Sesión`; it does not prove a world session. Battle.net
was then opened once in the same Wine context through its official launcher;
the existing game was left untouched. Its library/home window was observed,
but no successful official game launch or session transition is claimed.
Receipts: `reports/game-ui-reopened2712-00.json` and
`reports/windows-launch-official-battlenet-01.json`.

`pinned_live_probe_2712.py` records the new PID/startticks/base overrides and
runs the existing observers under independent source/PE/creation guards.
Both inner reports passed their guards. The complete selected runtime
`.rdata` scan found the same two public Lua labels but no qualifying pointer
pair; the RTTI candidates yielded no type identity. Receipts:
`reports/reopened2712-lua-00.json` and
`reports/reopened2712-rtti-00.json`, with their exclusive `.inner.json`
reports. The wrapper's post-source `observed.source_sha256` bookkeeping flag
remains false despite its true hash-result field; rely on the explicit inner
pre/post hash guards, not that inconsistent wrapper flag. Preserve the
executed wrapper unchanged; repair it in a new version before future use.

The interface leaf at RVA `0x4eb7b0` was successfully captured as a guarded
64-byte RX sample in this process. The classifier conservatively stopped at
its first branch. Separate local review followed both conditional paths
inside the first 36 bytes: the code checks the pointer stored at global RVA
`0x50612d8`, then bytes at object offsets `+0xc49` and `+0x7e8`. All three must
be nonzero for its true return; the other paths return false. Both return
instructions and their branch targets are inside the captured interval.
This identifies the structural predicate, **not the meanings of the two
indicators or their current values**. No `.pdata` boundary exists for this
leaf; do not promote the 64-byte sample to a source-bounded function claim.
Evidence: `private/interface-leaf-reopened2712-00.json` and
`reports/predicate-leaf-reviewed-2712-00.json`.

Page permissions were transient: metadata initially showed these code ranges
as RX, but subsequent function captures encountered `---s`. Two calls to the
old carver also stopped before reading because its old PID/base pins did not
match the new process; this was local stale-input validation, not an OS/MCP
permission denial. Separate rebound carver/carrier/runner files preserve the
old files, add the exact new creation-time pin, and retain all source, PE,
hash, RX and artifact guards. Their two callee capture attempts at RVAs
`0x1b8e9d0` and `0x1b8e8e0` then stopped at the full-extent RX guard, reading
zero code bytes. No new carrier or Ghidra analysis was produced for them.
Receipts: `reports/reopened2712-carve-1b8e9d0.json` and
`reports/reopened2712-carve-1b8e8e0.json`.

`sample_delivery_predicate_2712.py` was prepared to read only the exact global
pointer and two individual object bytes, conditional on the live RX sample
still matching the retained code hash and on stable source/PE/creation and
interface-chain guards. Its single attempt stopped at `sample_pre` because
the code was no longer fully RX; it read **zero predicate/chain bytes**.
Its unobserved post fields are not evidence of source/process changes.
Receipt: `reports/delivery-predicate-2712-00.json`. No protection was changed,
no protected-page read was forced, and no identical capture was retried.

The public-Lua-seed RX search completed one bounded scan of 3,394,044 bytes
in 391 selected spans, with stable source/PE/creation guards, 2,032 bad
decodes and no matching seed reference. Receipt:
`reports/runtime-lua-seed-xrefs-2712-00.json`. This is selected mapped coverage,
not all client code. Its two general RIP-total counters were initialized but
not maintained by the helper; their zero values are not valid totals. The
target-matching loop ran, but the receipt supplies no global absence claim.
Preserve the executed helper and repair counter bookkeeping in a new version
before reusing it.

The next causal comparison needs a confirmed official login/world transition
and new observations of those same structural indicators, followed by their
writers and the actual query callee. The operator was asked to confirm world
entry after this reopen. There is still no identified player action that
delivers either selected key and no authenticated recovery. The later
authored diagnostic source inventory is
`reports/authored-diagnostics-manifest-v7.json`; it includes prepared variants
that were not executed, as distinct from the receipts of actual attempts.

### Confirmed world session — available query gate, unresolved target states

The operator then confirmed being inside the world. Discovery found a different
exact-build process: Linux PID **1656634**, creation ticks **89478947**, Windows
PID **700**, PEB image base **`0x6ffff05f0000`**. The earlier PID/base must not be
reused. Exact executable path, creation/active identity and source/runtime PE
headers matched. As before, the identity helper's optional text sample failed;
its overall failure is retained, while successful structural fields were
independently checked by the later probes. Receipts:
`reports/world-client-windows-discovery-00.json`,
`reports/world-cross-process-identity-700-00.json` and
`reports/world-session-binding-700-predicate-00.json`. The last is the strict
probe binding; a separate binding retains operator-confirmation provenance.

The guarded cache walk found 202 nodes in 256 buckets. All **83 known controls
now match exactly in state 2**, compared with 69 state-2 controls in the earlier
login-screen process. Both selected missing entries still have zero value
fields. A new report version exposes their numeric state histogram only:
**both are state 3**. It exports neither selected names nor key values and
creates no candidate key file. All process/source/PE/cache stability guards
passed. Receipts: `reports/world-client-cache-700-00.json` and
`reports/world-client-cache-states-700-00.json`. This is a comparison between
different processes, not a continuously observed state transition. State 3
has not yet been identified as a timeout, missing row or server denial.

`run_rebound_delivery_predicate.py` validates the strict binding and Windows
identity receipt before rebinding the existing byte probe. Its one execution
verified the retained 64-byte RX code hash, interface chain, source/PE and
creation identity before/after reading the selected fields. The internal
global object is non-null and both bytes at `+0xc49` and `+0x7e8` are nonzero:
**the local query predicate is true in this confirmed world session**.
Receipts: `reports/world-delivery-predicate-700-00.json` and its inner report.
The two indicators' individual meanings and the action that changed them
remain unknown; the earlier login-screen byte probe was unobserved.

Fresh current-process carves of the query helper, node helper and callback
adapter at RVAs `0x1b8e9d0`, `0x1b8e8e0` and `0x1b8ed40` each encountered
`---s` over their selected extents and read zero code bytes. No protection was
changed and no protected read was forced. These are retained current-process
results, not successful captures or identical retries on the older process.
Their receipts are `reports/world-carve-<rva>-700-00.json`.

The request-setup region at RVA `0x1b8bc20` was captured successfully: 539 bytes,
stable source/creation/PE/RX guards and an exact match with the retained older
capture. Windows-ABI REA evidence
`ev_f80a358f1c6d946da0b5d80bd7a4d0b06c69eeb45b3f2d54a6ef4ca81c334edc`
has a normal return and the overlapping-global-symbol warning. It returns for
an available state-2 node and can defer a state-1 node until its cooldown.
A local TactKey row or the true interface predicate admits the query path;
that path records pending state 1 and a timestamp, constructs callback
pointers and calls the query helper. This identifies additional local query
conditions; it does not identify a packet or write state 3.

Two guarded 256-byte RX neighborhoods provided narrower structural checks.
The installed leaf at `0x1b8d610` treats a zero lookup as ready and otherwise
tests the matching key-cache node for state 2. The leaf at `0x1b8bef0` walks
the cache and returns false when any node has state 1. Neither leaf has a
source `.pdata` boundary; their branches/returns can be reviewed inside the
samples, but the full 256-byte neighborhoods are not complete functions.
Their neighboring partial instructions must not be treated as complete code.
Receipt: `reports/world-key-state-neighborhoods-700-01.json`.

Source-only unwind inspection and separate guarded carves expanded the
installed callback at `0x1b8d670` into five contiguous members: 34, 15, 603,
68 and 9 bytes, ending at `0x1b8d949`. Every member resolves to the same
source unwind root; the subsequent metadata gap was not filled. The new
`make_linked_windows_carrier_v4.py` preserves all v3 checks and also validates
and retains creation ticks, as required by the rebound analysis runner.
Windows-ABI evidence
`ev_ba4ab120e421387e4747ccdcb9b6bc748d236f3985cad0ac5d40657177a70790`
has a normal return. It stores/moves callbacks in a nested map using globals
different from the key cache; it is not established as the state-3 writer.
The initial source-metadata parser failure remains retained beside the
corrected `reports/world-d670-unwind-family-source-700-01.json`.

The adjacent cache iterator at `0x1b8be40` required three contiguous unwind
members, 29, 133 and 6 bytes, ending at `0x1b8bee8`. All source and live carve
guards passed. Windows-ABI evidence
`ev_f781e1289bd4a7d46369341d7f363cc5422b9cce59c9c4ff54adb9bd07afd818`
has a normal return, with type-propagation and overlapping-global warnings.
It walks the key cache and calls the request-setup routine for its nodes.
The caller/registration that activates this iterator is still to be resolved;
the iterator itself does not establish a state-3 assignment. Source metadata:
`reports/world-be40-unwind-family-source-700-00.json`. Reviewed structural
observations are retained separately from raw pseudocode in the laboratory.

Two bounded GDB hardware-entry attempts were retained. The first installed
three hardware breakpoints but stopped on `SIGUSR1` without a selected hit.
The new v2 observer preserves the original helper, passes that Wine signal
through without stopping, retains failed/partial captures and uses explicit
detach/identity-guarded cleanup. It then stopped after **773.773 ms** on a
`SIGTRAP` at client RVA `0x2ce407`, outside the three selected entries. Neither
attempt reached the intended 25-second observation window or captured callback
code. Both detached, cleared the tracer and passed every post source/PE/
creation guard; neither reported a permission rejection. No software
breakpoint fallback, target-function call or signal suppression for SIGTRAP
was used. This is a debugger limitation with an unresolved trap cause, not
evidence of no queries, an anti-debug diagnosis or a server decision. Receipts:
`reports/world-callback-gdb-safeworld70000.json`,
`reports/world-callback-gdb-v2-safeworld70001.json` and the redacted stop reviews.

The esES `DBCache.bin` is still 3,334,666 bytes with the previously recorded
digest and stable metadata. No new cache content or delivery was observed,
and the all-cache parser was not rerun against identical bytes. Receipt:
`reports/world-dbcache-continuity-700-00.json`. A historical 399-byte neighboring
routine analyzed under tag `safereply1b8d11000` uses a different map and is
not identified as a key-reply handler. Keep that negative interpretation and
the older v1-carve/schema rejection; neither justifies a key or state claim.

Repeat procedure after another client/process change: discover the exact
canonical executable and Windows/Linux creation identities; obtain the actual
PEB base; create a new strict binding; then run new-name cache/predicate probes
with all original code-hash, PE, RX and stability guards. For split functions,
inspect source unwind families, capture each authenticated contiguous member
and use the v4 carrier without filling gaps or rewriting older receipts.
Another build additionally needs newly established code/layout identities and
its own admission gates; changing the PID/base does not admit a different build.
Do not reuse the executed GDB observers unchanged as if they supplied a full
window. Resolve the iterator's activation and the actual response/state writer
before assigning a quest, NPC or packet as the trigger. No missing key has
been recovered or authenticated. Authored-source hashes are in
`reports/authored-diagnostics-manifest-v8.json`; all captures, keys, raw debugger
logs and full REA dossiers remain in the private external laboratory.

### Same-session continuation — numeric activation event found

The new `world_refresh_xrefs_700.py` uses the strict current-process binding,
20-second/4-MiB limits, a three-second final-guard reserve and at most 128
reported references. It refreshes mapping metadata immediately before and
after every captured span and decodes only spans that pass both RX checks.
Failed dumps remain private; protected spans are not read. Its single run
accepted 205 spans covering **1,232,896 bytes**, then stopped when the next
planned span was no longer fully RX. All binding, source, creation and runtime
PE guards passed. The report retains its failure status and incomplete
coverage: 707 initial spans were found, and 1,003,100 bytes were also outside
the four-MiB plan. It examined 337,415 instructions with 458 bad decodes and
reported 41 references. Its 15,300 RIP-relative instructions and zero missing
target comments are maintained counters, unlike the older Lua-seed helper's
unused total counters. Receipt: `reports/world-refresh-xrefs-700-00.json`.

One positive result in an accepted span was a direct call from **`0x211614`
to `0x1b8be40`**. A separate full-extent RX carve independently corroborated
the containing source range `0x2115f0`–`0x21162f` (63 bytes), with all original
source/PE/creation/RX guards. Windows-ABI REA evidence
`ev_8b9262c0b71c0e9a94469e32e69b1f7e88fdf0988ae512ebd8ad145ee0026d90`
has a normal return and an overlapping-global warning:

- Input event code **3** calls the key-cache iterator immediately, writes
  5000 to a 32-bit global field at `0x5223e80`, and sets byte `0x524850d` to 1.
- Input event code **4** changes that field and clears the byte.
- Other input values return without those actions in the captured region.

This supplies an actual activation callback and its numeric condition. The
event enum names, the field's timing unit and the event's connection to a
specific login/world transition are **not established**. Event code 3 and
key-cache node state 3 belong to different fields; their equal numbers do not
establish a relationship. The code was analyzed, not invoked by the probe.

Review of the retained initializer-caller dossier confirms that it registers
the `0x2115f0` callback through `0x4b1d60`. A current-process capture of that
101-byte registration routine stopped at its RX guard with zero bytes read,
so its event-dispatch implementation and enum identity remain unresolved.
The nearby registered 251-byte callback at `0x211630` was captured and
analyzed: it parses a numeric input, combines local feature bits and selects
from nine handlers. Its normal-return evidence is
`ev_2dff45e3fd8d0e5198706ff1089d1f993b92f9013be5609aac61288e73644ece`.
That structure does not identify it as a key request, network message or a
command safe to invoke. It was not invoked.

A separate guarded source-only search of the 86,509,776-byte executable found
no exact preferred-base absolute-pointer occurrences for the iterator, setup
or response-adapter targets. This negative byte-pattern hypothesis does not
rule out relocated, computed or dynamically installed references. Receipt:
`reports/world-source-refresh-pointer-search-700-00.json`. Retained capture
comparison `reports/world-request-setup-capture-continuity-700-01.json`
confirms the two 539-byte setup captures are identical; the initial comparison
missed the older filename because it named an interior requested address.

The closing bounded cache walk, with exact creation identity before/after and
stable cache/source/PE guards, again found both selected entries in state 3,
zero available candidates and no private key output. This is an end-point
observation, not continuous monitoring or proof of no intervening request.
Receipt: `reports/world-client-cache-final-700-00.json`. Consequently no new
candidate was passed to the authenticated CASC verifier. The remaining gaps
are the activation event's semantic identity and the actual query-response
path/state-3 writer; delivery of either selected key is still unproven.
Reviewed structural conclusions are in
`reports/world-reviewed-control-flow-700-02.json`. The later authored-source
inventory, with 74 sources and unchanged prior helper hashes, is
`reports/authored-diagnostics-manifest-v9.json`.

### Same-session continuation — empty target hotfix observations

A new disk-only audit joins the two private requested lookups to unique
`TactKeyLookup` row IDs, then selects the corresponding TactKey records in
the unchanged build-70450 HTFX v9 cache. The strict v2 parser validates each
record magic and payload boundary and requires eight-byte lookup values.
All source hashes and metadata remain stable before/after. It examined all
32,917 records within its 64-MiB, 100,000-record and ten-second limits:

- Both selected rows occur in the cache. All **eight observations have a
  numeric operation/status byte of 4 and zero payload bytes**, including the
  latest observation of each row under the recorded push/ordinal ordering.
- Across the TactKey table there are 490 observations: 14 with byte 1 and
  16-byte payloads, and 476 with byte 4 and empty payloads.
- Presence in the static source means presence in **TactKeyLookup**; it does
  not establish that either key value occurs in static TactKey.

Receipt:
`reports/audit-target-key-cache-status-v2-20261019T084410Z-1792399450575286177-0.json`.
The executed v1 and its earlier failed receipts remain retained; v2 was
created separately rather than changing an executed helper. This is a new
selection/query against previously audited bytes, not a new cache delivery.

The pinned fork's
[HotfixRecord enum](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/src/server/game/DataStores/DB2Stores.h)
names protocol value 4 `NotPublic`. Its hotfix-connect response path can
serialize a stored status, whereas its bulk DB-query path defaults to
`Invalid` and changes an existing record to `Valid`; that handler does not
assign `NotPublic`. See the pinned
[handler](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/src/server/game/Handlers/HotfixHandler.cpp)
and
[serializer](https://github.com/advocaite/TrinityCore/blob/e3abffa1e701e7387739e39f35c42071eac2d5c5/src/server/game/Server/Packets/HotfixPackets.cpp).
This does **not** establish that the cached byte is that exact protocol enum:
[DBCD's HTFX v9 structure](https://github.com/wowdev/DBCD/blob/e732093f8864240fc5884bd1bba6b02f3dfc0d56/DBCD.IO/Common/HTFXStructs.cs)
calls it an operation and only treats operation 1 as valid. The independently
inspected
[wow.tools.local parser](https://github.com/Marlamin/wow.tools.local/blob/5b2f85db477bd42205fde912ed72b3adadda218a/Utils/DBCacheParser.cs)
retains the numeric byte. Consequently **`NotPublic` is a candidate semantic
mapping**, not a proven live server/account decision. The cache operation,
protocol status, heap-node state 3 and activation event 3 must remain distinct
until their actual adapters/emitters are traced. The recorded empty payloads
contain no recoverable key values.

### Callback inventory and readable-alias limits

The offline retained-code inventory authenticates 23 captures intersecting
the key subsystem. The separate syntax scan checks 7,118 retained bytes,
finds the already known pending-state write and no explicit immediate-3
store pattern. This is not a complete dataflow analysis or proof that no
state-3 writer exists. The query helpers and response adapter are outside
the retained coverage. Receipts:
`reports/retained-key-code-inventory-700-00.json` and
`reports/retained-key-state-writes-700-03.json`; earlier failed/incomplete
variants remain retained.

Initializer review identifies two other callbacks registered through
`0x4b1d60`: `0x1f77840` and `0x28fb0e0`. The former has a unique 28-byte
source exception-directory entry; the latter has no matching entry. Their
ordinary guarded live capture/sample attempts read zero code bytes, so these
addresses and registration order do not establish callback roles. A new
bounded RX reference scan accepted 297 spans / 2,490,368 bytes before a
subsequent span changed permissions. It corroborates the three initializer
registration sites but has incomplete image coverage. The separate offline
scan of 28 authenticated captures / 8,168 bytes finds no additional retained
emitter/dispatcher references to the activation callback or its two fields.
Receipts: `reports/event-registration-xrefs-700-00.json` and
`reports/retained-callback-reference-scan-700-00.json`.

One newly captured readable 64-byte neighborhood at `0x1f0ed0` has a small
copy/address-management leaf whose reviewed return paths fit within its
first 29 bytes. Operations 1/2 copy 16 bytes and operation 3 stores a pointer;
it does not write the key-cache state. There is no source function boundary
for this leaf, and the neighboring partial code is not a complete function.
Receipt: `reports/callback-carrier-leaf-sample-700-00.json`.

Metadata inspection found an existing readable shared mapping of the same
backing object and offset ranges as the protected original function views.
The new `readable-code-alias-carve-v1` diagnostic preserves that distinct
provenance: it never reads original protected code addresses, changes page
permissions, invokes a target function or opens backing-file content. Before
reading a new target it compares two alias reads of the known 539-byte setup
control with its authenticated retained capture. It also checks exact
process creation/binding, source hash/PE, shared object/offset arithmetic,
mapping stability, two equal target reads and all final guards. It does not
claim RX capture or automatically establish plaintext.

Two one-function captures passed those guards: the 73-byte response adapter
at `0x1b8ed40` and the 101-byte registration entry at `0x4b1d60`. Both alias
ranges equal the corresponding source-file bytes and decode incoherently as
ordinary Windows functions, including respectively six/four invalid decode
rows and privileged I/O instructions. The known control still matches.
Thus a readable alias is **not sufficient to recover the usable code** at
these entries. No pseudocode or event/state semantics are inferred from
them. Their manifests remain private; reviewed aggregates are in
`reports/readable-code-alias-reply70000-1b8ed40.json`,
`reports/readable-code-alias-eventreg70000-4b1d60.json` and
`reports/readable-alias-code-review-700-00.json`. The separately prepared
Windows-ABI alias consumer remains unexecuted.

For another process/build, re-establish all source, creation, PEB and layout
pins. Reproduce the strict disk join without printing row IDs or values;
retain numeric cache operations until their client mapping is proved. Any
future alias capture needs its own truthful schema, a known-byte control and
all mapping/identity guards; equal source bytes or a readable view alone do
not admit those bytes as decoded executable code. No requested key has been
recovered or authenticated, and the event enum and response/state-3 writer
remain open.

The follow-up cohort audit preserves the v2 parser and compares row sets
locally. All 476 empty operation-4 observations represent **119 distinct
rows**, with four records but **one push per row**. The two selected rows
share that exact profile with the other 117. The 14 operation-1 rows are
disjoint from those 119. All 133 cache row IDs belong to the 202-row static
TactKeyLookup universe and none overlaps the 69 direct static TactKey rows.
This accounts for the previously known 83 available keys and the remaining
119 rows without providing additional values. It does not prove availability
for other accounts, an unpublished-content policy or a specific server cause.
In particular, four retained records must not be presented as four separate
delivery attempts. Receipt:
`reports/audit-tactkey-cache-cohorts-v1-20261019T085614Z-1792400174680867889-0.json`.

A read-only remote HEAD check still returns the pinned TACTKeys commit
`2394fc00248074b9624ebb6749934f7a41a5f09e`; no replacement dictionary was
downloaded. A bounded source-only check of twelve initializer string-pointer
RVAs found no matches for a fixed public-term dictionary. It emits only
length/termination and known-term categories, not client strings, and provides
no subsystem or event identity. Receipt:
`reports/public-string-terms-safe-init-string-rvas-00.json`.

The initializer also installs a pointer to `0x211730`. A new independent RX
carve captured its 389-byte source-bounded extent with all source, creation,
PE and before/after RX guards true. Windows-ABI REA evidence
`ev_8f212c87f929692ac1bbc8298f83f5af47d3450a9119aab9e410e9113f88024c`
returns normally, with an overlapping-global warning. The reviewed body builds
a local object, invokes three optional indirect hooks and other helpers, and
cleans up the object. It has no direct reference to the two activation fields
or direct call to the key-cache iterator; opaque callees remain opaque. Its
installation alone does not identify it as the retry timer or event emitter.
Receipt: `reports/installed-neighbor-carve-211730-700-00.json`.

The new field-focused RX scan uses a different explicit target selection with
the unchanged xref helper. It accepts 103 spans / 704,512 bytes, corroborates
the four known activation-field references, and then stops at a changed RX
span. All final identity/source/PE guards pass, but that partial search adds
no field reader or event emitter and proves no global absence. Receipt:
`reports/event-field-xrefs-700-00.json`.

The new passive RX-availability observer completed its configured 25-second
map-sampling window: 2,483 samples, 26.253 seconds including setup/final
checks, all binding/source/creation/retained-control guards true. The four
fixed entries started protected and no sample observed complete RX coverage.
Consequently it made **zero carver attempts and read zero target code bytes**.
Sampling at intervals of at least ten milliseconds cannot exclude shorter
transitions or execution through another view; this is not absence-of-query
evidence. It used no debugger, signal, target invocation or permission change.
Receipt: `reports/rx-availability-watch-700-00.json`.

The v10 authored-diagnostic inventory records 86 sources, with all 74 v9
hashes unchanged. It includes the new strict audits, alias tools, retained-code
scans and observer, distinguishing prepared tools from their executed
receipts. Keep the full private dossiers and unsuccessful captures. A useful
continuation must obtain the actual event emitter/response adapter with
authenticated usable code or trace a real response; repeating the same
protected carve or calling cache byte 4 a proven server refusal does not
close either gap. No observer remains running after this bounded window.

### Offline code-recovery model: independent byte-exact corroboration

A new route follows the author's primary
[analysis of lazy code encryption in another Blizzard title](https://pwn.dog/posts/16-kilobytes-in-the-clear/):
compare known runtime code with its encrypted file bytes, then look for the
resulting XOR material in the adjacent loader. This supplied a hypothesis,
not acceptance for WoW. No patching, protection change, debugger attachment
or new network key source was used to investigate it.

The local `WowB_loader.dll` is 41,341,648 bytes, AMD64/PE32+, SHA-256
`f7cefe21ed6933d1fdaac02bb755186d5f98481449f6f27230b0ccf693615e71`.
Its layout was read and hashed before/after; the report exposes only PE
metadata and allowlisted public-symbol matches. Source and loader remain
read-only. Receipt: `reports/loader-layout-v1-safeloader7045000.json`.

The new offline XOR probe uses the authenticated 539-byte setup and 257-byte
getter captures. All 33/17 selected overlapping 32-/64-byte fragments have
exact matches at coherent positions in the loader, and none in the source
executable. These are two captures of the same code page, not fifty
independent pages. Full source/loader hashes and metadata match before/after
the bounded 255,702,848-byte read. Receipt:
`reports/code-xor-material-v1-20261019T095653Z-1840994-d61d4dbd2404.json`.

Two new complete aligned RX page captures, at `0x1b8b000` and `0x211000`,
provide separate ground truth. All original source/PE/process/RX guards pass
for both 4,096-byte ranges. The input-selection record explicitly changes
the unchanged sampler's neighborhood sizes and byte cap; its original
256-byte template labels remain retained in receipt 00. Receipt
`reports/plaintext-reference-pages-700-01.json` corrects those labels without
changing capture sizes, bytes, hashes or guards. No whole-function extent
is inferred from these page samples.

A source-only lexical loader scan finds candidate references enclosing a
16-KiB region containing the observed XOR material. It hits its 256-reference
limit, includes overlapping REX/no-REX candidates and is not a verified
complete disassembly. The one fixed circular-region hypothesis was recorded
**before** the full-page comparison in
`reports/preregistered-loader-pad-hypothesis-700-00.json`. The earlier
contiguous-window verifier remains retained with `relocation_entry_limit`;
it selected zero candidates and establishes no negative cipher result.

The separate circular verifier authenticates that preregistration, source,
loader, page report, manifest and raw hashes. Each page independently uses
only its first 64 XOR bytes to select a unique position in the fixed region;
the remaining 4,032 bytes are excluded from selection. Both reconstructions
then match **every byte** of their 4,096-byte runtime captures: 8,192 bytes
total, including all 8,064 reserved bytes, with zero mismatches or relocation
patches. Every final guard passes. Neither selected page requires wrapping,
so this run does not independently validate a wrap crossing. Receipt:
`reports/code-pad-cycle-validation-v1-20261019T100623Z-1855621-b9cec9166fab.json`.

This corroborates a code-recovery model for those exact two pages and that
exact executable/loader pair. It does **not** recover either TACT asset key,
authenticate every unobserved code page, identify the event enum or establish
the server's policy. Material bytes, page contents and full dossiers remain
private. A further reconstruction must select an unobserved page's position
without using unknown plaintext and retain its inferred provenance; it must
not masquerade as an RX runtime capture.

The next diagnostic selects each page's phase from source unwind metadata,
using all eligible `UWOP_PUSH_NONVOL` terminal-byte constraints. Its two
allowed encodings are an explicit instruction-encoding hypothesis, not a
guarantee supplied by the unwind ABI. It searches all 16,384 phases and
requires a unique result with at least four constraints from two functions;
it neither ranks partial matches nor derives the phase from the RVA.
Control phases are frozen before opening their retained plaintext pages.
Both independently selected controls again match all 8,192 captured bytes,
with zero mismatches. The historical cycle receipt authenticates only those
two control pages; it contains neither pending target page's phase.

All input hashes, metadata and evidence/helper guards pass before either
target page is written. The page at `0x4b1000` has one phase satisfying 13
constraints from seven functions; `0x1b8e000` has one satisfying 16 constraints
from thirteen functions. Their private 4,096-byte artifacts are explicitly
`offline_inferred_reconstruction`, with no runtime/RX or target-execution
claim. Receipt:
`reports/offline-code-page-reconstruction-v1-20261019T101759Z-1867925-36712395f7b4.json`.
The source/loader two-pass budget is 255,702,848 bytes; elapsed time is 0.628
seconds. These reconstructions permit further bounded static analysis, but
do not themselves identify the response status or recover a TACT key.

#### Following the reconstructed response path

The Windows-ABI carrier consumer keeps its first failed version and a
redacted `TypeError` receipt; a separate v2 corrects a string/integer RVA
conversion. Initial v2 analyses include only a primary `.pdata` interval.
Those intervals can end in the middle of a chained function, causing
`halt_baddata` or analysis of synthetic zero padding. Such output is **not**
evidence of bad reconstruction. The separate v3 consumer supplies the whole
authenticated page as context, fixes only its synthetic PE entry-point header,
and retains explicit page-boundary/whole-function limitations.

The reconstructed response adapter at `0x1b8ed40` tail-jumps to `0x1b8b770`.
That destination belongs to the authenticated retained RX control page, so a
new, separately typed carrier analyzes the actual historical page rather
than inventing a runtime function-carve receipt. REA evidence
`ev_a7fcb70d99054f02913f758346158ff6c8afb0af932c75d11891778e6c8029d8`
corroborates the instruction-level state setter: for a found cache entry
whose state is not already 2, callback results **1, 3 and 4 store state 3**.
Results **0 and 2 copy sixteen payload bytes, store state 2 and call
`0x1b8e7b0`**. Other result values leave it unchanged. Thus heap state 3
alone cannot distinguish a local miss, invalidation and a non-public result;
it remains distinct from activation event 3.
Receipt:
`reports/retained-control-page-function-v1-safestatereply7045000-1b8b770.json`.

With full page context, reconstructed `0x1b8e9d0` calls generic query
`0x4baa90`, queues the callback and returns 2 for a pending result, or invokes
it immediately with 0 for a present row and 1 for a missing row. Generic
`0x4baa90` distinguishes an existing row, a per-store sentinel and eligibility
to enqueue a remote request. These are reviewed static paths, not an observed
request/response trace for either selected row. Evidence:
`ev_cd397394dcdffac64cebd9734a1043b4ce31c4c61c9f08e68ba1b0909339da47`
and
`ev_016edda28b55efbb03f626285d42bc73802f74b3fd474b1168c593a23f8bbd37`.

`0x4b1d60` appends callbacks to the vector at `0x6c32418`, using count
`0x6c32420` and capacity `0x6c32428`; the full-context analysis returns normally.
A bounded search of the reconstructed neighboring pages finds readers in
`0x4b10b0`, `0x4b08a0` and generic dispatcher `0x4b0850`. The reviewed paths
broadcast events 0/2 and 1 respectively. A direct call at `0x4b1a56` supplies
event **4**, not 3. This does not yet identify the activation-event-3 emitter.
One REA attempt on `0x4b18a0` failed during provider cleanup because a live
process's ownership environment was unreadable (`EACCES`); retain it as a
failed analysis, without pseudocode evidence. Read-only follow-up found that
process gone and Ghidra available; no permission change or forced cleanup was
performed. Receipts include
`reports/offline-event-vector-xrefs-700-01.json`, receipt 02 and
`reports/rea-offline-reconstructed-page-v3-safeeventtransition7045000-4b18a0.json`.

The operation-indexed label table referenced by reconstructed `0x4b08a0`
is source-file-backed at `0x414b928`. A bounded source-only comparison emits
only allowlisted public enum suffixes, never arbitrary strings. Indices
0–4 match **NotSet, Valid, Delete, Invalid, NotPublic**, sharing the same
unreported normalized prefix. Source hash and metadata are stable before/
after. This corroborates the exact client's name for index 4; tracing the
serializer and the selected rows remains separate from identifying that
label. The first whole-label dictionary attempt, which matched nothing, is
retained alongside the qualified-suffix comparison.
Receipt: `reports/source-status-labels-70450-01.json`.

#### Cache refresh during the investigation

The cache subsequently changes to 3,342,634 bytes, SHA-256
`4acfe8b7771327bf869d0f6331ee93c0378c09a37465fa9c4b74f1124dca4ec6`.
An explicit input-selection record pins that new snapshot for the unchanged
strict version-9/build-70450 parser. All source/integrity guards pass. The
two selected rows now have twelve observations total, all operation 4 with
zero payload, and their latest observations are still empty. The table-wide
totals are 714 empty operation-4 records and the same fourteen operation-1
records with sixteen-byte payloads. These observations still do not count
delivery attempts or supply a new key candidate.
Receipt:
`reports/audit-target-key-cache-status-v2-20261019T103348Z-1792406028015115865-0.json`.

For a new client/tool version, preserve these private receipts and failures.
Repin both executable and loader; independently authenticate new controls;
preregister any candidate code-material region before full-page comparison;
select phases from the declared constraints and require exact control
calibration. Never transfer these phases or byte-acceptance claims to another
build. Supply authentic neighboring page context when unwind chains cross a
primary interval, and retain missing-context limits. Code-recovery material
is not an asset decryption key, and static analysis does not establish that
the server delivered either requested payload.

#### Corroborating the cached operation with the serializer

A further explicitly selected offline batch reconstructs eleven neighboring
pages independently, with the same two exact controls and unchanged phase/
identity policies: eleven successes, zero failures, 14.426 seconds. Together
with the earlier six target pages this gives 17 authenticated inferred pages
(69,632 bytes), not full-image acceptance. Each page retains its own metadata
and reconstruction receipts. Batch manifest:
`private/hotfix-region-recovery-v1-20261019T103923Z-1885215-55ebedee24/manifest.json`.
The v11 authored-source inventory contains 100 diagnostic sources and
corroborates all 86 v10 hashes unchanged.

The new Windows-ABI analysis of serializer `0x4af5c0` succeeds with all final
input guards true. Evidence
`ev_794cdf9966ad5eee3b73dc819be99491f026c2776e93988c4b9677299e3319a5`
shows a 32-byte record header, with the unmodified sixth argument at offset
28 and payload length separate at offset 24. The reviewed caller supplies
the same operation value that indexes the status-name table. A private
comparison confirms the writer's magic matches the cache's record magic;
no literal bytes are exposed. The strict cache parser reads that operation
at the same offset. Receipt:
`reports/cache-operation-serialization-70450-00.json`.

This closes the earlier **unproven numeric-name mapping** for this exact
client's cached operation: the selected empty operation-4 observations are
classified **NotPublic** by the client's own code and enum-name table. They
contain no sixteen-byte key payload. It does not establish the server's
reason, an account entitlement rule, future publication timing or a way to
derive an absent key. The heap-state union and activation-event enum remain
separate facts.

A linear disassembly candidate search across those 17 selected pages finds
only the known direct dispatcher call `0x4b1a56`, passing event 4, plus the
already reviewed callback-vector readers. It does not find the event-3
emitter and makes no global absence claim. Receipt:
`reports/focused-hotfix-dispatch-xrefs-70450-00.json`. Further work must follow
references outside that bounded neighborhood, retaining inferred provenance
and the exact control/identity gates.
