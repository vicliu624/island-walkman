# Local analysis of Music Center 2.7.3

2026-10-17. REA 6.5.0 and Ghidra 12.1.4 on Windows 11 x64; native targets are PE32/x86. The exact original crash trigger remains unidentified.

Windows logs repeatedly reported access violation `0xc0000005` in `mediacore.node`, offset `0x0082a5d0`. This identifies a crash location, not the cause. REA static JavaScript analysis examined 866 related files; module/semantic coverage was partial and a semantic node cap was reached.

Root evidence: `ev_c096638b1a7329ca418a7f1050f51971affa532891b6bc63f476eb61a2667387`. Directory SHA-256: `1bc13f62e0fe7d614542d2e0b528fb7d3320a928f0aa6cd6b487d9207ba2a2e2`.

The shipped media-core wrapper exposes PdManager, PdStorage and AudioConvert. MediaPdStorage forwards through context/remoteCall. Static edges were not treated as execution traces. A getPdManager seed had no match; an exact MediaPdStorage seed found one node without traversable edges. Partial coverage cannot establish absence of support.

The installed Msc.dll SHA-256 was `ff028f50c1127101bd89acbc219cbf67a37e4ada7d50f76599b1d2ba75db7343`. A defined-string search for OMGAUDIO/PICP/PIC0/JACKET/COVER/NWWM matched only an unrelated error string. This does not exclude raw-byte or dependency support. Evidence: `ev_2895449029bee146a100a9cf46dc4e8b32ff40e57e4882111529fd1b7621a8b18`.

Recovered function `0x634ea520` checks `.ssthm`, `.jpeg`, `.jpg`, `.jpe` and `.jfif` through a shared checker. Evidence: `ev_ddd72f24d741e30140b2d98cd9c50d817046eb40c4e5d3e3667bb74b0ba172224`. The shared function at `0x634ea230` uses `_wcsicmp` and PathFileExistsW. Evidence: `ev_629edb0550c3238e7a6fb7f8d0e64bd57f0e9a750e32033f0faa3fd0d2ddf6e3`. These establish path validation, not the A918 music artwork layout.

Artwork evidence instead comes from original SonicStage binaries and the firmware reader. See [Sony inventory](sony-inventory.md). Raw recovered listings and Sony binaries remain local research inputs and are excluded from distribution.
