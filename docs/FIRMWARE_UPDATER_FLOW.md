# Official FWUpdater command flow and offline fault replay

Device-side follow-up: [upgrade flag, normal initrd, NAND boundaries and offline power-loss models](DEVICE_INSTALL_CHAIN.md).

Date: 2026-10-19. This follows [the first parity audit](FIRMWARE_UPDATER_AUDIT.md). **Native firmware upload/activation stays unavailable. No device testing is authorized or performed.**

## Scope and evidence strength

This work reconstructs the original A91x `ClassifyType=5`, MSC, regular-file, `CommandType=0` path on mainstream Windows NT. It adds static PE/COM binding verification, source-derived symbolic replay and fault tests. It does not execute the Sony EXE/DLL, emulate its CPU instructions, run Sony scripts, replay an observed USB capture or emulate the resident flash installer. Successful symbolic tests are not proof of safe flashing.

The EXE, COM DLL and resource DLL remain the pinned originals from the first audit. Fresh Ghidra 12.1.4 static imports were created under `data/firmware-analysis/flow-20261019/`; original files were not modified. Ghidra exports retain requested/actual function entries, recovered bodies, references and decompiler warnings. LLVM instruction exports provide independent checks where decompilation omits a branch or return value. [Static facts and original instruction ranges](audits/fwupdater-flow-2026-10-19/static-expanded/facts.json) are retained in the repository without bundling executable Sony binaries.

The pure [symbolic model](../tools/firmware_research/fwupdater_model.py) imports only dataclasses and has no file/device/network/driver/process API, real sleep, or injectable transport. The [golden event sequences](../tools/firmware_research/golden_traces.json) were derived from the instruction branches and manually specified. They are not captured device transcripts. [Final replay results](audits/fwupdater-flow-2026-10-19/replay-final.json) bind model/test/fixture hashes and preserve 28 scenarios. [Tests](../tools/test_fwupdater_flow.py) additionally verify constants against the pinned static facts and hashes of retained instruction exports.

## Active COM binding: closed statically

`FWUpdater.exe:0x00403260` obtains the main COM interface and queries interfaces using IIDs at `0x0041c39c`, `0x0041c3ac` and, through `0x00402fa0`, `0x0041c3bc`. Reading the COM map from the original DLL proves the corresponding object offsets 4, 8 and 12. Constructor instructions at `0x10006ec3` assign the matching vtables. A second object variant has different vtable locations but the same relevant method addresses; no class-instantiation runtime transcript is claimed.

| EXE global | Interface / object offset | DLL vtable | Relevant slot and method |
| --- | --- | --- | --- |
| `0x00422764` | updater / +4 | `0x1002f74c` | +0x0c power `0x10008a30`; +0x1c cleanup `0x1000b6e0` |
| `0x00422768` | transfer / +8 | `0x1002f738` | +0x0c upload wrapper `0x1000b600` |
| `0x0042276c` | device / +12 | `0x1002f704` | +0x0c enumerate `0x1000e830`; +0x14 reopen/snapshot `0x1000d4f0`; +0x2c DEVINFO `0x1000a180` |

The actual MSC upload route is:

```text
EXE 0x00408450 (worker)
  -> EXE 0x00408150 (dispatch, initial snapshot revalidation)
  -> transfer vtable +0x0c
  -> DLL 0x1000b600 (critical-section wrapper)
  -> DLL 0x10009100 (classify/space/measure/copy/activation)
  -> DLL 0x10007a10 (CopyFileA helper)
```

This extends the first audit: `0x10009640` is a related generic helper reached by other methods, not the complete configured MSC transfer route. `0x1000b350` uses a default `MSFWUPGR.UPG` path and must not be substituted for the A91x configured transfer route. The actual INI supplies `\NW_A91X.UPG`.

## Ordered host flow

1. Read the INI's three model records. A916/A918/A919 share VID 054C, PID 032A, firmware file, numeric target 1.03, command type 0, `dbmn` transport selector and timer 1:40. This selector is distinct from the encrypted UPG family signature `5dfeff3c`.
2. Enumerate and inspect candidates. EXE `0x00404d90` uses the primary MSC path. A fallback is conditional on ClassifyType 6; these ClassifyType 5 records do not universally fall back to that branch.
3. Compare the returned product name with DevicePropertyProductInfo. For the successful MSC read slice, compare SubIdentification with four vendor-specific response bytes at DEVINFO offset 96. This INI requires zero. This is distinct from ProductSubRevision at offset 44; do not confuse those fields when developing custom version branding. Header/length hardening in our decoder is a separate policy, not assumed to exist in every original function.
4. Require a single match: no match reports `0x80042200`, ambiguous matches report `0x80042201`. Select the highest configured target version through `0x00403eb0` and read the current numeric revision through `0x00401840`.
5. Perform conditional battery preflight. The original threshold default is -1 and the real INI has no RemainingBatteryLevel entry. The battery query/comparison is reached only when the threshold is positive. Missing policy therefore does not establish adequate power. The raw legacy comparison does not independently reject 255 as an unknown percentage; our native decoder does.
6. Select a version confirmation. Instruction tail `0x0040798d` chooses resource 202 for upgrade, 203 for same version and 204 for downgrade. The original resource DLL explicitly asks whether to continue in the latter two cases. IDOK permits continuation; cancellation does not. The native offline Session currently rejects same-version/downgrade instead. This is a deliberate stricter policy difference, not equivalent behavior.
7. Enter worker `0x00408450`; take a GetTickCount timestamp **before dispatch/upload**. Reopen and compare the 128-byte COM enumeration snapshot against the saved snapshot in `0x00407d40`. This object snapshot is not being equated with the DEVINFO wire frame or with a cryptographically bound serial.
8. In `0x10009100`, classify the source, query available space on the mounted data volume, measure source bytes, and require available bytes >= `size + floor(size / 20)`. Empty sources fail. A regular file uses FWPackageWritePathName; the directory path is distinct and outside the current replay.
9. Copy through `0x10007a10`. The file case invokes CopyFileA with overwrite permitted. Copy failures map to HRESULTs described below and prevent this worker from reaching activation. No staged SHA-256 readback or system-root allocation check was found in this copied-file branch. This is not a claim that fwpup performs no checks later.
10. Only after copy success, construct the fixed 12-byte no-data activation command. In this configuration its symbolic bytes are `FC 00 04 64 62 6D 6E 00 00 00 00 00`. It is not implemented in our application's request types or endpoints. The original worker has particular false-return/status exceptions, described below.
11. Wait for disappearance, then reappearance. The same worker-start elapsed budget covers copy and these waits. There is no new full timeout starting after upload. The host also calls `0x00402450`, which updates an existing Windows registry version cache; that bookkeeping call is not installed firmware readback or a success guarantee.
12. Reopen and compare the saved COM snapshot, up to ten attempts with a 300 ms sleep before each. The worker displays 100% during this stage. It has a special Windows Me OS-code-4 branch; the mainstream NT replay does not silently substitute for it.
13. For regular files, sleep one second, reopen/revalidate again, then try a filesystem stat up to ten times with one-second sleeps after failures. Exhausting the stat loop itself does not abort the next cleanup call. Cleanup `0x1000b6e0` -> `0x10009920` deletes the configured staged file.
14. Only **after deletion**, read the installed numeric version. A mismatch raises `0x80004005`; a matching result posts message `0x401`. Read failure, deletion failure or identity/reopen failure can occur after progress reached 100%. No complete custom-build success check exists.

Device-side continuation from the activation state through the normal app, persisted flag, resident installer and fwpup remains subject to the gaps in [installation/recovery evidence](FIRMWARE_RECOVERY.md). The host call chain above does not close that boot-time gap.

## Three separate timing layers

| Layer | Recovered behavior | Interpretation and replay boundary |
| --- | --- | --- |
| Internal volume handle open | `0x100010e0`: up to ten CreateFileA attempts without an explicit sleep | Not ten seconds. Blocking call/driver timing remains external to the symbolic model |
| Default SPTI command timeout | Constructor at `0x100010a1` sets member +0x10 to 5; SPTI worker copies it to TimeOutValue | Default five seconds per request, not the installation deadline. Actual per-instance overrides/driver timing not measured; [byte evidence](audits/fwupdater-flow-2026-10-19/transport-defaults.json) |
| UI timer and reconnect deadline | Config parser stores Timer in seconds; default TimeOut is `Timer + (Timer >> 1)` | Actual INI yields 100 seconds and 150 seconds. Worker timestamp begins before copying |

Disappearance wait has at most ten one-second sleeps, with presence polling beginning after the sixth sleep, plus a final check. A device that never disappears can fail this stage after about ten simulated post-copy seconds even though the configured outer timeout is 150 seconds. Individual blocking calls can add real time; these are not physical timing guarantees.

Reappearance wait checks `elapsed > timeout`, computes progress capped at 99, sleeps two seconds, then polls. It can accept a reappearance found by a poll that finishes beyond the nominal deadline, because there is no second deadline check before leaving that loop. Later open/stat/cleanup/version operations also run outside that loop's timeout test. Thus 150 seconds is a soft loop budget, not a strict total wall-clock bound.

Unsigned 32-bit GetTickCount subtraction wraps consistently for the modeled short interval. The replay verifies a wrap during the normal scenario. It does not model a 49-day operation or suspend/resume semantics.

## Error and ambiguity table

| Trigger | Official selected-path behavior | Proposed native policy / actual implementation status |
| --- | --- | --- |
| No source / empty source | Missing source returns `0x80047003`; empty measured size returns `0x80004005`; no activation | Reject before device mutation; no production firmware staging executor exists |
| Data volume below size + 5% integer allowance | `0x80047008`, no activation | Separate data-staging space and system-root installed-allocation gates; neither alone proves flash safety |
| CopyFileA/GetDiskFreeSpaceEx/cleanup error 112 | `0x80047008` | Preserve exact operation/error context |
| Error 5 | `0x80047007` | Preserve exact operation/error context |
| Other copy/file cleanup errors | `0x80047006` | Preserve original error and secondary cleanup errors separately |
| Upload HRESULT negative, classified regular file | Worker attempts cleanup before posting the error | A cleanup failure can replace the propagated error. Future native execution should preserve both and must not delete after an ambiguous activation intent |
| Activation helper returns false, drive index invalid | `0x80047002` | Outcome after sending must be treated as ambiguous until reconciled; no blind resend |
| Activation false, packed result has masked bits zero | Worker may continue | Preserve as an explicit legacy difference; a false transport result alone cannot prove activation did not happen |
| Activation false with packed `0xff020400` | Explicitly tolerated; other nonzero masked cases raise `0x80047001` | Packed field interpretation/runtime source still needs verified response traces; do not generalize this exception or automatically resend |
| Never disappears | `0x80042300` | Observe/review, never claim success from a progress value |
| Never reconnects | Timeout message `0x403`; retained package is not automatically rolled back | Keep durable unresolved intent and block other Island device writes across process restarts; current production executor/interlock is absent |
| Reconnect found, ten reopen/identity checks fail | Propagates the failed HRESULT; example simulation injects `0x80047002` or snapshot mismatch `0x80004005` | Reject wrong/unknown device identity; retain evidence |
| stat fails ten times | Continues to DeleteFileA; subsequent delete may fail | An inability to inspect a staged file is not proof of safe deletion |
| Delete fails after 100% progress | Terminal error can occur before final version read | No success notification before full readback; preserve evidence until confirmed |
| Final numeric version wrong/unreadable | Failure after staged file may already be removed | Require exact installed custom build identity, not just base 1.03; current full-build channel remains absent |
| Copy bytes corrupted while CopyFileA reports success | No host staged-hash comparison in this helper | Replay supplies a hypothetical matching numeric version to demonstrate the missing host check; it does not predict that real hardware accepts corrupt data. Native policy must require staged readback hash |

The CDB bytes, packed status and raw battery examples are data fed only to a pure model. No generic CDB submission code was added. The policy oracle returns requirement/review decisions in memory and is not connected to production hardware or endpoints.

## Why instruction verification matters

The fresh Ghidra output still recovers `0x10007a10` as a void function and reduces an error tail to GetLastError. The original instructions at `0x10007da7..0x10007dcc` explicitly map Win32 112/5/other errors to three HRESULTs and return through the epilogue. Ignoring that tail would incorrectly conclude that copy errors are swallowed. The retained instruction exports and error-map tests prevent that analysis mistake.

Likewise, the same-version/downgrade decision tail at `0x0040798d` sits outside a misleading recovered function body. Static resource decoding plus those instructions establishes the actual prompts. Decompiler output is evidence to inspect, not an infallible original source listing.

## Offline coverage and remaining gaps

23 new unittest methods cover successful ordering, five manually specified golden event traces, product/subidentification selection, zero/multiple matches, version prompts, missing/empty source, data space thresholds, source measurement failure, three copy errors, cleanup masking, unverified staged hashes, activation-status exceptions, default/conditional battery policy, wraparound, failed/incorrect reconnect, premature 100%, ten stat failures, unresolved-intent policy and purity/no-transport constraints. A bounded 110-scenario timing sweep verifies termination, at most one symbolic activation and no hardware-enable result. The persisted replay artifact contains 28 selected cases with event/time histories.

All 39 Python tests pass, including the existing three host-only pipeline failure reproductions. Rust tests pass 107 with one intentionally ignored fixture exporter; the existing authenticated API regression still rejects firmware mutation routes. Two fresh offline native CLI checks (`--firmware-safety`, including with `--allow-device-write`) also kept firmware/raw-query gates closed and created no catalog; see [native gate evidence](audits/fwupdater-flow-2026-10-19/native-gate.json). No application behavior changed in this follow-up, so the desktop version remains 0.4.0-alpha.4. This work changes research tools, tests and documentation only.

Still unproven:

- No observed official USB/driver transcript or real response timing; static/source-derived replay is not a differential run of both executable updaters.
- Exact actual-device storage geometry, flash state, power policy and complete DEVINFO/USB/volume identity semantics.
- Full directory, MTP, command-type-1, Windows Me and UI cancellation/exception state-space coverage. These branches are explicit gaps, not silent successes.
- Boot-time persisted flag consumer/clearing, resident installer behavior, fwpup erase/program/verify order, power-loss behavior and an independent matching-model rescue route. A fresh filename inventory of the available extracted rootfs and current initrd research directory found no fwpup executable; this limits the available inputs and is not proof that no such component exists on the device.
- Full installed custom build readback and durable native firmware intent/recovery executor.

The next offline priority is the resident installation/fwpup chain and a complete error/status interpretation from pinned device-side code, alongside any already available captured transcripts. Live activation must remain unreachable until those blockers and the original audit's release criteria are satisfied. This document authorizes no device access, flash experiment, speculative hardware recovery or release publication.

## Reproduce without a player

Run portable model and fault tests from the repository root:

```powershell
python -m unittest discover -s tools -p test_fwupdater_flow.py -v
python -m unittest discover -s tools -p 'test_*.py'
python tools/run_fwupdater_replay.py --output data/firmware-analysis/next-offline-replay.json
```

The replay writer requires a new output file. Static collection additionally requires the preserved original inputs and LLVM (no dependency on them in ordinary CI tests):

```powershell
python tools/firmware_research/collect_static_flow.py --exe data/firmware-analysis/FWUpdater.exe --dll data/firmware-analysis/FWUpdaterCom.dll --resources data/firmware-analysis/FWUpdaterRes.dll --ini ../sony-nw-a918/work/updater-1.03/NW-A91X.ini --objdump 'C:/Program Files/LLVM/bin/llvm-objdump.exe' --output data/firmware-analysis/next-static-flow
```

The collector refuses changed binary hashes, unexpected COM bindings/configuration and an existing output directory. It launches only llvm-objdump, passing the original Sony binaries as data inputs; it never loads or executes them. [ExportFlow.java](../tools/firmware_research/ExportFlow.java) is the optional static Ghidra export script; intermediate projects and decompilation logs remain under data/firmware-analysis rather than release directories.
