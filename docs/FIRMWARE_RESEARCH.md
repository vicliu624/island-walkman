# Native A918 firmware research

This is an unreleased, read-only first stage of replacing Sony's Windows updater.
It is not a firmware installer. The application contains no firmware write,
arbitrary vendor-command sending, reset, detach, upgrade activation, or updater-launch
implementation. No Sony executable, firmware image, theme or commercial ROM is
bundled with Island Walkman. The current extension adds a Windows-only explicit
reader restricted to two fixed data-in queries, plus offline response decoding
and transaction modeling. See [Protocol phase 2](FIRMWARE_PROTOCOL_PHASE2.md) for
its exact boundary, failed presence-only diagnostic and recovery limitations.

## Implemented boundary

Settings and help contains a Device firmware card in English, Simplified Chinese
and Japanese. Inspection is explicit and serialized with transfer requests; it
does not run periodically. Running transfers block inspection.

Native USB inventory uses nusb on Windows, Linux and macOS. It reads device
enumeration metadata without opening or claiming the player interface. Candidates
match VID 054C / PID 032A. Sony's INI assigns these IDs to A916, A918 and A919:
matching them does not establish the exact model. The report preserves missing
values and never presents USB bcdDevice as an installed firmware version. Battery
and installed firmware remain unknown. Candidate serial, controller/bus path,
port chain and negotiated speed are shown when the platform supplies them.

Local UPG inspection opens an existing regular file read-only, limits input to
64 MiB, verifies the outer MD5, decrypts the researched A91x DES-ECB container,
and checks its family signature, four component entries, alignment, bounds and
non-overlap. SHA-256 hashes identify the package and decrypted components.
Only the recorded stock 1.03 package receives an exact-hash match label.
Neither MD5 nor DES authenticates a publisher, authorizes a write or proves that
a modified package is compatible. No decrypted executable or script is run.

The command-line entry points exit before catalog loading, desktop startup and
transfer recovery:

```powershell
island-walkman.exe --firmware-probe
island-walkman.exe --inspect-firmware "C:\path\NW_A91X.UPG"
```

```sh
island-walkman --firmware-probe
island-walkman --inspect-firmware /path/to/NW_A91X.UPG
```

The authenticated local API exposes POST `/api/firmware/probe` and
`/api/firmware/inspect`. There is no flash endpoint and no raw-command endpoint.
`--allow-device-write`, used for music transfers, does not enable firmware writes.

## Reverse-engineering evidence

The existing `sony-nw-a918` project is an immutable input. Its original updater,
extracted DLLs, package reports and root filesystem were read without running
the updater or installing a driver. Working copies and retained analysis are
under this application's ignored `data/firmware-analysis` and
`docs/research/firmware-updater` directories.

| Input | SHA-256 |
| --- | --- |
| FWUpdater.exe | `3cf7eb2e354d2e6d3f0cddd06ab67755696579a0d2cc85b68672ecc44ccb3c1c` |
| FWUpdaterCom.dll | `2b7550cd945a8d71d139063fce0ce006bfa6b59ee940f17e5151bdb6e35ebe2f` |
| FWUpdaterRes.dll | `0536de4cc4373f2f1391fbb52f11edf47af28fa52551959d569b6488c639b243` |
| g_nwm_msc.ko | `dd6e6c7427e81494abd7751d3adfdcb760afad338c00820cf498e6690c6583d3` |
| Stock NW_A91X.UPG | `cfbcc372e2a18f11c12a080c9e5f039333e09175f5db03ac841a4227d3896b32` |
| mgscsi.ko | `27e9ae59a30d1c3999534838010b1288840c787b798fe46bc58657b7eb30cac6` |

REA 6.5.0 with Ghidra 12.1.4 was used for the three Windows PE inputs: binary
admission, procedure and string inventory, references, memory reads and targeted
decompilation. The known working Node 22.22.0 runtime avoided an ESTALE admission
failure encountered under Node 22.15.0. Retained evidence includes input digests,
provider metadata and function addresses. Some exploratory address lookups do
not establish a semantic function identity; the findings below cite resolved
functions rather than treating every lookup as a finding.

| Observation | Evidence location | What it establishes |
| --- | --- | --- |
| Windows SCSI pass-through | COM `0x10001180`, `0x10001480` | DeviceIoControl code `0x4D014`, CDB lengths 6/10/12, data direction, status and sense handling |
| Existing handle strategy | COM `0x100010e0` | Volume handle, read/write access, shared access and repeated open attempts; not a requirement to copy that strategy |
| Version-reading branches | COM `0x100082c0` | Legacy `0xFC` subcommands 5/9 exist; exact A918 call path and returned-version semantics still require verification |
| Power query construction | COM `0x10008a30` | `0xFB` with `PW_STAT` selector and a 32-byte response; not yet enabled or decoded |
| MSC device information | COM `0x1000a180` | `0xFC` subcommand `0x20`, signature field and 128-byte response, plus an alternate `0xFB DEVINFO` branch |
| Legacy copy and activation | COM `0x10009640`, `0x10007a10` | A branch copies the file and subsequently constructs `0xFC` subcommand 4; this is not proof of the complete A918 upgrade sequence |
| Configuration ownership | EXE `0x00405cf0` | Loads CommandType, signature, product information, firmware path/version and battery settings from INI |
| Resource module | REA resource string inventory | UI/status/error resources; not evidence for a replacement transport |

Windows REA's Ghidra P0 rejects ARM ELF. The WSL REA attempt opened the module
but deeper requests did not complete; concurrent diagnostics observed stalled
process-list commands. That attempt was interrupted. The device-side cross-check
therefore used a separate, static Ghidra headless
analysis, not a successful REA decompilation and not device execution.

In `mgscsi.ko`, Ghidra loaded the text section at `0x10000`:

- `vscCheckCmd` at `0x11aac` accepts the vendor opcode for further dispatch.
- `vscProcessCmd` at `0x11b8c` checks the complete device-information request:
  `FC 00 20 64 62 6D 6E 00 80 00 00 00`. It translates it to the `DEVINFO`
  query before obtaining the response.
- `vscDpccProcessDEVINFO` at `0x12210` copies prepared information into the
  response. It does not implement the legacy activation subcommand.
- DNK processing is a separate `0xDD` branch. It includes state-changing
  operations and is deliberately outside this read-only work.

The separate USB gadget module explains the activation boundary that is absent
from the ordinary mgscsi information-response handler. In `g_nwm_msc.ko`,
`fsg_main_thread` at `0x128fc` handles `0xFC` subcommand 4 directly. It checks the
12-byte no-data request, the LUN/command constraints and the `dbmn` selector,
then calls `fsync_all`, stores 2 in the gadget state byte at offset `0x3e`,
and calls `usbconn_chg_sts`.
Its distinct subcommand `0x20` goes to the information response path. These are
static observations, not a transmitted request or proof of successful upgrade
completion. The downstream connection-state consumer, update acceptance and
failure/recovery sequence remain to be established. The additional module was
also analyzed with static Ghidra headless, not Windows REA ARM support.

`ReadQuerySpec` records the cross-checked request and expected response length
as an **offline specification** with `live_enabled = false`. It cannot be
submitted through any application endpoint. Other `0xFC` requests must not be
treated as safe merely because they use the same opcode.

The original UPG's first component also confirms that actual installation is
performed on the player: its script formats the system partition and invokes
the updater to write/extract the remaining components. It was inspected as
text, never executed. Music-file recovery is not firmware-install recovery.

## Validation and remaining work

Local Windows validation includes real USB inventory, inspection of all 18
existing UPG files with unchanged source hashes, malformed-container unit tests,
an independently encrypted synthetic fixture, existing music-transfer
regressions, frontend translation/type checks and a native desktop smoke test.
The CI matrix runs the read-only firmware smoke on Windows, Debian and both
macOS architectures; configured coverage is distinct from a completed run or
physical player validation on those systems.

All 17 modified UPGs in that local set retained the stock upgrade-script,
kernel and initrd component hashes; their root filesystem components differ.
This comparison does not prove safe boot, compatibility or a recovery path.

The implementation at commit `e9436a40253d7cca415881bda7ea1cdce8a6984a`
passed its four build/test jobs (Windows x86_64, Debian x86_64, macOS ARM64 and
macOS Intel) in [CI run 39115986675](https://github.com/vicliu624/island-walkman/actions/runs/39115986675).
The native firmware CLI smoke passed on each target. Windows and Debian also
passed the real desktop-window tests, including the read-only firmware card/API.
Local Windows Rust validation passed 75 tests; one GUI fixture exporter remains
intentionally ignored. Eight Python packaging tests and 214 complete UI message
sets passed. Hardware evidence is limited to Windows USB inventory: no vendor
query, activation, firmware copy or physical flash was performed. No firmware
feature release was published; the current release remains v0.3.0.

Reproduce the hardware-independent checks:

```sh
cargo test
npm run build --prefix ui
python tools/firmware-smoke.py --executable target/debug/island-walkman
```

On Windows use `target/debug/island-walkman.exe`. The smoke performs native USB
enumeration but does not send SCSI commands or open a player filesystem. Its
96-byte synthetic UPG is not installable firmware.

Before a native installer can be enabled, remaining evidence must establish:

1. Physical validation of the decoded A918 identity/version/power responses,
   their serial binding and a model-specific battery policy.
2. Firmware upload/activation transport behavior on Windows, Linux and macOS;
   only a fixed Windows diagnostic reader currently exists. No driver
   replacement, interface seizure or automatic unmounting is implemented.
3. The EXE-to-COM branch selected by the A918 INI, device-side upgrade acceptance,
   activation command, return values and re-enumeration sequence.
4. Immutable host staging, fresh package hashes and complete device-file readback
   before activation, plus handling of same-version installs, incompatible/modified images and failures
   before versus after irreversible device-side installation begins.
5. Simulated transport/failure tests followed by separately authorized hardware
   validation. A successful copy, process exit or USB disconnect is insufficient
   evidence of completed firmware installation.

An absolute guarantee against bricking cannot be established by local tests.
The user explicitly requires conservative implementation. Unknown checks must
remain blocking: never substitute USB revision for firmware version, infer
battery readiness from a USB connection, override model/version rejection or
retry activation after an ambiguous timeout. A music-library backup is not a
recoverable system/NAND image. Device-side recovery behavior and the point at
which rollback becomes impossible must be understood before any live activation
implementation is enabled. No hardware write is authorized by this research stage.

There is no firmware release or enabled flashing feature in this stage.
