# Complete Sony firmware/recovery/backup catalog review

Date: 2026-10-19. Follow-up to [the A910 resident-candidate search](A910_RESIDENT_CANDIDATE_SEARCH.md). This closes the previously unreviewed 558 records for the specific archive query below. Public metadata was retrieved and analyzed locally; no device access, target-code execution, firmware payload installation or flashing enablement occurred.

## Coverage proof

Query: `(title:sony OR creator:sony) AND (title:firmware OR title:recovery OR title:backup)`.

The earlier 200-result response did not specify an ordering. To avoid assuming its page boundary remained stable, this pass retrieved all four pages with explicit `identifier asc` ordering, then compared identifier sets with the saved earlier response.

| Check | Result |
| --- | ---: |
| Reported total on each of four pages | 758 |
| Page sizes | 200 / 200 / 200 / 158 |
| Returned identifiers | 758 |
| Unique identifiers | 758 |
| Earlier response identifiers present | 200 of 200 |
| Earlier identifiers missing | 0 |
| Newly covered identifiers | **558** |

The archive preserves identifier case but sorts case-insensitively; the coverage checker verifies that ordering. Cached pages are hash-checked against their request records before offline re-analysis. A failed initial case-sensitive ordering check produced no completion report; it was corrected to match the observed archive ordering and rechecked against the same saved responses without another download.

[Coverage certificate](audits/sony-catalog-complete-2026-10-19/coverage.json) binds the earlier response SHA-256, counts and set difference. [Page requests](audits/sony-catalog-complete-2026-10-19/page-requests.json) preserve exact URLs and response hashes. The [catalog index](audits/sony-catalog-complete-2026-10-19/catalog-index.json) and [per-record triage](audits/sony-catalog-complete-2026-10-19/triage.json) retain all 758 identifiers and their review outcome. Raw response bodies remain in the ignored local research cache.

This proves complete **catalog metadata** coverage for this query snapshot. It is not a complete search of every archive collection, nested payload or source on the internet.

## Remaining 558 records reviewed

Screening included identifiers, titles, creator, subject and descriptions for A910/A916/A918/A919 forms, ICX1101, resident entry/tool names, wrldr input-role names and fUKt/fURt text. Broad Walkman/NW signals and generically named backups were reviewed separately, rather than requiring an exact model-name hit to inspect a lead.

| Final category | Newly reviewed records |
| --- | ---: |
| VAIO/PC recovery, drivers and related material | 487 |
| PlayStation/PSP/PSX and related backups | 35 |
| Phones and smartwatch firmware | 17 |
| TV, monitor, tuner and vehicle AV firmware | 7 |
| Unrelated media or Windows application backups | 6 |
| Cameras | 2 |
| MPD-AP20U optical-drive/player firmware | 2 |
| Other identified Sony devices | 2 |
| **Total** | **558** |

The category assignments describe catalog attribution. They do not authenticate the payloads or claim a binary-format match. No entry was qualified as an A916/A918/A919 resident image, factory package or complete storage backup.

## Expanded metadata and file-list review

Thirty-seven item-level metadata responses were additionally acquired and hashed. These cover the ambiguous initial matches, broadly named backups, two misleading signature-text hits, and the Walkman-related leads (including relevant records from the earlier 200). Their catalogs list 736 file records, including archive-generated metadata files. The listed filenames were screened for the target family/platform, resident entry/tool and input-role terms. No target filename match was found. None of these payloads was downloaded or executed.

Important exclusions:

| Lead | Evidence obtained | Disposition |
| --- | --- | --- |
| ap20update / mpd-ap20u | Item metadata identifies MPD-AP20U CD-RW/DVD-ROM/MP3 optical-drive player; file catalogs list its updater/PAC package | Another product; no A910 payload attribution |
| sonynwze360_1.0.0.firmware | File catalog explicitly lists NWZ-E360/E370/E380 SB firmware files | Other Walkman models; no A910 image admitted |
| se_w200i_firmware | Item metadata and RAR filename identify Sony Ericsson W200i | Phone firmware, not an A910 candidate |
| Sony Recovery DVD EU-AL6-06A-F | Full metadata identifies SVS1311/SVS1511 VAIO Windows 7 recovery discs | PC recovery media |
| hddfiles / Hard Drive Software Backup Archive | Full description and file list identify Windows application installers archived from external drives | Application backup, not acquired Walkman storage |
| Mylo COM-1 / Rolly / Dash / HUIS / nasne / BVM | Item metadata names their distinct products and lists associated packages | No A910 model attribution; recovery behavior not transferred |

Primary catalog references include [MPD-AP20U](https://archive.org/details/mpd-ap20u), [NWZ-E360 family firmware](https://archive.org/details/sonynwze360_1.0.0.firmware), [VAIO recovery DVD](https://archive.org/details/sony-rdvd-eual606af) and [the application backup](https://archive.org/details/hddfiles). Their uploader descriptions establish why these leads were excluded; they do not independently certify Sony authenticity. [Expanded file-list and response-hash evidence](audits/sony-catalog-complete-2026-10-19/metadata-review.json) preserves the metadata URLs, hashes and named files.

## fUKt/fURt layout screening boundary

Two case-insensitive `fURt` text matches came from the English word **further** in VAIO descriptions. They are neither exact-case fURt signatures nor bytes from NAND management pages. [False-positive review](audits/sony-catalog-complete-2026-10-19/false-positive-review.json) records them explicitly.

No candidate passed the A910 provenance stage, so no new binary entered the lOAd/pARt/fUKt/fURt layout verification stage. The matching official U-Boot source remains the basis for what a future candidate must contain: valid management context, bounded page-count/block-list tables and reconstructable resident kernel/initrd bytes with known capture geometry/OOB handling. A filename or text occurrence alone cannot establish that layout.

It would be inaccurate to claim that all 758 payloads were scanned for table signatures or that all their nested archive contents lack fwpup. This pass completes metadata review and the specified additional item file lists, not arbitrary other-model payload reverse engineering.

## Installation-chain status

No newly recovered `/linuxrc_fwup`, fwpup executable or completion/failure flag-clear caller was obtained. The prior findings remain:

- U-Boot has a matching-family source-level consumer for persistent fWUp and selects separate resident tables.
- wrldr accepts independent U-Boot/update-kernel/update-initrd inputs; the ordinary update rootfs does not supply those resident input files.
- nvpflag provides a four-byte FUP access primitive, but no recovered resident call binds it to successful/failed installation completion.
- RAM BoOt clearing still does not establish persistent NVP fWUp clearing.

There is no new basis for asserting fwpup erase/program/readback order, interrupted-update recovery or safe flashing. No replacement resident image was synthesized, and no other model's image or recovery logic was applied.

## Reproduction and validation

`tools/firmware_research/complete_sony_catalog.py` retrieves the bounded public catalog, rejects inconsistent totals/duplicate coverage/missing prior identifiers, and supports `--cached` to repeat checks with no HTTP. `tools/test_sony_catalog.py` uses synthetic local responses to test preserved-case ordering, remaining-record counts, duplicate rejection, changed totals, missing prior records and altered response hashes. These checks concern catalog accounting, not hardware recovery.

The application's prerelease version remains 0.4.0-alpha.5 because this pass changes research tooling/evidence/documentation only. No native or frontend firmware route was added or enabled.
