# A918 firmware protocol: response layouts and failure boundaries

This stage adds response decoders, a Windows diagnostic reader restricted to two
fixed data-in requests, and an offline transaction model. It does not implement
firmware upload, activation, reset, NVP writes, automatic recovery or a flash UI.
The application still refuses firmware writing. No physical vendor request was
sent during this stage: the diagnostic attempt stopped at USB identity matching
because the A918 was absent. The user then explicitly chose local testing only.

## Evidence and coverage

REA/Ghidra analysis of the original Windows PE inputs continued from the hashes
in [Firmware research](FIRMWARE_RESEARCH.md). New evidence is retained locally as
`docs/research/firmware-updater/rea/phase2-*.json`. Binary input bytes were not
modified and the Sony updater was not run or instantiated.

The embedded COM type library was independently read with Windows
`LoadTypeLibEx(REGKIND_NONE)` and ITypeInfo record metadata. This reads type
descriptions without COM-class activation or type-library registration; no Sony
updater methods were called. The retained layout is
`data/firmware-analysis/typelib-layout.txt`.

ARM device-side analysis used separate static Ghidra headless, because Windows
REA's Ghidra P0 rejects ARM ELF. These results must not be attributed to a
successful REA ARM analysis. The additional inputs are:

| Input | SHA-256 |
| --- | --- |
| icx1087_usbconn.ko | `26b57aad0efd8e0fb4653421be4025a497f3e645b9262cb9c0bff250636323e2` |
| apl_conn | `37580d0393de4f548718a7b5fdb1b58dc3310edde37582dace172e9b54de5af7` |
| gemini-ICX1101 | `a5c94ce492fb7ec9cd7a740d30cb79a1c2b43ee782e2da036a72f57dabdcd177` |
| Normal initrd component | `4c11d0c396dff18465ae1d24aa0692edd5ecb14ca5790232f7fdba92c8ae4252` |

The stripped Gemini application's `FWUPDATE` string alone did not resolve the
handler. Subsequent constant and device-path cross-references found partial
normal-application evidence described below. Ghidra reported function recovery
warnings in other parts of the binary. No complete normal-application-to-resident
installer graph is claimed; missing recovered references are not absence proof.

## Device information and firmware version

The MSC information request is:

```text
FC 00 20 64 62 6D 6E 00 80 00 00 00
```

The response is exactly 128 bytes, begins with seven ASCII bytes `DEVINFO`, and
has a one-byte size of 128 at offset 7. All offsets are zero-based. COM record metadata and the device
`vscDpccProcessDEVINFO` handler agree on the field offsets:

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 16 | 8 | Vendor identification |
| 24 | 16 | Product/model identification |
| 40 | 4 | Product/firmware revision |
| 44 | 4 | Product subrevision |
| 48 | 4 | Storage-size bytes; units remain uninterpreted |
| 52 | 16 | Device serial bytes; relation to USB serial remains unverified |
| 96 | 32 | Vendor-specific data; not used to authorize installation |

REA decompilation of updater `0x00401840` establishes the version interpretation:
it reads the revision at response offset 40, uses byte 0 as the major digit and
bytes 2/3 as the decimal minor digits, and constructs a packed numeric value.
The Rust decoder conservatively requires the complete `digit.digitdigit` form,
validates the header/length and rejects malformed ASCII/padding. It never reads
the installed firmware version from USB bcdDevice.

Raw device serial bytes and capacity bytes are retained as hex. No conversion
to the USB serial, capacity units or model compatibility is guessed.

## Power information

REA COM function `0x10008a30` constructs:

```text
FB 00 00 50 57 5F 53 54 41 54 20 00
```

Its 32-byte response starts with `PW_STAT`, followed by size 32 at offset 7.
The type library records:

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 9 | 1 | Raw power-supply status |
| 10 | 1 | Raw battery status |
| 11 | 1 | Battery life percentage |
| 12 | 4 | Battery life time, big-endian on the wire |
| 16 | 4 | Full battery life time, big-endian on the wire |

The COM function applies ntohl to the two time values. Percentages 0 through 100
are decoded numerically; 255 remains unknown and other out-of-range values are
rejected. All-ones time fields remain unknown. Supply/status bytes are preserved
without assuming that a USB connection or charging flag authorizes an upgrade.
Device acceptance of this query and real response values have not been measured.

REA updater function `0x00404d90` checks battery percentage only when the parsed
RemainingBatteryLevel field is positive. `0x00403540` initializes it to -1 and
the INI reader uses -1 as the absent-field default. The researched A91x INI has
no RemainingBatteryLevel entry. This conditional legacy behavior is not a safe
default policy to copy. The native firmware gate remains closed even for a
synthetic 100-percent response: model-specific battery policy and recovery
conditions are still missing.

## Windows diagnostic boundary

The explicit diagnostic command is development-only and is not called by the UI
or background scanning:

```powershell
island-walkman.exe --firmware-readonly-probe F: --usb-serial DEVICE_SERIAL
```

It performs these checks in order:

1. A drive-letter-only argument and an explicit USB serial are required.
2. Exactly one enumerated Sony 054C:032A device must match that USB serial.
3. The volume's STORAGE_DEVICE_DESCRIPTOR must identify USB storage and the
   same serial. Descriptor size/version, bounds and termination are checked. Missing,
   differently encoded or mismatched serials stop the
   operation; no heuristic binding is performed.
4. The SCSI address is queried, and only LUN zero is accepted. The volume binding
   is rechecked before each fixed information query.
5. The request constructor permits only DeviceInfo or PowerStatus. Every packet
   is DATA_IN, CDB length 12, at most 128 response bytes, with a five-second SCSI
   timeout. The aligned buffer, x64 structure offsets, SCSI status and returned
   lengths are validated. No retry loop or arbitrary CDB argument exists.
6. The decoded information must identify SONY / NW-A918 before power is queried.
   A power-query failure is reported as unknown, never as a ready battery.

Windows SPTI requires read/write **handle access** for this ioctl. Opening an
existing volume with those access rights is not a file write: the implementation
contains no data-out request or filesystem writing operation. It does not elevate,
lock/dismount the volume, install a driver or change device mode.

The initial diagnostic attempt failed the USB-presence check, before volume
opening or vendor-query submission. There is no live model/version/battery
measurement from this stage. Linux SG_IO and macOS storage-query adapters are
not implemented; those platforms retain USB inventory and offline parsing only.
The generic/raw vendor-command gate remains closed. The explicit Windows reader
is separately identified in the safety report as an available fixed read-only
diagnostic, not a firmware activation capability.

## Activation and installation chain

The evidence now establishes these separate pieces:

1. Host COM `0x10009640` copies the configured firmware and constructs activation.
2. USB gadget `fsg_main_thread` at `0x128fc` validates the subcommand/signature,
   synchronizes storage, changes its state and notifies `usbconn_chg_sts`.
3. `icx1087_usbconn.ko` function `usbconn_chg_sts` at `0x108a0` stores the notified
   state and wakes a consumer; ioctl `0x80046701` returns its state.
4. In the separate MSC helper `apl_conn`, main at `0x87f0` observes state 2 and
   calls `setfuflag(1)`. That function at `0x8d88` opens the NVP device with index
   9 and writes the 32-bit value `0x70555766` (little-endian ASCII `fWUp`). On a
   successful return, the helper sleeps two seconds, syncs and calls reboot.
5. The original package's installation script checks /version, formats the
   system root partition and invokes fwpup to write/extract components. It has
   explicit error-display/exit branches for mount, format, missing file and
   updater failures. Those branches do not restore the formatted old root.

Step 4 is a verified **helper path**, not proof that the normal stripped Gemini
application uses an identical sequence. Additional normal-application evidence:

- Gemini `0x1a4ffc` opens `/dev/icx1087_nvp/9`, writes the same flag and requires
  an exact four-byte write before setting a context request field.
- Its caller `0x75c1c`, event case `0x58`, invokes this setter and sets that request
  field again. This is not yet a demonstrated immediate reboot call.
- Gemini `0x1d0f24` reads ioctl `0x80046701` but translates raw state 2 into its
  own state 3. Receiver `0x1cd2b4` then maps that translated state to event `0x15`
  and queues it through `0x1ced34`.
- Factory `0x1a5918`, case `0x0c`, allocates an object with message ID `0x58`
  using constructor `0x1a6100` and submits it through `0x1b24ac`.
- Several state-dependent dispatch handlers (`0x1cf894`, `0x1cfa08`,
  `0x1cfc70`, `0x1cfdc8`) map event `0x15` to callback argument `0x0c` at
  interface offset 4. Registration `0x1ccd34` writes the factory pointer to a
  callback slot. The active runtime dispatch mode and complete interface-object
  binding have not been independently validated. These are separate conversion
  layers, not a linear numeric offset or a complete verified install graph.

The resident loader/installer that
consumes the persisted flag has not been recovered or verified. The normal
initrd image contains BusyBox and a small driver set; its existence does not
establish a complete recovery installer. The recovered normal kernel's embedded
initramfs spans `0xc001fca0` to `0xc001fd26`: 134 compressed bytes, expanding to a
512-byte CPIO. This small archive is not evidence of a resident recovery toolset.
No NVP flag was changed, helper run,
script executed or reboot requested on a player.

The original Windows worker `0x00408450` polls disappearance/reappearance with
sleep/progress messages and eventually calls `0x00401840` to read the installed
revision and compare it against the target. Progress 100 or USB reappearance
alone is therefore not completion evidence. Native modeling retains this
readback requirement and adds stricter identity/ambiguity handling.

## Recovery limitations

Follow-up [installation/recovery evidence](FIRMWARE_RECOVERY.md) closes the
static interface-table binding described above, records reboot cleanup evidence,
and adds a host-only pipeline-failure reproduction. Resident recovery remains
unverified and firmware activation stays disabled.

The root filesystem's `/etc/init.d/gemini` has run_emergency for a missing or
failed Sony application. It starts MSC helper behavior, or a WMT rescue-driver
branch when configured for WMT. load_usb_epson contains a shared rescue PID
0324; that constant alone is not proof of an A918 brick-recovery mode. The AAD
path depends on an operational kernel, root scripts, drivers and helper files.

This is application-level fallback. It does not establish recovery after root
formatting, interrupted kernel/flash programming or power loss. reboot.sh also
contains destructive repair behavior for a particular application exit code;
it was read as text and must not be used as a generic recovery command.

No verified full-system recovery image, resident-loader recovery contract or
power-loss recovery procedure is available from this research. Firmware
activation remains disabled for that reason, independently of battery readings.

## Offline native transaction model and fault tests

The Rust model has these logical phases:

```text
Prepared -> PayloadVerified -> ActivationIntent -> AwaitingReconnect
         -> AwaitingReadback -> Completed
```

Invalid identity/digest/readback or ambiguous post-intent transport failure leads
to Review. Cancellation is allowed only before intent. Checkpoints carry a
schema, immutable candidate digest, identity snapshot, generation, intent and
readback evidence inside a checksummed envelope. Session has no public mutable
fields and cannot be deserialized except at the validated restore boundary.

Restoring any unfinished post-intent checkpoint leads to Review, not an
automatic resend. A Completed checkpoint requires recorded exact identity and
target-revision readback evidence. Same-version installation and downgrade are
rejected: a second 1.03 image cannot be distinguished from the old 1.03 image by
version equality alone. Modified variants need additional installation evidence.

This model is **offline**: there is no production uploader, activation executor,
durable filesystem journal writer or rollback executor attached to it. The
checkpoint codec and simulated restart tests do not prove filesystem durability
or physical flash recovery. Any future executor must persist and sync intent
before transmission and must never fall back to an older pre-intent checkpoint
after an ambiguous activation. No current endpoint can execute these phases.

Local faults cover truncated/corrupt frames, unknown/invalid power values,
Windows alignment and layout, wrong volume descriptors, nonzero LUN, failed and
short driver responses, changed image/identity, interruptions at every modeled
boundary, wrong reconnects, old/wrong readback, corrupted/truncated checkpoints,
missing completion evidence, and cancellation after intent. No fault is injected
into a physical player.

Additional read-only developer commands are:

```sh
island-walkman --firmware-safety
island-walkman --inspect-firmware-response devinfo /path/to/capture.bin
island-walkman --inspect-firmware-response power /path/to/capture.bin
```

They exit before application catalog loading or music-transfer recovery.
Multiple firmware operation flags are rejected before any device or file access. Capture
results are explicitly labeled supplied_capture_not_live_device. Synthetic test
captures are never presented as live measurements or safe-to-flash evidence.

## Validation of this source revision

Application source commit `8ac38d630c958df78e1736f18cd9a68ba4c6e293` passed all
four jobs in [CI run 39120369850](https://github.com/vicliu624/island-walkman/actions/runs/39120369850):
Windows x86_64, Debian x86_64, macOS ARM64 and macOS Intel. Windows has 20 firmware
unit tests; the shared decoder/model tests run on the other platforms. Local
Windows full Rust validation passed 90 tests with one intentionally ignored GUI
fixture exporter. Eight Python packaging tests, the firmware CLI smoke, native
desktop smoke, preparation/server smoke and an authenticated safety-API check
also passed. UI translations remain the existing 214 complete message sets.

CI and local smokes do not invoke the physical diagnostic command. Tests use
synthetic captures and modeled faults. The absent-device attempt submitted no
vendor command; the user chose to keep the player disconnected afterwards.
There is no physical response, firmware installation, rollback or power-loss
validation result in this stage. No new Release was published.
