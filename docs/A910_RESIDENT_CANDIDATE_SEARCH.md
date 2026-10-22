# A910 resident image, factory package and backup candidate search

Date: 2026-10-19. Continues [wrldr/rootfs analysis](A910_ROOTFS_AND_RESIDENT_LAYOUT.md). Work was limited to public HTTP metadata/source retrieval and local file inspection. No player was enumerated, connected, mounted, queried, reset or written. No target executable/script was executed. Application firmware flashing remains unavailable.

## Result

Coverage update: [the complete Sony catalog review](SONY_CATALOG_COMPLETION.md) now covers all 758 unique query records, including all earlier 200 and the remaining 558. It also reviews 37 item-level file catalogs; no qualifying A910 resident payload was recovered. The first-page limitation below describes the earlier pass and is superseded for this specific query.

This pass did **not** recover an attributable A916/A918/A919 resident image, factory package or complete storage backup. Therefore it does not recover `/linuxrc_fwup`, fwpup, or the installation-completion/failure caller that clears persistent fWUp. The existing source-level flag consumer and wrldr writer evidence remain valid, but no new binary closes the resident installation chain.

The search now has a retained [candidate ledger](audits/resident-candidates-2026-10-19/candidate-ledger.json), [catalog requests](audits/resident-candidates-2026-10-19/catalog-requests.json), [catalog results](audits/resident-candidates-2026-10-19/catalog-results.json), [GitHub search results](audits/resident-candidates-2026-10-19/github-search.json) and additional official-source inventories. Negative results below are bounded to these requests and input files, not a claim that no image exists anywhere.

## What the three wrldr arguments actually identify

The embedded usage fields UBOOT_FILE, FUKERNEL_FILE and FUINITRD_FILE are argument placeholders, **not recovered release filenames**. Searching those exact words can find a utility's usage text but cannot establish that Sony published images under those names. The only recovered matching U-Boot source reference to the resident entry is its `/linuxrc_fwup` boot argument; no wrldr invocation with concrete three-file paths was found in that archive.

fUKt/fURt identify management-table roles. They are neither model authentication nor resident-image filenames. The prior analysis establishes that the payloads are selected by NAND block lists and page counts. Four-byte signature hits in a binary, source listing or extracted rootfs cannot themselves qualify a backup as containing the actual resident images.

## Official-source branch extended

The [Sony open-source page explicitly listing NW-A916/NW-A918/NW-A919](https://oss.sony.net/Products/Linux/Audio/NW-A916.html) also links BusyBox, sysvinit and dosfstools sources. These were downloaded into the ignored research cache and checked against the official server's Content-Length and Content-MD5; fresh SHA-256 values were retained.

| Source package | Verified bytes | Regular files scanned directly |
| --- | ---: | ---: |
| busybox-1.1.0.tar.gz | 1,695,886 | 914 |
| sysvinit-2.85.tar.gz | 92,758 | 59 |
| dosfstools-2.11.local.tgz | 73,123 | 8 |

The dosfstools package additionally contains a nested source tarball. Recursing into that archive brings the total to **1,018 regular member records**. None contains the searched literals fwpup, linuxrc_fwup, FUINITRD, FUKERNEL, wrldr, fUKt or fURt. This closes an additional source-package lead; it does not exclude arbitrary encoded names or establish contents of unacquired firmware. No build/install step was run.

[Download hashes](audits/resident-candidates-2026-10-19/official-extra-source.json) and [complete scanned member records](audits/resident-candidates-2026-10-19/extra-source-members.json) preserve the result. Content-MD5 is a transfer check, not an independent authenticity signature.

## Public archive and download-history coverage

The read-only Internet Archive/Wayback queries returned:

| Query scope | Observed result | Interpretation |
| --- | --- | --- |
| Exact NW-A916/A918/A919/A910 terms | 16 results | Catalog titles/types identify manuals, not candidate factory images |
| Compact NWA916/NWA918/NWA919 and NW_A91X forms | 0 results | No candidate in this metadata query |
| ICX1101/FUINITRD/FUKERNEL/linuxrc_fwup | 0 results | No candidate in this metadata query |
| fwpup | 0 results | No candidate in this metadata query |
| Walkman software title/description | 89 results, all returned | Reviewed catalog; five potentially relevant application/disc item metadata records inspected further |
| Broad Sony firmware/recovery/backup | Earlier pass: 758 results, first 200 returned; [follow-up completes all 758](SONY_CATALOG_COMPLETION.md) | Full metadata coverage now established for this query; not a global or payload-content absence proof |
| Historical Sony A910 download directory | Two distinct archived URLs | Ordinary 1.02 and 1.03 updater distributions already inspected |

The CDX response's length column describes an archive record and must not be treated as the downloaded executable's byte length or a new package variant. The recorded URLs are the existing NW-A910_V1_02.exe and NW-A910_V1_03.exe. No new factory/resident distribution was exposed by this bounded directory query. The old support-page wildcard and source-download wildcard queries returned no captures; this does not contradict live availability of the previously recovered official source files.

The five inspected archive item metadata records were:

- [Walkman Files](https://archive.org/details/setup_20240510): uploader identifies NWZ-E353. Excluded as another model.
- [wmsoft_ww](https://archive.org/details/wmsoft_ww): described as PC media-manager/conversion software, with no A910 resident-image attribution.
- [SonicStage Digital Archive](https://archive.org/details/jp.sony.omgjbox): application software collection; inspected file catalog supplies no A910 resident-image attribution.
- [Media Manager for WALKMAN 1.2](https://archive.org/details/media-manager-for-walkman-1.2): described as a Media Manager/Content Transfer/WMP disc, with no A910 resident-image attribution.
- [Sony Walkman CD-ROM](https://archive.org/details/sony-walkman-cd-rom): described as an MP3 Conversion Tool/Napster/WMP disc, with no A910 resident-image attribution.

These five metadata responses were saved and hashed. Their disc/application payloads were **not** downloaded or executed; unexamined payload contents are not claimed absent. They remain unqualified rather than being relabeled factory firmware because an item title says Walkman. Sony's [A910 product page](https://www.sony.jp/walkman/products/archive/NW-A910_series/) describes the accompanying disc as application software, which likewise does not establish a service/factory image.

Bounded GitHub searches of model/platform names, wrldr argument placeholders, linuxrc_fwup and fwpup produced no qualifying independent image. Several short terms generated unrelated substring matches; these are retained as search limitations, not firmware evidence. Hits in this project's own published research are not independent corroboration. Model-ID databases and music-sync projects do not supply the missing resident bytes. The [A820 project's own description](https://raw.githubusercontent.com/manuaguirre9/nw-a820-homebrew/main/README.md) identifies development on A829 and no bundled Sony files; its installation/recovery behavior was not transferred to A910. No author was contacted and no issue/comment/message was posted.

## Local candidate triage

A bounded local scan covered the current analysis material plus the sibling firmware project's work, firmware and releases directories. Symlinks/junctions were not followed in the local image scan. The scan of image/backup/archive extensions larger than 20 MiB found ten 48 MiB image candidates in the sibling work directory. Every one has an ext2 superblock header. Their locations and the local capacity-generation script identify rootfs allocation/capacity fixtures, not acquired NAND backups. Hashes and header observations are retained in [local image triage](audits/resident-candidates-2026-10-19/local-image-triage.json). These generated images do not add resident data missing from their original rootfs inputs.

This size/extension pass is not an exhaustive content scan of every local file; smaller/differently named inputs are covered only where previous rootfs/UPG/source inventories explicitly examined them. The sibling project's inputs and deliverables were not modified. Its capacity scripts were read as text, never run in this pass.

## Evidence still required to advance the chain

An acquired candidate still needs all of the following before its contents can change the safety conclusion:

1. Attributable origin for A916/A918/A919: original publisher/distributor context or a documented backup acquisition, plus preserved bytes and hashes. A checksum alone proves neither model nor source.
2. A known capture/container layout, including whether NAND spare/OOB data, bad-block treatment and truncation are represented. No physical address or geometry is inferred from another model.
3. Coherent lOAd/pARt and fUKt/fURt tables with bounded page counts/block lists, and the actual reconstructable resident payloads. Merely finding signature text is insufficient.
4. An offline inventory exposing the resident init entry, fwpup implementation and the callers that change persistent FUP on success/failure. RAM BoOt clearing is still a different operation.

This pass supplies additional provenance and eliminates misleading candidates; it does not meet those requirements. No replacement resident image was synthesized, no other model's image was admitted, and no flashing or hardware experiment was authorized by the findings.

The new `tools/firmware_research/search_resident_catalogs.py` performs bounded public metadata GETs and saves raw response hashes. It does not fetch device state, mount images, execute target code or enable any application firmware route.
