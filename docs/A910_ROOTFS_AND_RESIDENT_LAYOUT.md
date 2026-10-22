# A910 1.02 rootfs and resident-image layout follow-up

Date: 2026-10-19. Extends [official source discovery](RESIDENT_SOURCE_SEARCH.md) and [device installation analysis](DEVICE_INSTALL_CHAIN.md). Scope: local archive inspection, hash comparison, static ARM disassembly/decompilation and matching published U-Boot source. No device enumeration, connection, mount, read, reset or write occurred. No Sony executable or target script was executed; no firmware executor was added.

## Complete 1.02 rootfs comparison

The 1.02 rootfs component was decrypted locally from the previously signature-checked official distribution. Its SHA-256 is `27efd3993693f28f303842784516824bb327a564fa91cc2d1c86825c7c95b9de`. The comparison input for 1.03 is the previously pinned original component, SHA-256 `f5ad4b2991ebece66abe6d26ec1f726c56dcd9469ed5888de1e7fd07b11c0557`. Neither was mounted or unpacked onto a target filesystem. Tar regular files were read in memory; link targets were retained as metadata and never followed on the host.

Both archives contain 613 entries and 341 regular files. There are no added/deleted paths. Only eight regular-file payloads differ:

| Changed file | Interpretation bounded by inspection |
| --- | --- |
| etc/firmversion | Sony version declaration changes from 1.02 to 1.03 |
| usr/lib/omf/VERSION | Build/revision provenance text |
| usr/local/bin/GEMINI_REVISION | Build/revision provenance text |
| usr/local/bin/cogemini | Binary content differs |
| usr/local/bin/gemini-ICX1101 | Binary content differs; previously located flag setter examined separately below |
| usr/local/bin/gldump | Binary content differs |
| usr/local/bin/glogd | Binary content differs |
| usr/local/bin/mptapp | Binary content differs |

All entries change archived uid/gid from 0/0 to 505/505; some modes also differ. These explain why a metadata-inclusive diff marks all 613 entries changed. They do not establish resulting installed ownership: the installer/tar options would also have to be considered. Modification-time/owner-name/PAX fields are not compared by this tool; it is a payload/type/link/mode/numeric-owner comparison, not whole-tar byte equivalence.

[Comparison](audits/rootfs-layout-2026-10-19/comparison.json) retains every recorded difference and literal-search hit. The two full member inventories are beside it. Every regular-file byte was searched for fwpup, linuxrc_fwup, linuxrc_inst, the known NVP path/magic, resident table signatures and helper names. Neither archive supplies a file or literal for fwpup or linuxrc_fwup. No regular file begins with the examined gzip/ZIP/xz/bzip2/CPIO archive signatures. This does not exclude arbitrary compression, constructed names, or a component absent from the update package.

The unchanged helpers are important:

- wrldr: SHA-256 `6d1b2f2a0d7c336cdcc9a84667561d101cfec12e2ebc878867dc043c49d061a1`.
- nvpflag: SHA-256 `90aa0b11ef01c0997a1db28faf664d3eed931675e7c272b7232d74ed3591e7ac`.
- apl_conn: same bytes as the prior 1.03 audit, including its known flag setter.

The Gemini setter/code-and-literal window `[0x1a4ffc,0x1a50b4)` differs at only two words, at 0x1a509c and 0x1a50a0, both in its literal pool and both shifted by 24 bytes. Its instruction bytes in the examined range are unchanged. This is a bounded comparison of an existing setter location, not proof that all Gemini behavior or all callers are identical. [Window hashes and words](audits/rootfs-layout-2026-10-19/gemini-setter-comparison.json) retain the result.

## nvpflag supplies a primitive, not a recovered completion clear

The unchanged ARM ELF was decompiled as data. Its startup entry binds main to VA 0x86bc. Main selects a read helper at 0x882c or a write helper at 0x8be8 according to the argument form. Literal-pool bindings independently map the fup selector to `/dev/icx1087_nvp/9`.

The write helper converts a supplied numeric value, writes exactly four bytes and accepts success only when the returned count is four. Negative or short writes return an error. The helper does not prescribe an installation-completion value, enforce the fWUp magic, or contain a dedicated firmware-success clear branch. It therefore demonstrates another available way to change the persistent field, not evidence of when the actual installer does so. No command for performing that write was run or exposed through the application.

All regular-file bytes in 1.02 were additionally searched for helper-name references. Recovered script uses of nvpflag are:

| Script | Recovered selector use |
| --- | --- |
| etc/init.d/gemini | tst read, and tst write of zero |
| etc/init.d/rc.sysinfo | mid read |
| etc/init.d/consstart | tst and gty reads; comment about gty setting |

These scripts are byte-identical in 1.02 and 1.03. The tst write is not a fup clear. No script call to write fup, and no script invocation of wrldr, was recovered. Encoded or indirect calls are not excluded. [Helper references](audits/rootfs-layout-2026-10-19/helper-callers.json) retain the exact lines and offsets.

## wrldr links the resident layout to a separate input set

The ELF's libc startup argument identifies main at VA 0xa194, confirmed by raw ARM instructions before explicitly creating that analysis function in Ghidra. The command's embedded usage text describes three input files in order: U-Boot, firmware-update kernel, firmware-update initrd. This establishes expected inputs; those input image files are not supplied by the rootfs.

The recovered static flow is:

| Stage / VA | Observed behavior |
| --- | --- |
| Main 0xa194; validator 0x9f24 | Read each of the three files first and require XOR32 zero. Read buffers are zeroed for a partial final word. Any validation failure exits before writer initialization. No signature verification or nonempty/minimum-size check was recovered in this validator. |
| Geometry 0x9af8 | Open `/dev/icx1087_nvp/io`; obtain kernel-area start and NAND page/block geometry through ioctls. Define the loader interval from block 1 through kernel-start minus one. |
| Initialization 0x8618; reader 0x9914 | Locate an old lOAd control block and read its partition-table page. Prepare five management records: lOAd, pARt, uBTt, fUKt, fURt. The existing partition page is retained while image tables are prepared. |
| Main; page buffer 0x9178 | Reopen and stream U-Boot first, then update kernel, then update initrd. Buffer into NAND pages; data-block allocation proceeds from the upper loader bound downward. The files are reopened after validation; there is no recovered same-file-handle binding preventing intervening file changes. |
| Page writer 0x933c | Erase at a new data-block boundary, issue write ioctl, then read ioctl. Write/read failures enter block relocation or exit when no space remains. |
| Relocator 0x9604 | Move already written pages to a lower candidate block after erase; failure can select another candidate, or exit on a source-page read error/no remaining space. |
| Finalizer 0x8bb0 | Finish the last image, then search control-block candidates from the lower bound upward. Erase a candidate and write/read five management pages in order. Failed write/read can trigger an erase attempt and advance to the next candidate. |

The ioctl identities agree with the official NVP header: operation 1 write, 2 read, 3 erase, 4 kernel-area start, 12 NAND geometry. These observations establish the order in **wrldr**. They must not be substituted for the missing fwpup implementation or for the UPG install path.

The listings call a successful read ioctl a verification step. No user-space memcmp between write and read buffers was recovered at the examined page/control write sites. Whether lower layers compare content requires a separate driver-path proof; readback success alone must not be described as byte-for-byte verification.

XOR32 can accept empty/zero-filled data and paired changes that cancel; it supplies no publisher authenticity. The fact that this utility writes image data before control tables does not establish atomic publication. A candidate control block is erased before its pages are rewritten; the recovered bounds do not exclude the old control block. A later-page failure can erase that candidate again. No persistent fup clearing or complete power-loss rollback contract was recovered here.

[Helper provenance](audits/rootfs-layout-2026-10-19/helper-provenance.json) binds both ELF hashes and 34 exported static functions. [Literal-pool bindings](audits/rootfs-layout-2026-10-19/elf-data-bindings.json) resolve device paths, table signatures, shared bound variables and messages. Raw instruction listings are also retained for both helpers. Ghidra's synthetic research_entry names identify analyst-created function boundaries, not original Sony symbols. Decompilation is not target execution.

## U-Boot layout: table-directed, not a fixed resident byte offset

The matching official source defines:

- A loader-management block starts with lOAd in the first four data bytes of page zero. The search scans blocks in ascending order and table pages 1 through 127, returning the first readable signature match.
- pARt is a partition-table page; uBTt, fUKt and fURt are image-table signatures. An image table begins with a four-byte signature, a 32-bit page count, then a list of 32-bit block numbers. Image size is a page count, not a byte length.
- For the non-ICX1111 2 KiB source branch, 128 pages form a block. The declared image-table array holds 510 block entries. The loader scans from block 1 up to, excluding, block 488 (40 + 448). This is a search bound, not a claim that resident images are contiguous or that an inspected player has that exact compiled geometry.
- The source also has ICX1111 runtime variants, including a 4 KiB branch with a different loader search limit. Build options and actual geometry remain unverified, so these constants are not converted into device write addresses.
- Image bytes are loaded by following the image table's block list. fUKt/fURt table discovery occurs independently; this routine does not require the two matching pages to come from the same management block or generation. The source search does not choose the highest sequence generation or check a table checksum here.

Consequently, a future attributable full image must preserve enough layout information to distinguish data pages, spare/OOB bytes, bad blocks and management pages. Searching a file for the four-byte fURt string alone would not establish a valid resident initrd. A partial image also cannot be treated as proof of absence outside its captured range.

The source setting reader copies the selected settings into RAM. The recovered `icx1087_set_val.fwup` assignment is a read result, not a NAND clear. Generic U-Boot write/erase command implementations exist, but no recovered call links them to installation-completion clearing of fup. The previously documented RAM BoOt clear remains separate from persistent NVP fWUp.

Source references: the retained [NAND boot source](audits/resident-source-search-2026-10-19/official-source/board/mp201xx/icx1087/icx1087_nand.c.txt), [geometry header](audits/rootfs-layout-2026-10-19/official-source/nand_config.h.txt), [source provenance](audits/rootfs-layout-2026-10-19/layout-source-provenance.json), and [search anchors](audits/rootfs-layout-2026-10-19/u-boot-layout-hits.json). Unrelated board source matches in that search record are explicitly excluded from A910 reasoning.

## Remaining boundary

[Further resident-candidate search](A910_RESIDENT_CANDIDATE_SEARCH.md) records public archive/download-history coverage, additional official source inventories and local capacity-image exclusions. It still supplies no attributable resident payload or fwpup.

The 1.02 rootfs gap is now closed at archive-inventory and byte-search level. It did not reveal `/linuxrc_fwup` or fwpup. wrldr and nvpflag provide useful layout/write primitives but do not reveal their resident installation caller, persistent flag-clear timing/value, or fwpup's erase/program/authentication/recovery sequence. No complete resident image was obtained in this follow-up. Flashing remains unavailable.

Reproduction tools are `compare_a910_rootfs.py`, `collect_rootfs_helpers.py` and `ExportSmallElf.java` under `tools/firmware_research/`. They only inspect local files; the Java script runs inside the static analyzer. Rootfs reader regression tests use synthetic archives and require no Sony files or device access.
