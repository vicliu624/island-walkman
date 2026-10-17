# Global table header comparison

The accepted A918 `00GTRLST.DAT` snapshot has SHA-256 `4492885fde3677b0bfa726c0139ab6ce590db81b3d13e3991f1eaedb45b47988`. It was retained in the verified host backup before native transfer testing. No player was accessed for this comparison.

## Observed profiles

| Field | Earlier transferred library | Firmware-normalized accepted library | Earlier Rust output |
| --- | --- | --- | --- |
| SYSB offset | 0x30 | 0x30 | 0x30 |
| SYSB length | 0x70 | 0x60 | 0x70 |
| SYSB row count/width | 1 / 0x50 | 1 / 0x50 | 1 / 0x50 |
| SYSB flag word | 0xD1010100 | 0xD1010100 | 0xD0000000 |
| GTLB offset | 0xA0 | 0x90 | 0xA0 |

The firmware-normalized SYSB span is exactly its 16-byte header plus one 80-byte row. Earlier padding was accepted and later removed, so the padding difference alone is not established as invalid. The flag word's business meaning is unresolved.

## Firmware reader evidence

REA/Ghidra analysis of original `gemini-ICX1101` (SHA-256 `a5c94ce492fb7ec9cd7a740d30cb79a1c2b43ee782e2da036a72f57dabdcd177`) shows:

- `0x213924` opens the two-section global table and reads section index 1 (GTLB) in read mode. Evidence: `ev_a6038ae91b5847ee31995d9280deec66c130aafbcd633072ae17a5ae57a85756`.
- `0x2131f0` reads the file header and directory and decodes directory offsets and lengths.
- `0x212c5c` seeks to the section offset from the directory and reads its class header. It does not hard-code either observed GTLB offset.

This path supports offset-driven parsing. It does not independently establish SYSB flag semantics or prove that the earlier Rust flag value would be rejected. Other SYSB consumers have not been exhaustively traced. The short callback entry `0x213c98` was not resolved as a Ghidra procedure; its retained error is not behavioral evidence.

## Serializer choice

The current serializer uses the accepted A918 flag word as opaque profile data and lets the SYSB length derive from its row data, yielding the compact 0x60 span. A regression fixture compares the first 64 protocol bytes with the accepted snapshot. The GTLB active-tree count is generated from this application's views rather than copied from firmware-generated auxiliary trees.

This aligns output with a known accepted profile. It is not a claim to fix the physical device's I/O faults or to complete native transfer validation. Raw analysis JSON and decompiled listings remain local and are not distributed.
