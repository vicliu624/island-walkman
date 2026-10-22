# Custom firmware identity and version contract

Status: reserved wire contract and host-side decoder implemented. Existing R14 LOCAL1 and earlier packages have not been changed, rebuilt or flashed. No validated firmware producer currently emits the new identity. Do not advertise live custom-firmware recognition as complete.

## Separate version domains

- Desktop tool: application SemVer from tools/version.py.
- Sony base: retain the researched four-byte numeric revision, e.g. 1.03. Do not inflate it to distinguish a modified build: Sony updater comparison and /version checks depend on it.
- Custom distribution: Island (IW namespace), revision R14, build LOCAL2, prerelease status. Full human display example: `Island R14 LOCAL2 (Sony base 1.03)`. LOCAL2 is an example for a future candidate, not a renamed LOCAL1 artifact.
- Disk applications: their own package versions and an explicitly reported launcher ABI. Never derive ABI from desktop version or a minimum R number.

A916/A918/A919 share the same firmware identity/version domain. Device model and serial remain separate identity fields.

## Compact device declaration

Reserve four DEVINFO subrevision bytes (offsets 44..48) for ASCII `IWnn`, e.g. `IW14`. Preserve the base revision at 40..44. This proposal fits the decoded field width, but the firmware-side source of the subrevision and its effect on Sony software must be verified before implementation. Do not patch a binary or NVP value based only on this reservation.

The host decoder reports `island_declared`, custom revision and base version when a supported family reports this exact namespace. It reports `unidentified` otherwise. `1.03 / 00` cannot establish stock provenance: older modified images also retained Sony's version. Unknown versions must never be labelled original Sony or current Island by inference. USB bcdDevice, UPG filenames and copies of files on the data volume are not installed firmware identity.

The compact marker identifies the declared distribution/revision only. It does not distinguish LOCAL1 from LOCAL2 and grants no launcher ABI or authenticity. Those fields remain unknown in current diagnostics.

## Full installed-build report required before app compatibility enforcement

The future firmware-side identity reader must return a versioned structure generated from the same build metadata as the firmware's on-device About/version display:

```json
{
  "schema": 1,
  "distribution": "island",
  "base_version": "1.03",
  "custom_revision": 14,
  "build": "LOCAL2",
  "channel": "local",
  "firmware_family": "sony-a910-upg",
  "launcher_abi": 1
}
```

These are example values, not evidence of an installed release. Bind a live response to the connected volume/device and invalidate it on disconnect or firmware update. A manifest copied to the data volume can be stale or user-created and is not sufficient by itself. A reported brand is identification, not cryptographic publisher authentication.

App compatibility must require that live identity plus an explicitly supported launcher ABI; stock, legacy custom, malformed, unsupported and unreadable identities are distinct outcomes. The current installer does not yet implement that live identity gate. No marker in a filename or a catalog is a substitute. Current host decoder work must not be presented as completing that gate.

## Firmware producer acceptance criteria

Before creating a new candidate, trace the actual source of DEVINFO subrevision, ensure a build identifier can be read live, and test the Sony updater and on-device version display. Generate all identity fields from one build metadata source. Verify identity in the decoded candidate, then compare a live readback with its expected full build and exact device identity after a separately authorized installation. Retain the previous UPG/ZIP unchanged and create a new candidate/version. Do not modify or flash the currently failed-update device as part of identity development.
