# Sony binary inventory

Static inventory on 2026-10-17. No Sony binary was executed, registered or installed during this sub-investigation. No device was modified. Strings establish candidate locations, not executed behavior.

SonicStage artifacts were extracted from the parent investigation’s 4.4 Japanese installer ISO. See parent provenance for download and signature verification.

## Priority targets

### C:\Users\VicLi\Documents\Codex\2026-10-17\c-program-files-x86-sony-music\work\sonicstage-openmg\AuraPACAPI.dll

SHA-256: `faad0b0a86cb51d52834989afca83166498b7385d48bf130bd9e38685d27670a`; bytes: 1609728.

- Original file offset `0x10ae48` (ascii): `PIC0`
- Original file offset `0x10ae50` (ascii): `PICP`
- Original file offset `0x11201c` (utf-16-le): `\OMGAUDIO\01TREE%02X.DAT`
- Original file offset `0x112378` (utf-16-le): `\OMGAUDIO\00GTRLST.DAT`
- Original file offset `0x1123d8` (utf-16-le): `\OMGAUDIO\04CNTINF.DAT`
- Original file offset `0x1128ec` (utf-16-le): `\OMGAUDIO\01TREE01.DAT`
- Original file offset `0x118888` (utf-16-le): `\OMGAUDIO\00GTRLST.DAT`

### C:\Users\VicLi\Documents\Codex\2026-10-17\c-program-files-x86-sony-music\work\sonicstage-openmg\FrankPACAPI.dll

SHA-256: `845998624cc49476fb8e427c307ea17e3d4c11af4ff2e1f029ac476702fd1bc9`; bytes: 1323008.

- Original file offset `0xca59c` (ascii): `PIC0`
- Original file offset `0xca5a4` (ascii): `PICP`
- Original file offset `0xca568` (utf-16-le): `\OMGAUDIO\01TREE%02X.DAT`
- Original file offset `0xca8bc` (utf-16-le): `\OMGAUDIO\00GTRLST.DAT`
- Original file offset `0xca91c` (utf-16-le): `\OMGAUDIO\04CNTINF.DAT`
- Original file offset `0xcaf98` (utf-16-le): `\OMGAUDIO\01TREE01.DAT`
- Original file offset `0xcff64` (utf-16-le): `\OMGAUDIO\00GTRLST.DAT`

### C:\Users\VicLi\Documents\Codex\2026-10-17\c-program-files-x86-sony-music\work\sonicstage-openmg\MsProPACAPI.dll

SHA-256: `6f7a57938a2b557cda9716877013683cb9e40b5ae3c09cf400676141756c23c1`; bytes: 827392.

- Original file offset `0x6ca60` (ascii): `PIC0`
- Original file offset `0x6ca68` (ascii): `PICP`
- Original file offset `0x70377` (ascii): `p\OMGAUDIO\04CNTINF.DAT`
- Original file offset `0x7047f` (ascii): `p\OMGAUDIO\01TREE01.DAT`
- Original file offset `0x70593` (ascii): `p\OMGAUDIO\00GTRLST.DAT`

### C:\Users\VicLi\Documents\Codex\2026-10-17\c-program-files-x86-sony-music\work\sonicstage-openmg\EA3FringeLib.dll

SHA-256: `72f1e8464fc2d9df53eb806e3bac1525efa7e35ff7e6bdfaedd302b6d2501ffa`; bytes: 151552.

- Original file offset `0x187a4` (ascii): `APIC`

### C:\Program Files (x86)\Sony\Music Center\Sony.Earth\EA3FringeLib.dll

SHA-256: `7c438b8036485554113d55838379ca798986e427ede8458fb22d7d399a6e0883`; bytes: 149568.

- Original file offset `0x18e5c` (ascii): `APIC`

### C:\Users\VicLi\Documents\Codex\2026-10-17\c-program-files-x86-sony-music\work\sonicstage-openmg\id3parser.dll

SHA-256: `e524d4b8de81575de4c0bc07addfa483837408dbe765ecd983e4699edbedbd2e`; bytes: 135168.

- Original file offset `0x14744` (ascii): `APIC`
- Original file offset `0x1ab9a` (ascii): `?GetAPIC@CId3parser@@AAEJAAV?$vector@V?$shared_ptr@VCPicture@CId3parser@@@boost@@V?$allocator@V?$shared_ptr@VCPicture@CId3parser@@@boost@@@std@@@std@@@Z`
- Original file offset `0x1b962` (ascii): `?SetAPIC@CId3parser@@AAEJABV?$vector@V?$shared_ptr@VCPicture@CId3parser@@@boost@@V?$allocator@V?$shared_ptr@VCPicture@CId3parser@@@boost@@@std@@@std@@@Z`
- Original file offset `0x1b9fb` (ascii): `?SetAPIC@CPicture@CId3parser@@QAEJPBEJ@Z`

### C:\Program Files (x86)\Sony\Music Center\Sony.Earth\id3parser.dll

SHA-256: `698681184fd635d16aef415072a1e7e2609a525c248b229e1c8e4aebc5bb1a56`; bytes: 180288.

- Original file offset `0x24f03` (ascii): `?GetAPIC@CId3parser@@AAEJAAV?$vector@V?$shared_ptr@VCPicture@CId3parser@@@tr1@std@@V?$allocator@V?$shared_ptr@VCPicture@CId3parser@@@tr1@std@@@3@@std@@@Z`
- Original file offset `0x266d1` (ascii): `?SetAPIC@CId3parser@@AAEJABV?$vector@V?$shared_ptr@VCPicture@CId3parser@@@tr1@std@@V?$allocator@V?$shared_ptr@VCPicture@CId3parser@@@tr1@std@@@3@@std@@@Z`
- Original file offset `0x2676b` (ascii): `?SetAPIC@CPicture@CId3parser@@QAEJPBEJ@Z`

### C:\Program Files (x86)\Sony\Music Center\Sony.Earth\Msc.dll

SHA-256: `ff028f50c1127101bd89acbc219cbf67a37e4ada7d50f76599b1d2ba75db7343`; bytes: 2035264.

- Original file offset `0x1d4a5b` (ascii): `020P0^0o0u0`

## Interpretation and next REA queries

- AuraPACAPI combines ordinary OMGAUDIO database filenames, `PIC0` / `PICP`, extended artist tables and metadata identifiers; prioritize its string references for the old device format. FrankPACAPI has the same core database and image markers and is a useful independent implementation comparison. MsProPACAPI may target a different device family: do not adopt it without matching model evidence.
- Music Center Msc.dll contains ID3 property markers, but the bounded plain-string scan did not find `PICP` or `PIC0`; absence from this scan is not proof of absent runtime support. It can still be a modern mass-storage transport target.
- `20P` matches appearing inside arbitrary printable byte runs (for example Msc.dll 0x1d4a5b) are possible false positives, not confirmed cover directory names.
- REA portable PE resource inspection of AuraPACAPI: evidence `ev_bfc825752dcfd944be88e3b4b28e16f0b7caf73510663bc1a12e4e0d07022fe3`, provider rea-pe-resources v1; complete 4 resources (REGISTRY, TYPELIB, RT_STRING, RT_VERSION). Registry payload at 1500352 (654 bytes) identifies AuraPACAPI.AuraPacManager and CLSID CDB56660-8038-4950-A7F6-6C5B9911F4A5.
- LLVM PE export inspection establishes AuraPACAPI exposes only DllCanUnloadNow, DllGetClassObject, DllRegisterServer and DllUnregisterServer; field semantics require decompilation, not export-name inference.
- Ghidra through REA: query search_strings PICP/PIC0/04CNTINF, then xrefs and procedure_pseudo_code. Native analysis is in progress; do not cite a format layout until complete.

## ID3 interface observation

SonicStage's id3parser.dll exports both GetPIC and GetAPIC, and GetString uses a vector of UTF-16 strings. These names/signatures establish separate ID3v2.2 picture and APIC candidates, plus multi-valued text representation. They do not establish correct handling of every encoding or malformed tag. A replacement should explicitly test v2.2 PIC, v2.3/v2.4 APIC, UTF-16 BOM, UTF-16BE, UTF-8 and multi-valued text instead of reducing metadata to one lossy byte string. Export table retained in id3parser-exports.txt.

## REA Ghidra native findings

REA through Ghidra 12.1.4 completed default analysis for AuraPACAPI.dll. Complete original Evidence and separate convenience pseudocode text live in `sonicstage-rea/`. All addresses below are analysis addresses; the PE image base is 0x6d200000. Pseudocode is recovered analysis, not original source.

- `search_strings('PICP')` returned empty because Ghidra did not define these four bytes as string data. Direct xrefs to 0x6d30ae50 found 19 references. This is an example of why empty defined-string search cannot establish absence.
- `search_strings('20P')` observed `20P??` at 0x6d312544 and `20Pcc\1Gssssxx.JPG` at 0x6d318ed8. The latter has one direct reference, inside recovered function FUN_6d2b4c20. This is stronger evidence than a raw printable-byte match.
- FUN_6d2b4c20 starts with `20Pcc\1Gssssxx.JPG`, replaces positions 3..4 using `%02X` on parameter 3 masked with 31, and positions 8..11 using `%04X` on parameter 3. Parameter 1 equal to 0 produces suffix P0; other values use `%d%d` with `(parameter1 - 1) / 2` and a parity-derived value. Parameter 2 greater than 1 replaces `1G` with two characters derived from its nibbles plus 0x47. Finally it prefixes `\OMGAUDIO\`. Parameter meanings are not yet established; in particular parameter 3 is not asserted to be a song index.
- FUN_6d2431a6 calls the common setter with tags PICP and PIC0 and UTF-16 strings. The common writer FUN_6d2a3e10 computes a slot using `fieldIndex * 0x80 + 0x10 + recordSize * recordIndex`, copies the first four tag bytes, writes 2 at slot+5, zeroes payload storage, copies at most 0x78 bytes of UTF-16 text at slot+6, and conditionally byte-swaps UTF-16 code units. This demonstrates the fixed-width metadata field representation in this path. It does not yet establish which file/table owns the passed record instance or the physical-device meanings of PICP versus PIC0.

Original format constants read from immutable source bytes, with PE section mapping checked: file offset 0x118f94 UTF-16 `%03d%03d`; 0x1172e0 `%02X`; 0x118f78 `%04X`; 0x118f6c `%d%d`. These support the recovered swprintf calls.

No output of this analysis has been written to the player. Device acceptance and cover display remain separate validation obligations.

## Cover reader/writer call path

FUN_6d23974f is an observed writer-side caller of FUN_6d2b4c20. It iterates a picture vector, opens each nonempty source file for reading, calls FUN_6d2b46c0 to inspect JPEG markers/dimensions, rejects either dimension above 999, then passes:

1. Output picture counter (zero-based) as parameter 1.
2. Owner object UTF-16-sized value at offset +0x14 as parameter 2; its semantic meaning is still unknown.
3. A value obtained via FUN_6d20b398 from the input and validated within 1..65535 as parameter 3; its semantic meaning is still unknown.

The writer later invokes a COM method at vtable+0x6c with source/destination material. No image scaling call was observed in this caller; the image's input dimensions feed the naming descriptor. This does not prove scaling cannot happen upstream.

Suffix counter samples directly derived from the recovered arithmetic are: 0 -> P0, 1 -> 01, 2 -> 02, 3 -> 11, 4 -> 12. For parameter2 <= 1, the template's 1G is left unchanged. These are independently implementable naming rules; do not silently assign a track ID or album ID meaning to parameter3.

PICP versus PIC0: setter FUN_6d2431a6 places the first returned picture string in PICP and joins subsequent strings using UTF-16 `|` (immutable original bytes at file offset 0x10d584) in PIC0. Reader FUN_6d25ffa6 finds the first semicolon in PICP, takes the following substring and prefixes `\OMGAUDIO\` to find the file. Continuation FUN_6d26031c splits PIC0 on `|` and applies the same semicolon rule to each entry. Thus PICP is the primary picture descriptor; PIC0 contains additional picture descriptors. It is **not established as a thumbnail-only field**.

The builder formats two image dimensions as `%03d%03d`, and combines this prefix, a semicolon and a relative path. The reader's `substring(firstSemicolon+1)` independently confirms the descriptor's path comes **after** the semicolon. Expected descriptor shape is `WWW HHH` without a space, followed by `;20Pcc\1Gssssxx.JPG`; width-versus-height ordering still requires checking the dimension reader's output argument order, as decompiler argument recovery is imperfect.

## Follow-up: namespace, record size, and terminator verification

The apparent UTF-16 length discrepancy is resolved by assembly. FUN_6d2a3e10 at 0x6d2a3e70..87 clamps copying to 0x78 bytes (60 UTF-16 code units). At 0x6d2a3e90..9a it clears storage starting at slot+6 using 30 STOSD writes (120 bytes), followed by one STOSW (2 bytes). Thus the 0x80-byte slot reserves 122 bytes after its six-byte prefix: **60 UTF-16 code units plus a zero terminator**. This agrees with the Rust writer's intended boundary. Evidence: sony-rea-field-writer-assembly.json.

Namespace mapping: FUN_6d264bb2 iterates tree objects and calls vtable+0x18 with an output pointer. Assembly 0x6d264c2f..41 stores this result at EBP-0x2a8. The same value is pushed to format `01TREE%02X.DAT` at 0x6d264cee and `03GINF%02X.DAT` at 0x6d264e69. One observed vtable containing the shared picture writer FUN_6d23974f starts at 0x6d30b098; its +0x18 entry is 0x6d233128. REA read_bytes of that entry shows `8b4114 83f8ff ... 8b4c2404 8901 33c0 c20400`: it reads owner+0x14, returns an error for -1, otherwise writes this value to the supplied output and returns zero. This connects the owner+0x14 namespace passed to the picture filename builder with the TREE/GINF filename suffix for this observed implementation. Namespace 1 maps to 03GINF01 and the unchanged `1G` picture basename prefix. This chain does not mean every file uses namespace1.

The picture writer's parameter3 is populated through helper FUN_6d20b398 requesting numeric property **0x50018** from the input item; assembly at 0x6d2397ef..807 shows the property ID explicitly. The helper retrieves a property vector through COM and reads the first item's numeric value via vtable+0x24. The caller checks it is within 1..65535. The property enum's source-level name is not established, so mapping it to a physical zero-based GPFB record index remains unverified.

03GINF01 has constructor FUN_6d2b4140; it assigns GPIF and GPFB section tags. The generic file-size estimator FUN_6d264bb2 uses `(fieldCount * 0x80 + 0x10) * groupCount + 0x30` for GINF, and when loading an existing file reads the actual record length from a parsed descriptor using FUN_6d29c3f0 (big-endian ushort at that descriptor+0x10). **Do not hardcode a new field count or reuse group-id as record index without parsing the existing GPFB/group index.** This investigation did not establish the complete group-id-to-record-index mapping. A cover-only patch should preserve the accepted file's headers, actual field count, record length and all unrelated slots, and locate PICP/PIC0 in the identified existing record.
