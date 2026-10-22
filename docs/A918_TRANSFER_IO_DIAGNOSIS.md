# A918 transfer I/O and scan-and-repair investigation

Date: 2026-10-21. This investigation combines existing device evidence, current Island Walkman source, and read-only REA analysis of Sony binaries. No device writes or flashing were performed in this round.

## Conclusion and limits

The recorded failure is below the music database: **WRITE(10) requests timed out** during ordinary copy and during the alpha.6 compatible-mode transfer. The later dirty-volume state is consistent with the I/O errors. The ultimate source of the timeouts remains unknown: USB transport, device-side storage blocking, connection/power, and hardware faults have not been distinguished.

REA also found weak cache-flush behavior in the original Sony mass-storage driver. This is relevant to durability, but it is not proof that SYNCHRONIZE CACHE caused these timeouts. The timed-out opcode was WRITE(10), not SYNCHRONIZE CACHE.

A completed copy and matching file hashes do not establish an error-free storage session or durable physical-media writes. No conclusion here proves that the NAND is damaged.

## Device evidence

- The user explicitly reported successful Windows safe ejection before unplugging. Unsafe removal is not an established explanation.
- Two no-transfer controls produced no repair prompt: immediate reconnection after safe ejection, and reconnection after 15 seconds in Sony's UI.
- At approximately 21:06 on 2026-10-20, CopyFileW/readback replay into an isolated IWDIAG directory copied and verified one audio file, then failed with Win32 1117. It did not commit an OMGAUDIO database. Database contents are therefore not necessary to reproduce this I/O failure.
- Windows disk 2 logged events 51/153. The volume became Full Repair Needed with DirtyBitSet=true. FAT32 boot byte 65 changed from 0 to 1; both FAT entry-1 clean/error flags remained set.
- User-authorized CHKDSK /f found no structural errors. The 21:13:55 snapshot showed boot byte 65 restored to zero and the volume Healthy/OK. This does not establish flash health or prevent recurrence.
- The alpha.6 compatible-mode single-song session ran from approximately 21:20:12 to 21:22:01 and verified/committed 17 files. A WRITE(10) timeout was nevertheless logged at 21:20:35. A subsequent mounted-volume inspection showed the dirty flag set. Safe-eject/reconnect and playback validation of this session was not completed. A dirty flag while mounted alone is not proof of persistent damage.

## Decoding event 153

Four relevant events at 21:06:00, 21:06:23, 21:06:50 and 21:20:35 decode identically:

| Field | Value | Meaning |
| --- | --- | --- |
| SCSI status | 0x00 | GOOD; not a durability attestation |
| SRB status | 0x09 | TIMEOUT |
| SCSI opcode | 0x2A | WRITE(10) |

Microsoft classpnp encodes DumpData[0] as opcode shifted by 24, SRB status by 16, and SCSI status by 8. These events have 44 binary bytes, DumpDataSize=4 and StringOffset=44; the status word starts at offset 40. Do not blindly use the fixed offsets shown for older event layouts.

See Microsoft's [event 153 explanation](https://learn.microsoft.com/en-us/archive/blogs/ntdebugging/interpreting-event-153-errors) and [classpnp logger implementation](https://github.com/microsoft/Windows-driver-samples/blob/main/storage/class/classpnp/src/utils.c). No device kernel log, NAND error counter or useful medium-error sense data was captured.

## REA analysis

REA 6.5.0 called Ghidra 12.1.4 through MCP in WSL. The inspected targets were original Sony ELF modules extracted from the firmware, not reconstructed source. The primary mass-storage module is g_nwm_msc.ko, 41,860 bytes, SHA-256:

    dd6e6c7427e81494abd7751d3adfdcb760afad338c00820cf498e6690c6583d3

| Function | ELF .text symbol offset | Observation |
| --- | --- | --- |
| fsg_main_thread | 0x28fc | Dispatches storage commands |
| do_write | 0x0b6c | Uses 2048-byte logical sectors, a sync heuristic, then vfs_write |
| jadge_sync | 0x4aa8 | Reads FAT parameters and detects sufficiently large forward jumps |
| fsync_sub | 0x1014 | Performs filemap writeback, file fsync and waiting; returns errors |
| fsync_all | 0x1114 | Iterates LUNs and prints results without returning those errors |
| handle_exception | 0x2270 | Some reset, suspend and Vbus-off paths call fsync_all |

Ghidra loads this module at 0x10000, so its analysis addresses are the offsets above plus that base. The raw Evidence retains target hashes and analysis profiles.

### Cache synchronization

The recognized SYNCHRONIZE CACHE (0x35) branch checks the command and joins the common response path without calling fsync_sub/fsync_all. The inspected check_command, finish_reply and send_status paths do not add a storage flush. MODE SENSE's caching page sets the WCE bit.

For comparison, [upstream Linux v2.6.12 file_storage.c](https://github.com/torvalds/linux/blob/v2.6.12/drivers/usb/gadget/file_storage.c) routes that command to a function that calls fsync_sub. This nearby historical source is a behavioral reference, not proof of Sony's exact parent revision or development history.

Do not assume one host flush necessarily commits all device-side buffered data through the standard command. Whether that command was actually sent in this reproduction still requires USB tracing. Actual flushing exists on other Sony driver lifecycle paths, so this is not a claim that the device never flushes. fsync_all prints individual errors but does not return them to its caller. Do not repurpose Sony's firmware-upgrade command as a flush workaround; it also changes device state.

### Write latency hypothesis

Sony's do_write clears the backing file's old-kernel synchronous-write flag 0x1000, then sets it according to jadge_sync. For a data-area write exceeding 4096 bytes, a start LBA more than 16 logical sectors beyond the previous qualifying write's end can select synchronous output. Each logical sector is 2048 bytes here.

File allocation and write ordering could therefore change latency. We have not captured each failed request's full CDB, length or device execution time; the heuristic is a falsifiable hypothesis, not a proven explanation of every timeout.

REA also inspected NAND write-page, work-block merge/flush, bad-block limits and continuous-error handling in icx1087-nand-p.ko. Static code exposes possible work-block and mapping updates, but does not reveal the player's actual latency, bad-block count or error counters. Neither NAND failure nor healthy NAND has been established.

## Original versus custom firmware

All 27 ordinary kernel module files in the downloaded alpha.2 English rootfs are byte-identical to original Sony 1.03, including the USB gadget/controller and NAND drivers. load_usb_epson and the driver startup script also match. The cache behavior above belongs to original Sony code, not a driver patch introduced here.

This is an artifact comparison, not a live read of the player's installed files. It does not exclude all runtime effects of custom firmware, app ownership, or the device's current state.

## Related Island Walkman implementation gaps

1. The user's original alpha.1 executable differs from later source. The alpha.6 candidate connects the active reliable-transfer path to the selected Compatible profile; newer source is not evidence of older binary behavior.
2. Through alpha.6, reliable_transfer::transient classified Windows 1117 as retryable, permitting up to three retries. The local alpha.7 candidate stops automatic retries for storage errors (Windows 23/29/30/121/1117/1392/1393), retains the original error and recovery state, and avoids further device-resolution reads in that failure branch. It does not automatically resume that session even if the volume disappears. This changes failure handling, not the underlying timeout cause.
3. The 32KiB application buffer is not a USB/SCSI request-size setting. This code does not directly configure WRITE(10) lengths.
4. Normal file readback may hit host or device caches. Hash verification is not an independent post-disconnect media read.
5. The successful compatible-mode session coexisted with a kernel storage retry. The application currently does not expose recovered kernel I/O errors as session health; its completed state cannot mean no storage anomalies occurred.

These are confirmed limits, not separate proven root causes. Alpha.7 regression tests cover immediate stop, commit-time recovery state and wrapped I/O errors. The final suite passed 111 tests (one ignored); frontend build, version consistency, package smoke and native desktop smoke also passed. No hardware qualification was performed. Recovered event-153 retries can still be invisible to the file API and are not detected by this change. Clearing dirty flags or ignoring errors is not a repair.

## Application installation as a control

The user confirmed that installing the color Tamagotchi application, including its 4 MiB ROM, does not produce the repair warning. The bundled package contains 5,139,454 payload bytes; its largest file is 4,194,304 bytes. The prepared single-song OMA is only 3,661,512 bytes. File size alone therefore does not explain the observed difference.

| Path | Current behavior |
| --- | --- |
| Application installer | Stage under APPS/.install-<id>; write each loaded payload with write_all, sync, close and verify; rename the staged application into place |
| Music transfer | Copy audio via temporary files under live OMGAUDIO/10F00, move into staging, write multiple database files, verify and commit the library; copy strategy depends on the selected profile |
| Shared operations | Both paths synchronize files, verify contents and rename directories/files |

Write granularity, pacing, file count, allocation/layout and operation ordering remain candidate differences. The ordinary-copy failure in IWDIAG means a music database commit is not required to trigger the underlying I/O failure. A missing flush in only the music path has not been established.

A private comparison catalog is prepared under target/diagnostics/app-vs-music-2026-10-21/repository. It sends the exact same 3,661,512-byte OMA payload through the unchanged application installer as sample.bin; the SHA-256 is 80de05aa42c353b8b6fd9da62f2e46cc00bd3a2f9fe67b2519b9d454580592a3. The local alpha.7 backend accepted the catalog and all hashes matched. It has not been installed on the device. This control holds payload contents constant but does not independently isolate destination layout from write strategy. It is private diagnostic data, not a distributable application or firmware.

The device is currently unavailable. Start any later test with read-only volume health inspection, then observe safe-eject/reconnect behavior and independently reread the payload. Do not interpret local checks as proof that alpha.7 eliminates the repair prompt.

## Remaining discriminating test

On the same working player, cable and firmware, capture a controlled small transfer's SCSI/USB timeline: WRITE(10) LBA, size, completion time, resets and SYNCHRONIZE CACHE. Compare with ordinary Windows copying and correlate with the device-side paths. This can separate transport completion problems from storage-backend delays without first reflashing firmware.

Then safely eject, reconnect and independently reread the files and volume flags. Mounted-volume DirtyBitSet or a second cached hash alone cannot complete that test.

Microsoft's [FastFAT volume-state logic](https://github.com/microsoft/Windows-driver-samples/blob/main/filesys/fastfat/verfysup.c) and [I/O handling](https://github.com/microsoft/Windows-driver-samples/blob/main/filesys/fastfat/deviosup.c) explain how errors can lead to a retained dirty/check state. These public samples are explanatory references, not byte-for-byte analysis of this Windows installation's driver.

## Local evidence

- target/diagnostics/a918-transfer-2026-10-20/: original journals, ordinary-copy replay, before/after FAT snapshots, and the compatible single-song session.
- target/diagnostics/a918-transfer-2026-10-21/rea/: original REA Evidence, pseudocode, module inputs, MCP plans and server logs.
- decoded-event153.json, disk-events-full.json and driver-comparison.json in that day's directory: event decoding, original event records and original/alpha.2 comparison.
- microsoft-source/: downloaded Microsoft sources and content SHA-256 manifest. The main branch was not commit-pinned; hashes bind only the retrieved bytes.

These local diagnostic directories contain personal evidence or Sony binaries and are not public release assets. The accompanying findings.zh-CN.md is a local Chinese working note, not the maintained report.
