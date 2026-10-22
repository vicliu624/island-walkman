# Offline device-install evidence

See [device installation analysis](../../DEVICE_INSTALL_CHAIN.md) for findings, power-loss boundaries and unresolved recovery/installer questions.

- `static/facts.json`: pinned input hashes, logical NVP mapping including ELF relocation, complete initrd/rootfs inventory scope, kernel byte binding, script anchors and limitations.
- `static/initrd-inventory.json`, `static/rootfs-inventory.json`: all known normal-package entries; no image mounted or executed.
- `static/*.txt`: static ARM instruction listings and the outer self-extracting archive listing. No target binaries are bundled.
- `analysis-exports.json`, `decompilation/`: 26 static Ghidra/reference exports and input/output hashes. Kernel warning/architecture/reconstruction limits remain explicit.
- `fault-replay.json`: 55 source-derived synthetic cases, including 11 shell power-cut boundaries. Every result keeps recovery unverified; no physical power-cut test is represented.
- `workspace-consistency.json`: compilation interface mismatch and restoration of the previously agreed shared A910 firmware profile.
- `validation.json`: host test/build results and closed native safety-gate reports for application candidate 0.4.0-alpha.5.

No boot-time flag consumer, resident installer or fwpup executable was recovered. In particular, normal-driver NVP metadata handling is not a full-system recovery guarantee. No live command, flag clearing, firmware build/flash or release publication is authorized by these artifacts.
