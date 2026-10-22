# Offline FWUpdater follow-up evidence

See [flow analysis](../../FIRMWARE_UPDATER_FLOW.md) for conclusions and limitations.

- `static-expanded/`: current static PE/COM bindings, official English confirmation resources and 15 instruction-range exports, indexed by SHA-256 in facts.json.
- `native-gate.json`: fresh offline safety CLI results with and without music-write authorization; both keep firmware writes unavailable and create no catalog.
- `transport-defaults.json`: pinned original constructor bytes for the default five-second SPTI timeout; not a measurement of physical timing.
- `replay-final.json`: current 28-case pure symbolic replay. Records source/test/fixture hashes and every fake-clock event. Not a physical USB trace or an execution of Sony code.
- `static/` and `replay.json`: superseded preliminary snapshots from this research turn; retained for audit. Use static-expanded and replay-final for the final analysis.

No executable Sony binary, new firmware candidate, live device capture or flashing result is included. Original inputs remain unchanged in their existing project locations. The initial [parity audit snapshot](../fwupdater-2026-10-19.json) is preserved separately and is not overwritten by this follow-up.
