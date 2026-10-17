# Post-update device baseline

The user confirmed on 2026-10-17 that the A918 firmware update was complete and real-device testing could resume. The exact installed version has not been independently read. This confirmation is not evidence that the artwork problem changed.

The read-only snapshot is `data/device-observations/post-firmware-1792248821086586700/OMGAUDIO`; `comparison.json` records its complete SHA-256 manifest. Compared with the second installed artwork probe, there were no changed, added or removed files. The accepted OMA audio hash is unchanged. No third artwork probe has been installed.

Both previous probes played normally but showed no artwork according to the user. The first populated existing PICP/PIC0 fields in 03GINF01; the second added the Sony-observed TIT2/PICP/PIC0/XSOT field layout to 03GINF2D. Neither experiment establishes that this firmware reads those descriptors for the playback screen.

The next investigation follows the actual firmware's artwork metadata request and GINF readers. The `STD_TPE1_THUMBNAIL` string is part of a structure passed to `/dev/mgscsi` via ioctl in AdtMgScsi_setup; it is not sufficient evidence to replace the 02TREINF tree marker. MP3Parser's getPictureOffset handles embedded ID3 images and must not be mistaken for the OMA database artwork path.

The firmware project is an immutable research input. All analysis artifacts are saved in this application's research directory; no firmware update or firmware modification is performed here.
