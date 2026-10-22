# A918 installation and recovery evidence

Research date: 2026-10-18. Firmware activation remains disabled. This work uses
local immutable Sony package inputs, static analysis and host-only tests. No
player command, NVP write, reboot, mount, repair or firmware installation was
performed. See [protocol evidence](FIRMWARE_PROTOCOL_PHASE2.md).

## Installation boundary

The normal player receives USB state 2, translates it to internal state 3 and
event 0x15, and state-dependent handlers issue callback 0x0c. Factory 0x1a5918
creates message 0x58; handler 0x75c1c calls the exact-four-byte NVP flag setter
0x1a4ffc and sets the context request flag. These edges are static evidence;
active runtime dispatch and the complete shutdown path remain unverified.

Additional Ghidra/ARM analysis located initialization at 0x1ceaf8: it stores
its argument in global 0x5d9b14, which the event handlers dereference. Caller
0x1cd2b4 at 0x1cd6e0 passes literal 0x541e84 (pool 0x1cdc50). The file-backed
interface table contains 0x1cd204 at offset 4. That wrapper calls the function
pointer stored at 0x5d9a94, where registration 0x1ccd34 places factory 0x1a5918.
This closes the static interface binding without assuming a BSS object layout.
The active runtime state and execution of a particular event remain unmeasured.

Cleanup reached through thunk 0x1a5fd8 leads to 0x1cc4a0, which waits for helper
processes, closes/unlinks FIFOs and conditionally unmounts storage. It selects
0x1d115c or 0x1d11a0 using its argument's field at offset 0x0c. Both call sync
twice, then the imported reboot function. The former uses 0x4321fedc (Linux
power-off); the latter uses 0x01234567 (Linux restart, literal 0x1d11dc). Neither is yet proven to be the
end-to-end successor of message 0x58. Ghidra follows the thunk and may export
the larger destination function under the thunk's name; check ARM instructions
and addresses rather than relying on the export filename.

The persisted flag's boot-time consumer and resident installer are still missing.
The separate apl_conn helper's direct reboot path does not close this gap.

## Failure and power-loss matrix

| Boundary | Evidence | Recovery conclusion |
| --- | --- | --- |
| Host payload preparation | Native offline digest/identity/checkpoint tests | Can reject locally; no physical recovery claim |
| Flag persisted, before reboot | Normal player requires a four-byte NVP write | Boot selection, flag clearing and torn-write behavior unknown |
| Resident installer startup | Installer not recovered | Retry and rescue transport unknown |
| Before/while root formatting | Stock script calls mkfs.ext2 before checking update-file presence | Old-root preservation cannot be assumed |
| After format, missing package or failed mount | Script displays error and exits 1 | No old-root restoration in this script |
| Kernel/initrd programming and root extraction | fwpup feeds tar in one pipeline | Write order, verification and atomicity inside fwpup unknown |
| Pipeline producer fails | Host POSIX reproduction can return success if tar succeeds | Script status alone is insufficient; target shell behavior remains unverified |
| sync/unmount fails | Script displays error and exits 1 | Durability and safe restart not established |
| USB returns | Original Windows worker checks installed version | Enumeration/progress alone is insufficient |

No dual-bank image switch, rollback copy, recovery flag lifetime or independent
rescue loader has been verified. This is an absence of evidence, not proof that
the hardware has no recovery mechanism. Application-level run_emergency needs
a functioning kernel/root/helper stack and is not evidence of recovery after
root formatting. A visible music-volume backup is not a system/NAND backup.

## Local fault reproduction

`python -m unittest discover -s tools -p test_firmware_pipeline.py -v`

Three tests run host Python through a host POSIX shell into host tar **list
mode**, using only an empty synthetic archive. They cover successful producer,
failed producer with valid archive, and failed producer with invalid archive.
No Sony executable/script is run and no archive is extracted. A failed producer
with a valid archive reproduces a successful ordinary pipeline status. This
demonstrates a potential error-propagation gap, not a measured A918 failure.
Existing CI test discovery includes these tests. Missing host shell is reported
as a skip, never physical validation.

The native firmware tests separately retain identity/digest checks, ambiguous
post-intent restart refusal, malformed checkpoint rejection and exact readback.
They cannot establish flash durability or restore damaged device partitions.

## Search for missing recovery inputs

Searches covered exact A918/A910 names with recovery, full dump, NAND, service
manual, fwpup and ICX1101/ICX1087 terms. No verifiable model-matching full recovery
image or resident installer was obtained in this stage. Unrelated Walkman and
Xperia recovery results were excluded. No unverified image was imported.

Primary sources checked:

- [Sony A918 support](https://support.sony.jp/electronics/support/digital-music-players-nw-nwz-a-series/nw-a918).
- [Sony A910 update history](https://www.sony.jp/support/walkman/download/nw-a916_h.html): normal versions 1.01/1.02/1.03, not a recovery-image description.
- [Sony A916/A918/A919 manual](https://www.sony.jp/ServiceArea/impdf/pdf/3283404041.pdf): normal PC update procedure, page 165.
- [Sony A-series troubleshooting](https://support.sony.jp/electronics/support/digital-music-players-other-digital-music-players/nwd-w263/articles/S1110278026242): includes A918 reset/charging guidance; does not establish restoration of interrupted flash programming.

The historical nw-a916_fw.html endpoint did not yield usable content through
the web reader in this stage. The already retained original 1.03 updater remains
an update package, not evidence of a full system recovery image.

## Required evidence before activation work

Recover the exact boot-time flag consumer, partition map and resident installer;
trace fwpup's erase/program/verify order; establish immutable rescue entry and
model-specific image provenance; determine NVP flag clearing and interrupted
write behavior. Any later hardware validation requires separate authorization
and an independently established recovery method. Until then no flash executor,
automatic retry, rollback button or firmware-write UI may be enabled.

## 2026-10-19 offline follow-up

[Device-side installation analysis](DEVICE_INSTALL_CHAIN.md) adds complete normal-initrd/rootfs inventories, resolved logical NVP flag mapping, normal-driver metadata/read/status branches and synthetic power-loss models. It does not recover the resident flag consumer or fwpup, establish a successful rescue path, or authorize hardware access. Earlier evidence and its limits remain preserved.
