# A918 cover tree investigation

2026-10-17. Static REA/Ghidra investigation of immutable SonicStage 4.4 AuraPACAPI.dll (SHA-256 faad0b0a86cb51d52834989afca83166498b7385d48bf130bd9e38685d27670a). No device writes or Rust changes in this investigation. Pseudocode is recovered analysis, not Sony source. Original evidence lives in sonicstage-tree-rea/.

## Findings

- FUN_6d232ef1 (query interior 0x6d232f68) declares six fields TIT2, TPE1, TCON, TSOP, PICP, PIC0. FUN_6d26518a writes the exact same six tags to field indices 0..5. Its PIC0 write is conditional on owner+0x1c0 != 1. This is consistent with the existing namespace-1 album group structure, but this query does not independently prove its filename.
- Hierarchical constructors FUN_6d241d4f, FUN_6d241ec6, FUN_6d2420b3, FUN_6d242307 all declare four metadata tags at object DWORD indices 0x50..0x53: TIT2, PICP, PIC0 and the four-byte marker at 0x6d30ae40 (raw source bytes XSOT; REA read_bytes confirmation pending). Existing two-field TIT2/XSOT records omit both picture descriptors supported by these Sony implementations.
- FUN_6d241ec6 uses ordering keys TPE1 then TALB; FUN_6d2420b3 uses TCON, TPE1, TALB. This is stronger evidence for the hierarchical tree family than arbitrary PICP string matches. Mapping the artist/album constructor to physical namespace 0x2d still requires its factory or base-constructor argument evidence.
- FUN_6d26733d is a reader for PICP/PIC0, producing property objects with numeric property 0x20015. It does not by itself establish 04CNTINF cover fields.
- No evidence here proves 04CNTINF must receive picture slots. Its current five fields should not be expanded solely from the PICP xref list.

## Evidence caveat

Earlier file sony-rea-PICP-setter-code.c is actually FUN_6d2a3fa0, a generic metadata operation. The actual observed PICP/PIC0 group setter is FUN_6d2431a6 in sony-rea-PICP-code.c. Always use recovered function addresses rather than convenience filenames.

## Next focused query

Decompile FUN_6d2426f4 (base of hierarchical constructors), query xrefs to FUN_6d241ec6, inspect its callers for namespace argument 0x2d. Only after establishing that link is a four-field 03GINF2D candidate directly supported. Preserve unrelated accepted database headers, record identifiers, counts and UTF-16 content, adjusting GPFB lengths/record strides consistently when inserting fields. Candidate-device acceptance remains an independent experiment.

## Follow-up completed

REA read_bytes at 0x6d30ae40 confirms XSOT, followed by PIC0 at +8 and PICP at +16 (evidence ev_296530c6e35bab108e80da50e02aef8d39e77b4e6afc7ee08e535280bb5151ff). The hierarchy base FUN_6d2426f4 first declares TIT2/XSOT, then the four derived constructors replace it with TIT2/PICP/PIC0/XSOT. This explains the two-field versus four-field format family without assuming the two-field version is an invalid database.

The unique direct callsite for artist/album constructor FUN_6d241ec6 is 0x6d26d4b8, inside factory FUN_6d26cc56. REA complete factory evidence ev_0ae9ddae12f55c327c590750fd589b559d65828472adffa7c62cf89002937a82 shows it creates the four hierarchical subclasses and stores them in a collection; it then calls each object vtable+4 initializer. No fixed 0x2d constructor argument occurs. The shared base FUN_6d239492 initializes namespace object+0x14 to -1. Therefore the observed implementation assigns namespace later at runtime; a static mapping to constant 2D is not proven. Existing device tree ordering TPE1→TALB matches the artist/album subclass, making a controlled 2D four-field experiment supported, while its acceptance remains unproven.

Additional convenience queries: tree-initialize.c and tree-initialize-impl.c show shared creation of metadata objects and properties. They do not turn the tree namespace into a static constant. No 04CNTINF or 00GTRLST cover flag requirement was established. Last attempted pseudo query 0x6d24a355 was rejected as an unresolved procedure; its error is retained and is not evidence of behavior.

All native sessions were closed after saving evidence. No device or Rust files were modified by this sub-investigation.
