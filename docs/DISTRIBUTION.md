# Desktop distribution and updates

## Application lifecycle

Tauri and the music service share one process. Normal startup opens an independent window with bundled assets. Repeated launches focus the running instance. Closing the last window exits on Windows, Linux and macOS. The optional `--headless` mode retains the research HTTP interface.

## Installer choices

Windows NSIS installs for the current user. The Start Menu page offers an option to skip shortcuts. The final page independently controls the desktop shortcut. Passive updates retain previous choices. `/S /NS /D=<directory>` installs silently without creating shortcuts; CI uses an isolated runner directory. The customized template derives from Tauri and retains its upstream licenses.

DEB installation integrates with the Linux applications menu. AppImage runs without package-manager installation. macOS DMG installation follows the normal drag-to-Applications flow; Finder manages Dock aliases. Windows shortcut options do not apply to Debian or macOS.

## Publishing

1. Set the same semantic version in Cargo.toml, tauri.conf.json and ui/package.json; refresh both lockfiles.
2. Update the English changelog and run checks.
3. Configure the repository Actions secret `TAURI_SIGNING_PRIVATE_KEY`. The initial key has an empty password.
4. Push the commit to main and wait for the Windows x86-64, Debian x86-64, macOS Apple Silicon and macOS Intel package jobs to pass.
5. Push a `v<version>` tag. The release workflow first looks for a successful main-branch build of that SHA. If only the release workflow, manifest generator, its tests or this distribution guide changed afterwards, it may reuse the last successful ancestor build. Every other changed path rejects reuse. Thus application code, UI, versions, icons, dependencies, installer templates and build configuration must match the tested build.
6. The publisher records both commits in build-provenance.json, normalizes filenames before generating URLs and checksums, and validates five updater targets (including separate DEB and AppImage entries). Assets upload into a draft; the Release becomes public and latest only after uploads finish. GitHub's upload filename rewriting cannot invalidate the generated URLs.

A partial build cannot publish a partial feed. Publish a new version for changes instead of replacing existing assets. Pull requests without the signing secret build unsigned installers and never publish releases. For local unsigned packages, pass `--config '{"bundle":{"createUpdaterArtifacts":false}}'` to the Tauri build command.

## Signing keys

The public key is committed in tauri.conf.json. The private key is stored at the ignored local path `data/updater-signing.key` and in the repository Actions secret. Preserve an independent secure backup. Losing it prevents publishing updates accepted by existing installations. Never commit it, print it in logs or include it in installers. Do not rotate the public key without a migration plan.

Updater signatures are separate from Windows Authenticode and Apple Developer ID. Windows packages are not Authenticode-signed. macOS Developer ID signing and notarization credentials are not configured.

## Update behavior

Settings offers an explicit check, release notes, download progress and installation. Windows verifies the setup EXE and invokes passive update mode. macOS verifies an APP archive and replaces the application. Linux uses separate signed AppImage and DEB targets. Tauri 2.14 installs DEB updates through dpkg with system privilege authorization. Physical Linux update installation remains to be verified. The feed uses HTTPS assets from this public repository and contains no GitHub token.

Checking alone never installs an update. Music, catalogs and backups are outside the installation directory. Finish active transfers before updating. Network and signature failures must leave the existing installation usable.

## Validation and limits

`tools/smoke.py` tests source preservation, preparation and occupied-port fallback. `tools/desktop-smoke.py` creates an actual desktop window (hidden during unattended checks), checks React and native IPC, verifies the default write gate, then closes the window and waits for process exit. Windows and Debian CI additionally install and test the native package. Windows tests verify both shortcut modes, their preservation through installer update mode, and uninstall cleanup. Raw executable smoke tests may use source-tree assets; installed-package tests must find packaged assets.

Installed-package checks pass `--installed`, which forbids fallback to source-tree UI assets. After publishing, `tools/desktop-smoke.py --installed --executable <installed-executable> --verify-release` downloads the real HTTPS updater artifact, verifies its signature and rejects a modified copy. This check does not install over the test application.

macOS builds validate packaging; physical macOS window and update behavior still require platform testing. A successful first-release check verifies feed access, not an actual upgrade between two installed versions.

## Upstream dependency tracking

The Linux GTK 3 dependency tree includes glib 0.18, covered by [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) concerning VariantStrIter. The application does not directly call that API; transitive exposure has not been audited. GTK 3 bindings constrain this dependency below the patched glib 0.20 line. The advisory remains open for upstream tracking rather than being dismissed as unaffected.

## v0.2.0 verification record

The [four-target native build](https://github.com/vicliu624/island-walkman/actions/runs/39085190643) passed for application commit `6437db3`. The [release workflow](https://github.com/vicliu624/island-walkman/actions/runs/39085944396) published [v0.2.0](https://github.com/vicliu624/island-walkman/releases/tag/v0.2.0) after verifying that subsequent changes were confined to the publisher and its documentation/tests. The release includes build-provenance.json and SHA256SUMS.

On Windows, an isolated installation of the publicly downloaded setup passed strict installed-resource checks, native IPC and close-to-exit. A live invocation of the updater check through the desktop bridge reported the current release as up to date. The updater downloaded the actual public Windows artifact, accepted its signature, and rejected a byte-modified copy. The original test HTML was restored after the IPC probe. Installer update mode retained both enabled and disabled shortcut choices; temporary installations were removed afterwards.

Debian CI installed and launched the DEB with strict resource checks under Xvfb. Both macOS architectures passed native compilation and packaging. Physical macOS updates, a version-to-version desktop upgrade, and full A918 transfers remain outside this verification record.

## v0.2.1 verification record

The [four-target native build](https://github.com/vicliu624/island-walkman/actions/runs/39087652709) passed for application commit `610903a`. The [successful publisher](https://github.com/vicliu624/island-walkman/actions/runs/39088701099) released v0.2.1 after excluding older versioned bundles restored from the Rust build cache. Duplicate-target checks remain mandatory. A manual publisher dispatch can recover an existing tag after publisher-only fixes; application inputs must still match the successful native build and tag. The original failed publication exposed no release or partial update feed.

On Windows, a disposable installation of the public v0.2.0 installer checked the public update endpoint through the native updater bridge, downloaded and installed signed v0.2.1, and restarted into the new application's desktop self-test. The installed executable reported version 0.2.1, the native desktop check passed, and a marker in the separate data directory survived. The temporary installation was uninstalled after testing. This verifies the Windows version-to-version update path; physical macOS/Linux upgrades and A918 transfer reliability remain unverified.

## v0.2.2 transfer entry validation

The running v0.2.1 desktop service reported eight queued tracks, F:/ connected, no active job and device_write_enabled=false. Its UI disabled Start transfer solely because the experimental startup flag was absent. The v0.2.2 UI opens a per-transfer confirmation dialog without that flag. Browser interaction against the actual rebuilt UI and an isolated catalog verified that Start transfer is enabled, acknowledgment enables Back up and transfer, and canceling/reopening clears acknowledgment. No transfer was submitted to the physical player.

HTTP regression tests exercise the real transfer endpoint: missing, false and string-valued confirmation are rejected before launch; boolean true reaches ordinary preflight and does not unlock later requests. Virtual-player tests continue to cover staging, append, backup, artwork and error rollback. Windows passed 44 Rust tests, the package smoke and translation/build checks for 177 messages before publication. This fixes access to experimental transfers; it does not establish physical A918 transfer reliability.
