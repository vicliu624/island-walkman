# A910 resident updater and boot source search

Date: 2026-10-19. This extends [the device installation audit](DEVICE_INSTALL_CHAIN.md). All work is offline or public HTTP research. No player was enumerated, connected, mounted, queried, reset or written. No Sony executable, target shell script, bootloader build or installer was executed. Native firmware flashing remains unavailable.

## Result and provenance

Follow-up: [complete 1.02 rootfs comparison and resident layout](A910_ROOTFS_AND_RESIDENT_LAYOUT.md) now covers the previously un-inventoried rootfs. It finds unchanged wrldr/nvpflag helpers, eight changed payloads, and no recovered resident entry or fwpup. Statements below about that rootfs not yet being inventoried describe the original search stage.

Sony's [NW-A916 open-source page](https://oss.sony.net/Products/Linux/Audio/NW-A916.html) explicitly lists **NW-A916, NW-A918 and NW-A919 together**. It links the following matching-family source distributions. The current Audio category index did not expose this page during this search; direct retrieval succeeded. Model listing is stronger evidence than inferring compatibility from a similar model, but does not establish that a particular source build equals the installed bootloader.

| Official material recovered | Verified bytes | What it supplies | What it does not supply |
| --- | ---: | --- | --- |
| u-boot-1.1.1.local.tgz | 6,231,505 | Boot selection, NVP setting reader, separate update image tables, image checksum and fatal boot branches | Qualified installed loader binary or complete NAND image |
| linux-kernel-2.6.10.local.tgz | 49,068,350 | Matching published kernel sources; NVP header names minor 9 as FUP | Resident userspace, `/linuxrc_fwup`, `fwpup` or recovery image |
| mtd-utils-2.6.9_1.1-6.0.0.0501450.src.rpm | 1,132,487 | Ordinary MTD source archive and three flash_eraseall patches | A910 fwpup implementation or complete image |
| Sony NW-A910_V1_02.exe | 13,625,952 | Older official update distribution, valid Windows SONY Corporation signature at inspection | Factory/resident partition dump |

The three source archives match both freshly computed SHA-256 and the official download server's Content-MD5. MD5 here detects transfer corruption; it is not an independent publisher signature. The page, URLs, hashes and retained upstream file hashes are recorded in [U-Boot provenance](audits/resident-source-search-2026-10-19/u-boot-provenance.json), [archive verification](audits/resident-source-search-2026-10-19/archive-verification.json) and [search evidence](audits/resident-source-search-2026-10-19/search-evidence.json). Downloads remain in the ignored local research cache. Interrupted/partial copies are preserved separately and excluded from verified inputs.

U-Boot has 3,497 archive entries, including 3,146 regular files; the kernel archive has 18,890 entries, including 17,773 regular files. All regular files in these two archives were scanned, with none exceeding the collector's 4 MiB bound. No file is named fwpup or linuxrc_fwup. U-Boot contains the linuxrc_fwup command-line literal, while neither archive supplies its executable/script. The MTD inner source archive has 665 entries and no recovered fwpup name/content match. These are bounded inventory results, not proof that Sony never distributed a resident image elsewhere.

`include/configs/mp201mini.h:58` identifies ICX1101 and a 20070928A string. The source also contains CONFIG_ICX1087/CONFIG_ICX1111 variants. Official model attribution plus this marker does not determine the exact installed build options. No build was attempted, and no other model's image or recovery procedure was applied.

## Boot-time request consumption now visible in matching official source

The retained upstream references include original copyrights and GPL terms; [source hash/line anchors](audits/resident-source-search-2026-10-19/u-boot-provenance.json) bind the observations below. They are static source observations, not device execution results.

`board/mp201xx/mp200.c` initializes the settings before the power-management path. The NAND reader obtains the setting tag 0x70. `icx1087.h` defines the persistent request magic as 0x70555766, matching the fWUp word written by the previously analyzed Gemini/apl_conn path. The 2 KiB setting layout places this field at byte 2044, corroborating the normal NVP driver mapping. These are logical layout observations, not permission to write a physical NAND address.

`board/mp201xx/icx1087/icx1087_nand.c:439` selects the following paths in order:

| Priority | Source condition | Init entry | Image selection |
| --- | --- | --- | --- |
| 1 | Hardware install-mode condition | /linuxrc_inst | Loader management lOAd, fUKt and fURt |
| 2 | Emergency mode argument | /linuxrc | Same update table pair |
| 3 | Persistent fWUp equals magic | /linuxrc_fwup | Same update table pair |
| 4 | Otherwise | Existing normal boot arguments | lOAd/pARt partition table, then kRNl management and kNLt/iRDt |

This establishes a source-level consumer and a separate resident kernel/initrd pair. It does **not** establish a usable hardware entry sequence, a recovery command, or the availability/integrity of those resident images on a particular player. The emergency wrapper in the source is not a tested recovery procedure.

## Checks, retry bounds and fatal failures

The loader retries management-table selection up to five times. It loads the kernel first; only a successful kernel load proceeds to the initrd. Both loads request a checksum. `nand_load_image` XORs the loaded 32-bit words and accepts only zero. Checksum failure can cause up to five whole-image attempts. The underlying page read has its own five-attempt bound; exhausted page reading returns an error immediately from the image loader rather than completing the remaining checksum attempts.

The pinned normal 1.03 kernel and initrd both have XOR32 zero, which corroborates this format interpretation. XOR is a weak corruption check, not a cryptographic signature, and this result does not authenticate unknown images or prove that the resident images are intact.

Exhausted table lookup, failed kernel/initrd loading, or a returned kernel entry reaches `nand_boot_err` (line 810), which prints a boot failure and loops forever. There is no automatic switch from a failed resident boot to normal images in this function. No overall elapsed timeout or automatic successful recovery follows from the retry counters.

Power-management source includes conditional firmware-update battery thresholds. Their values are raw ADC thresholds with different compilation branches, not percentages or established voltage limits for the examined player. The host INI's missing threshold is therefore not proof that boot has no power check. This still does not establish power-loss recovery during erase/program.

## RAM boot marker is not the persistent update request

The source clears `ICX1087_CLR_BOOTFLG()` before jumping to an update kernel. This macro clears a RAM marker with the BoOt value 0x744f6f42. It does **not** clear the persistent NVP fWUp field. Nearby comments about the kernel USB driver clearing BOOTFLG refer to that RAM marker.

No installation-completion/failure clear of the persistent request has been recovered in these boot routines. If that request remains set, the selector would choose the update path again on a subsequent normal-mode invocation; whether and when the actual resident userspace clears it remains unknown. This conditional source result is not a diagnosis of a particular observed reboot loop.

## Older update and public-search coverage

The official [A910 update history](https://www.sony.jp/support/walkman/download/nw-a916_h.html) and historical download records were checked. The official 1.02 package was downloaded from Sony's update host and listed/extracted as data. Windows reports a valid SONY Corporation Authenticode signer (thumbprint 4FD9992EDE35215F4DED496DBE6CDBA16EA67FE5). This authenticates the signed distribution at inspection, not an unknown device dump.

The local offline package inspector validates the 1.02 UPG envelope and reports the same family signature 5dfeff3c. Its script, normal kernel and normal initrd hashes equal the pinned 1.03 counterparts; its rootfs differs. Thus the 1.02 package does not supply a new resident initrd through those three components. Its differing rootfs is not claimed fully inventoried here. The outer package lists 15 updater/config/license/UPG entries, with no separate resident image found.

Bounded GitHub, Internet Archive and Wayback searches were also recorded. Exact-model public results included manuals and model-ID databases; no independently attributable complete A910 flash image or fwpup executable was recovered. The model database is identity evidence only. A820/A829 homebrew, A50 installers and other platforms' flashing tools were excluded as recovery evidence. Search-result absence is not a global absence claim.

## Remaining safety blockers

The resident fUKt/fURt contents, `/linuxrc_fwup`, actual fwpup executable, and installed-loader binary equivalence remain missing. Therefore the fwpup authentication/decryption boundaries, erase/program/readback order, bad-block handling, persistent-request clearing and power-loss resume/rollback contract remain unverified. A source-visible fatal loop is evidence against assuming automatic recovery, not evidence that every failure bricks the player.

The next qualifying input must be an attributable A916/A918/A919 resident image or complete factory/storage image with a documented origin, hashes and layout. It must first be inventoried as data and matched to these source table signatures and known firmware inputs. No live-device experiment or flashing enablement is justified by the present findings.

Reproduce archive/hash checks without network or target execution using `python tools/firmware_research/collect_resident_sources.py`. This collector only reads already downloaded files and emits the audit JSON; it does not model or exercise hardware recovery.
