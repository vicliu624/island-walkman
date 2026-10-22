# Firmware updater parity and brick-risk audit

Follow-up: [command flow, exact timer/error branches and offline fault replay](FIRMWARE_UPDATER_FLOW.md) extends this initial snapshot without changing its no-flash verdict.

Date: 2026-10-19. Verdict: **NOT equivalent to Sony FWUpdater; NOT approved for firmware flashing.** The current Island Walkman source has no live firmware uploader or activation executor. This is an implementation gap, not a parity pass.

This review covers the dirty workspace based on commit `6e9c06f6cbf3956c4def738a25c4c4ffadcbda23`, application `0.4.0-alpha.4`. [Evidence snapshot](audits/fwupdater-2026-10-19.json) records source hashes, the freshly built CLI hash, original Sony binary hashes and fresh package-inspection results. It does not certify an installed application of unknown provenance, an older wrapper, or a future build.

Only static inspection, host filesystem/loopback tests and offline CLI commands were used. No Sony program was executed or instantiated. No player was queried, opened, mounted, reset or flashed during this audit. The reserved IW identity proposal was not written into firmware. No release asset was changed.

## Immediate interpretation

The current settings UI offers USB inventory and local UPG inspection. The local server exposes firmware safety, inventory and inspection only. The explicit Windows developer command can submit two fixed read-only queries; it has no activation request. CLI firmware diagnostics return before normal catalog/recovery startup. Desktop application self-updates are a separate Tauri feature and do not constitute player firmware updates.

There is no current firmware-write path to compare end to end with Sony. A disabled-looking button or a JSON flag alone would not establish this; the source entry points, request constructors and subprocess call sites were inspected. A new loopback regression exercises the real authenticated HTTP handler with music-write authorization enabled and `force`, confirmation, raw activation CDB and write-enable request fields. Flash/upload/activate/reset/raw/rollback routes return an unknown-endpoint error, no worker starts, and the virtual device tree and catalog state remain unchanged. The safety endpoint remains disabled regardless of those fields. This does not prove future routes or other executables safe.

Sources: [CLI entry](../src/main.rs), [firmware API and regression](../src/service.rs), [Windows request constructor](../src/firmware/windows_readonly.rs), [firmware UI](../ui/src/firmware.tsx), [offline model](../src/firmware/session.rs).

## Reference inputs

The retained originals and the originals in the sibling firmware project were independently hashed and match:

| Input | SHA-256 |
| --- | --- |
| FWUpdater.exe | `3cf7eb2e354d2e6d3f0cddd06ab67755696579a0d2cc85b68672ecc44ccb3c1c` |
| FWUpdaterCom.dll | `2b7550cd945a8d71d139063fce0ce006bfa6b59ee940f17e5151bdb6e35ebe2f` |
| FWUpdaterRes.dll | `0536de4cc4373f2f1391fbb52f11edf47af28fa52551959d569b6488c639b243` |
| Sony 1.03 NW_A91X.UPG | `cfbcc372e2a18f11c12a080c9e5f039333e09175f5db03ac841a4227d3896b32` |
| R14 Disk Apps LOCAL1 NW_A91X.UPG | `7c545418f6b231eb926d1d75742c953f5f9f4abd6a1042069a00b9ae631563a9` |

The official INI has separate A916/A918/A919 sections with the same VID/PID, target 1.03, UPG filename, root write path, `CommandType=0`, `Signature=dbmn` and `Timer=1:40`. The shared firmware relationship is explicit, but exact target identity still matters. Timer is a configured value, not a measured flash duration or proof of timeout semantics.

The public [Sony 2008 update history](https://www.sony.jp/walkman/update/2008.html) lists the shared A916/A918/A919 1.03 update. It is contextual evidence only. The historical detailed updater URL currently returns 404, and the manual URL redirects; missing public pages were not treated as additional protocol evidence. Transport findings below come from pinned local original binaries and retained static decompilation, not assumptions based on general Sony updaters.

## Behavior comparison

| Stage | Official reference behavior / evidence | Current Island behavior | Assessment |
| --- | --- | --- | --- |
| Select target | INI model-specific product entries, common family VID/PID, signatures and command type; EXE configuration/preflight | USB family inventory; explicit Windows query binds USB serial to volume descriptor and then accepts a supported exact model | Partial research match; no physical response parity or full Sony selection matrix measured |
| Read version | DEVINFO numeric revision; EXE `0x00401840` extracts major/minor digits | Strict four-byte numeric decoder; custom declarations interpreted separately | Wire layout cross-checked; custom full-build identity remains unavailable |
| Battery | EXE `0x00404d90` has a conditional threshold; INI lacks RemainingBatteryLevel and parser defaults to -1 | Missing/unknown values remain unknown; live write gate always closed | Intentional stricter policy, not behaviorally identical; actual device status/threshold unverified |
| Validate local package | Official worker checks input existence and configured package; full official package validation not recovered | Bound length, outer MD5, DES/container signature, component bounds/overlaps and SHA-256 | Useful format checks; no publisher authentication, installed-capacity guarantee or bootability proof |
| Stage payload | COM `0x10009640` builds destination and calls recursive copy helper `0x10007a10`; helper uses CopyFileA | No firmware staging/copy implementation | Missing; not equivalent. Fresh hashes must be read back from the staged device file before any future activation |
| Activate | COM copy branch with CommandType 0 constructs a 12-byte FC subcommand 4 request after copy; gadget `fsg_main_thread` validates selector, syncs storage and notifies state 2 | No request type, endpoint or executor for that command | Missing and intentionally unreachable. A no-data request can still mutate device state |
| Flag/shutdown | Static device paths lead toward NVP fWUp flag, sync and shutdown/reboot; exact active runtime chain remains unmeasured | No firmware NVP write or reboot implementation | Missing; no end-to-end replay or trace establishes equivalence |
| On-device install | Original script formats system root, mounts it and invokes fwpup/root extraction | No host-side installer; a future native activation would still enter the device installer | Official installer risk is inherited if invoked; host parity cannot make it transactional |
| Monitor reconnect | EXE `0x00408450` polls disappearance/reappearance and retries reads | Offline transitions only; no physical poll/timeout executor | Missing; exact timing/sense/timeout matrix is not validated |
| Confirm success | Worker reads numeric installed version and compares it with target; progress is not sufficient | Offline model requires exact identity and numeric target readback; rejects same-version/downgrade | Similar intent with stricter policy, but not a live implementation; cannot distinguish two modified 1.03 builds |
| Retry/cancel/restart | Several official branches exist; no complete measured retry/cancellation contract or physical recovery proof | Offline model forbids cancel/resend after activation intent and restores ambiguity into Review | Conservative design only; no durable on-disk firmware intent writer, recovery executor or proven rollback |
| Concurrent writes | COM has internal synchronization; exclusion with unrelated host applications not established | Shared host file lock used by transfer, firmware diagnostics and app installation | Coordinates participating Island processes only. Does not block official FWUpdater, Explorer, another user/host or other software |

Primary static evidence: [original copy/activation](research/firmware-updater/rea/com-update-worker.json), [copy helper](research/firmware-updater/rea/com-firmware-copy.json), [original EXE worker](../data/firmware-analysis/phase2-native-updater-worker.c), [version readback](../data/firmware-analysis/phase2-post-version-read.c), [protocol analysis](FIRMWARE_PROTOCOL_PHASE2.md), [device installation/recovery chain](FIRMWARE_RECOVERY.md).

## Release-blocking findings

1. **No executable parity implementation.** The pure Session state machine has no hardware transport or durable filesystem journal writer. Calling payload_verified with a supplied digest is not a readback of a real device file. Its successful unit tests cannot qualify an uploader that does not exist.
2. **The original installer has a destructive nontransactional boundary.** Fresh offline DES decryption of the pinned stock package reproduced first-component SHA-256 `60aedd9213cd6178edcc92a1f168d3e9efda3d7af21292c7d3ea8bbceae713d4`. It formats `/dev/mtdblock/1` before checking the expected package file under `/contents`, then performs fwpup into tar. Error-display/exit branches do not restore the old root. Exact fwpup erase/program/verify order and its boot-time launcher remain unverified.
3. **Pipeline failure can be hidden.** The script checks the shell pipeline status after the producer feeds tar. Three freshly rerun host-only tests reproduce producer failure masked by a successful consumer. This is a host POSIX reproduction of a possible failure mode, not proof that it occurred on the A918 or caused the reported failure.
4. **Custom build completion cannot currently be proved.** Same numeric base version is insufficient; IWnn is only a proposed declaration and is not emitted by a validated producer. Full build/ABI readback remains absent. Do not change numeric Sony revisions or internal `/version` merely to defeat version checks. The script's `/version` is a date-like internal value compared with `20070508A`, distinct from displayed 1.03. Any subrevision modification requires rechecking official selection/update behavior.
5. **No independently established recovery contract.** Boot-time flag consumption/clearing, interrupted flag writes, resident recovery, flash verification and a usable rescue channel have not been established. The normal application emergency path needs working system components. Music-volume backups are not firmware rollback images.
6. **Same first three components does not make the modified root safe.** Fresh native inspection found stock and R14 LOCAL1 script/kernel/initrd component hashes identical, with different rootfs hashes. The retained R14 allocation report is bound to this freshly hashed rootfs; it reports 5,267,456 free bytes in a 48 MiB model. That allocation simulation was not rerun in this audit. Actual geometry, NAND condition, runtime writes, power-loss behavior and bootability remain unmeasured. Capacity headroom addresses one risk, not all brick mechanisms.

These findings do not establish the cause of the earlier failed R12 update. There is no new physical trace or failure-state dump in this audit that attributes that failure to a particular host updater or installer boundary.

## Required evidence before a live executor is enabled

1. Establish the complete original host sequence for the actual A91x configuration: selected COM method/vtable route, configured staging filename and its relationship to device ufn, close/flush semantics, activation CDB, sense/status acceptance, timing, timeout, reconnect and final checks. Replay available captured traces against a transport double. Static branches alone are not a measured transcript.
2. Separate required compatibility behavior from deliberate stricter policy. Make every difference explicit: no unsafe automatic retry after ambiguous activation, exact target binding, mandatory staged-payload verification, power policy and full custom-build readback. Do not blindly duplicate an official weakness to claim parity.
3. Bind every candidate to exact package/component hashes, intended family, validated metadata and complete rootfs allocation/headroom evidence. Preserve originals and prior candidates. No UPG filename or compressed size can establish installed capacity or provenance.
4. Identify the resident installer/fwpup behavior, real partition constraints and a matching-model independent recovery path before hardware flashing is considered. Simulated recovery is insufficient.
5. Add a durable intent-before-send executor with process-restart interlocks; prevent music/app writes while firmware activation is unresolved, including after the OS lock is released by a crash. Test partial copy, short writes, disk full, cable loss, ambiguous activation, driver/sense errors, unexpected reconnect, wrong/old build readback and corrupted journals with fakes first.
6. Only after those blockers are resolved, separately authorize a narrowly scoped hardware validation plan on an appropriate recoverable device. Do not use the currently failed-update device or perform speculative physical recovery/probing as part of this audit.

## Validation completed

- Fresh `cargo test --no-default-features`: 107 passed, 1 intentionally ignored. Includes fixed DATA_IN request whitelist/layout/error handling, firmware state/checkpoint ambiguity tests, device-operation exclusion and the new real HTTP mutation-route rejection regression.
- Fresh `cargo build --no-default-features` succeeded. Existing no-desktop dead-code warning remains.
- Fresh `python -m unittest discover -s tools -p test_firmware_pipeline.py -v`: all 3 host-only tests passed.
- Fresh CLI `--firmware-safety` returned firmware writes and raw vendor queries disabled.
- Fresh CLI `--inspect-firmware` on pinned stock and R14 LOCAL1 packages verified format and component comparison without device access. Official PE hashes match their preserved package copies.

No hardware flash qualification or official/native USB trace equivalence was obtained. The operational decision is to keep native firmware activation unavailable and prioritize closing these evidence gaps over adding firmware features.
