# Device-side installation, upgrade flag and failure recovery

Date: 2026-10-19. Follow-up to [the host FWUpdater flow](FIRMWARE_UPDATER_FLOW.md). **Native firmware flashing remains unavailable. No player was connected, queried, mounted, written, reset or tested.** Sony target binaries and scripts were analyzed as bytes only, never executed. This is not a flash qualification or a successful recovery result.

## Findings and unresolved questions

Follow-up: [matching official A910 boot sources](RESIDENT_SOURCE_SEARCH.md) now establish a source-level fWUp consumer and separate resident image tables. The installed binary, resident userspace, persistent clear path and fwpup remain unverified. The original pinned-package coverage below is preserved; it must not be read as a claim that no matching boot source is now available.

Further [1.02 rootfs and resident-layout analysis](A910_ROOTFS_AND_RESIDENT_LAYOUT.md) recovers wrldr image/control-table write order and nvpflag's generic four-byte FUP write primitive. Neither supplies a verified installation-completion clear or the missing fwpup sequence.

| Question | Current evidence | Conclusion |
| --- | --- | --- |
| Where is the upgrade request written? | Gemini and apl_conn write the word `0x70555766` (`fWUp`) through NVP minor 9 | Write-side path established statically |
| What does the normal NVP driver map that field to? | ELF NodeInfo, Area relocation and MSCZone table | Logical zone 0, node 8, four bytes at logical offset 2044; not a physical NAND address |
| Who consumes the request at boot? | Follow-up official three-model U-Boot source checks fWUp and selects fUKt/fURt with /linuxrc_fwup | Source-level consumer found; installed binary equivalence and resident contents remain unknown |
| Who clears the request, when and to what value? | apl_conn helper has an all-ones branch; recovered internal caller supplies 1, not 0; no explicit clear command in the stock install script | Clear primitive is visible, but no verified installation-completion or failure-clear path |
| Is the original initrd an independent resident installer? | Complete read-only ext2 inventory and linuxrc text | This normal initrd loads five drivers and exits; no resident upgrade program found in it |
| Where is fwpup and what is its component write order? | Script invokes it, but no executable found in rootfs/initrd/embedded initramfs or outer package listing | Internal erase/program/verify order remains unknown |
| Does NVP metadata recovery establish firmware rollback? | Normal NVP driver appends/copies page groups and scans sequence metadata | No. It is parameter metadata handling in the normal Linux driver, not proof of boot-loader behavior or full-system rollback |
| Is power-loss recovery established? | Shell error branches, normal NAND status/ECC code and offline models | No independent recovery guarantee or complete power-loss contract |

## Pinned input coverage

[Static facts](audits/device-install-2026-10-19/static/facts.json) record source SHA-256 values, inventories, rootfs matches and code bindings. The inspection covers all four decrypted Sony 1.03 UPG components, already bound to the original package in the earlier audit:

- The original 1,578-byte installation script.
- The complete 2,523,136-byte normal kernel image.
- The complete 786,432-byte normal initrd image, including its ext2 directory tree and file contents.
- The rootfs compressed archive: 613 tar members, read in memory without extraction. Original symbolic links were inspected as archive metadata; the Windows extracted folder omits some of them and was not treated as a complete startup inventory.
- The preserved original self-extracting distribution, listed with 7z as data: 15 host updater/configuration/license/UPG entries, not a separate rescue image.

The normal initrd has a 752-block, 1024-byte-block ext2 filesystem in the padded image. The new bounded reader follows direct, single-indirect and double-indirect blocks in memory and reports symbolic links without following host paths. Its 502,328-byte BusyBox hash matches the earlier independently extracted copy. [Initrd inventory](audits/device-install-2026-10-19/static/initrd-inventory.json) contains 25 entries; there is no fwpup entry. Its `/linuxrc` loads the key, power, NVP and two NAND modules, then exits 0.

The normal kernel's embedded initramfs was decoded at raw-image offset `0x17ca0`: 134 gzip bytes expand to 512 CPIO bytes. Entries are `/dev`, `/dev/console` and `/root`; no installer or rescue shell contents are present in that embedded archive. This is distinct from the external normal initrd described above.

No `fwpup` literal or file was found in the decompressed rootfs inputs, and no `fWUp` literal was found in the normal kernel image. Negative literal searches do not prove absence of a generic/encoded consumer or of components elsewhere on device storage. The bounded conclusion is that these inputs do not supply the resident boot selector or fwpup implementation needed for the requested end-to-end proof.

## Upgrade flag: write side and clear-side limits

The original Gemini setter at VA `0x001a4ffc` opens the NVP path, submits four bytes and requires the returned count to equal 4 before setting its request state. The known path string and magic-value references in the preserved analysis lead back to that setter. [Static reference record](audits/device-install-2026-10-19/decompilation/gemini-flag-refs.txt) does not establish a boot-time reader or a clear operation.

The separate MSC emergency helper `apl_conn` handles connection state 2 by calling `setfuflag(1)`, then sleeping two seconds, syncing and requesting reboot if the helper reports success. Fresh static decompilation and original ARM instructions confirm:

- Nonzero argument writes `fWUp`.
- Zero argument writes `0xffffffff`, not zero.
- The helper rejects a negative write return but accepts any nonnegative count. Gemini requires exactly four.
- The one recovered internal call to this helper supplies 1. No internal call to the zero-argument branch was recovered.

The short-write tests inject zero through three returned bytes at the caller boundary. They demonstrate different caller checks, not a measured short write or a proven source of one in the normal NVP driver. In the recovered successful four-byte field-write path, the driver completes its zone work before returning the copied count; no physical short-write reproduction is claimed.

The all-ones branch cannot be promoted to an authorized recovery action. We have not established when the resident code reads the flag, whether it recognizes that value, whether it clears before/after installation, whether it preserves the request on failure, or whether it uses the same snapshot selection as the normal kernel driver. The fact that the shell script contains no explicit clear command does not exclude clearing inside the unavailable fwpup/resident code.

## NVP field binding and storage layer

The NVP module in the normal initrd and the original rootfs is byte-identical, SHA-256 `c0639bc1f31c0208ee71381c11777950d5af157baf23bde6ecde6cbd148ab377`.

Reading its ELF symbol tables gives NodeInfo[9] = (zone 0, node 8). Area[0]'s node-table pointer must be resolved through an R_ARM_ABS32 relocation: it points to `.rodata + 0xc0`, the MSCZone table. It does not point to the differently named INIZone table. MSCZone[8] gives logical offset `0x7fc`, size 4, inside the configured 2048-byte zone tagged `0x70`. These are logical NVP metadata offsets, not measured device geometry or an address for direct flash access.

The fresh static module analysis establishes the following normal-driver sequence:

1. Select the minor's logical zone/item and serialize access with the driver semaphore.
2. Read current zone pages and merge the requested data into the RAM page buffer. An absent prior area is represented by an all-ones buffer in the area-reader helper.
3. Append a new group in the current block if space allows, or allocate another block. The allocator erases a candidate block before using it and handles failed erase candidates through its defect path.
4. Give pages zone tags and incrementing sequence metadata, program each page, and attempt to read it back. The zone writer tests read/write return codes; no explicit payload-byte comparison appears in that function. The BIOS/controller/ECC layers may themselves detect faults, so this is not proof that corrupt NAND necessarily passes.
5. On page errors, attempt allocation/copying through a different block. After a complete group, update in-memory block/group selection. Retiring a block ID in `icx1087_nvp_set_block_id` updates RAM bookkeeping; it is not itself proof of an on-flash commit or an erase.
6. During normal module startup, analyze candidate blocks and page groups using zone tags and sequence metadata. The selected-block scan tracks the highest valid sequence and can ignore failed tail reads or stop at an erased tag after a valid group.

The first candidate-block multi-page check also needs care: at module `.text` offset `0x93c`, the last-page read call is followed by tag/sequence comparisons without an explicit test of its returned error code. The selected-block tail loop modeled here is separate and does check read success. Failed-read buffer/OOB effects must therefore not be inferred from that smaller model or treated as universally safe. Both the Ghidra export and original object instructions are retained.

The normal-driver snapshot mechanism is useful evidence for NVP handling, but not an atomicity proof. Torn NAND programming, torn OOB/tag writes, ECC outcomes, sequence rollover, stale data after failed reads and available spare blocks remain relevant. The normal scanner is not the missing earlier boot-time flag consumer. The model therefore covers only its already-selected-block tail scan, and never claims to simulate the complete boot selector or NAND recovery algorithm.

## Normal kernel NAND status and verification boundaries

The reconstructed kernel ELF is a research wrapper, not the original package format. Entire selected function ranges were compared back to the pinned raw kernel bytes before using its symbols/decompilation. These bindings are in the static facts. Ghidra's ARM decoder setting is not a hardware ISA qualification, and decompiler type/register warnings remain visible in [26 preserved analysis exports](audits/device-install-2026-10-19/analysis-exports.json).

Relevant original symbol addresses are:

| Function | Normal-kernel VA | What was recovered |
| --- | --- | --- |
| `icx1087_nand_bios_read_page` | `0xc0145544` | Controller transfer, ECC correction/error paths and timeout handling |
| `icx1087_nand_bios_write_page` | `0xc01459f4` | Program operation, controller completion/status checks and error/timeout branches |
| `icx1087_nand_bios_erase_block` | `0xc0145eb4` | Erase operation and analogous completion/status handling |
| `icx1087_nand_bios_probe` | `0xc014756c` | Runtime controller/BIOS structure initialization |

The NVP read-page wrapper retries a nonzero BIOS result at most twice and returns zero only on a zero BIOS result; otherwise it returns -5. The write/erase wrappers also translate nonzero BIOS failure into driver error rather than pretending success.

For the inspected ordinary program/erase routines, controller status satisfying `(status & 0xc1) == 0xc0` is the success branch; the NAND failure indication returns failure. Unexpected controller state and configured timeout conditions can enter `panic` outside the special panic context. Other timeout cases log/return an error. This does not establish how a watchdog or resident upgrader subsequently restarts, retries or repairs a failed installation. Timing constants in these functions were not converted into physical milliseconds without a verified clock/configuration contract.

No mapping from fwpup's command-line flags to these particular normal-kernel operations has been established. It may run in a different resident environment. Finding NAND program/erase/ECC routines in this kernel does not reveal fwpup's component order or prove full-image readback, hash/signature checking, atomic replacement or rollback.

## Known shell ordering and opaque fwpup boundary

The pinned install script has these 1-based anchors:

| Line | Operation |
| --- | --- |
| 23 | Format the system root partition |
| 30 | Mount that new filesystem |
| 47 | Read the expected update filename from NVP |
| 54 | Check that the expected UPG file exists under contents |
| 60 | Invoke fwpup and pipe output into tar |
| 77 | sync |
| 84 | Unmount the new root |

This proves that filename/package failure can happen after the old root has already been formatted. It does not establish which flash component fwpup erases/programs first. The pure model records a single opaque fwpup/tar event instead of inventing kernel/initrd write stages.

The `/version` guard is a lexical comparison against `20070508A`, not a comparison of the UI's numeric 1.03. An old value exits 0 without installing anything. The script does not separately test cat's return status, so empty stdout also follows the early-exit branch in the model. A zero shell exit is therefore not an installed-build readback.

The script checks the pipeline exit status, normally the tar consumer's status. Existing host-only pipeline tests plus the new control-flow replay cover failed fwpup with successful tar. These are synthetic host/model observations; resident-shell semantics and an actual device failure trace remain unmeasured. A successful final sync/unmount does not retroactively establish that the producer, all component writes or the installed build were valid.

## Power-loss and failure matrix

| Boundary or failure | What the current evidence supports | Still unknown / not guaranteed |
| --- | --- | --- |
| Before a flag request or before root formatting | This particular modeled operation has not changed the old root yet | Whether an already-failed device can boot; no reset/recovery recommendation follows |
| During flag page/tag programming | Normal NVP driver has snapshot metadata and retry paths | What the earlier boot consumer selects after torn data/OOB writes |
| Flag written, before reboot | Four-byte request/return checks exist; helper and normal app differ | Durable flag value under power loss, boot selection, clearing and resident recovery |
| Resident installer starts | The update script needs an installation environment and fwpup | Its entry contract, package staging, preparation and failure handling are not recovered |
| During root formatting | Old-root preservation ceases to be guaranteed once formatting starts | Independent rescue/rollback is not established |
| Filename read, package presence or mount fails after format | Error-display and exit branch; no old-root restoration in the shell | Whether resident code repeats, clears the request or provides a working transport |
| fwpup programs kernel/initrd or generates root archive | Script invokes an opaque producer; normal kernel has low-level NAND functions | Component order, erase boundaries, verification, failure cleanup and boot survivability |
| tar fails / root image cannot be fully expanded | Script can exit with a partially populated new root | No transactional root rollback in the script; normal app emergency depends on a usable root |
| Producer fails but consumer succeeds | Host/model pipeline can return success | Not proof this happened on device or caused the reported R12 failure |
| sync or unmount fails | Script displays an error and exits 1 | Durability of prior writes and resident retry/flag behavior |
| Power drops at any modeled boundary | Model stops without inventing a resume action | Post-restart flag, boot target and usable recovery remain unknown |
| Script exits 0 | Control flow finished or exited early | Exact installed build, flag clearing and bootability require separate readback/evidence |
| A flag is hypothetically cleared after a failed install | No restored old root follows from that operation | Clearing alone is not a firmware recovery procedure |

The normal rootfs inittab references `/etc/init.d/powerfail`, but that file is not present in the pinned rootfs archive. Those entries cannot establish a functioning brownout flush/repair hook. This is not evidence that the hardware has no lower-level power protection.

The normal `gemini` startup script's emergency path depends on the root scripts, drivers and helper executable remaining usable. Its separate exit-code-63 repair path may format data/db storage and is not a generic root/NAND restore mechanism. None of those scripts was executed. No speculative physical reset, wiring, probe or device-side command is part of this work.

## Offline replay, validation and build consistency

[The pure device-side model](../tools/firmware_research/device_install_model.py) and [21 new tests](../tools/test_device_install.py) cover caller write-count checks, the uncalled helper all-ones branch, selected NVP tail metadata behavior, two-read retry, BIOS status/timeout/panic branches, every explicit shell failure branch, masked producer failure, old/empty version exit and all 11 shell power-cut boundaries. Every modeled result retains unverified recovery and unavailable firmware writing. The model has no target I/O, file/script execution or driver imports.

[55 retained symbolic cases](audits/device-install-2026-10-19/fault-replay.json), including 26 install-script scenarios, bind source hashes. They are neither CPU execution of Sony code nor real power-cut experiments. Synthetic ext2 tests validate direct/indirect file reading, link reporting and rejection of malformed geometry/directory/block references; original files were never mounted.

During native validation, the workspace's device configuration had reverted to the old A918-only fixture interface while firmware callers still required the shared-profile API. This caused six missing-item compilation errors. The already-agreed A916/A918/A919 shared firmware definition was restored, keeping fixture provenance separate from compatibility. [Consistency evidence](audits/device-install-2026-10-19/workspace-consistency.json) preserves pre-fix hashes; this is not a new flashing capability. Application prerelease metadata was synchronized to 0.4.0-alpha.5 without dependency upgrades, firmware package changes, release publication or tag changes.

Final checks: 60 Python tests passed; 107 Rust tests passed with one intentionally ignored fixture exporter; frontend build, 248 translation checks and version consistency passed. Fresh native safety commands with and without music-write authorization still report firmware/raw-command writes disabled and create no catalog. [Validation record](audits/device-install-2026-10-19/validation.json) pins the new native binary/source hashes. No physical or desktop hardware smoke was performed.

## Evidence needed to close the remaining chain

A verified matching resident boot/installer image or an already available trustworthy offline dump is required to identify the actual upgrade-flag consumer, accepted inactive values, clear timing and retry policy. The fwpup executable and its environment are required to establish component erase/program/readback order and return/error behavior. Boot selection and NAND/OOB/flag persistence must then be checked against those exact implementations, followed by offline fault modeling of their actual branches.

The existing package alone does not supply those missing implementations. Do not substitute another model's rescue behavior, the normal NVP scanner, a zero shell exit or the visible music-volume backup. Until those gaps and an independent matching-model recovery contract are closed, no live flash executor, recovery/clear-flag action or hardware trial may be enabled. This report authorizes none of them.

## Reproduce using local data only

```powershell
python -m unittest discover -s tools -p test_device_install.py -v
python -m unittest discover -s tools -p 'test_*.py'
python tools/run_device_install_replay.py --output data/firmware-analysis/new-device-fault-replay.json
python tools/firmware_research/collect_device_install.py --work ../sony-nw-a918/work --output data/firmware-analysis/new-device-static-inventory --objdump 'C:/Program Files/LLVM/bin/llvm-objdump.exe' --sevenzip 'C:/Program Files/7-Zip/7z.exe'
```

The collection/replay writers require new output paths. Original input hashes are checked before parsing; root/initrd drivers are compared byte for byte and the NVP relocation is resolved, not guessed. llvm-objdump and 7z only read supplied files. Optional static Ghidra exports use [ExportDevice.java](../tools/firmware_research/ExportDevice.java) and [ExportDeviceRefs.java](../tools/firmware_research/ExportDeviceRefs.java); intermediate projects remain under data/firmware-analysis. No source Sony executable or shell script is run by these commands.
