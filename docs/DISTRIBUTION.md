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
4. Push a `v<version>` tag. CI builds Windows x86-64, Debian x86-64, macOS Apple Silicon and macOS Intel.
5. After every package job passes, CI gathers signatures, validates all four updater targets, generates latest.json and SHA256SUMS, and publishes the Release.

A partial build cannot publish a partial feed. Publish a new version for changes instead of replacing existing assets. Pull requests without the signing secret build unsigned installers and never publish releases. For local unsigned packages, pass `--config '{"bundle":{"createUpdaterArtifacts":false}}'` to the Tauri build command.

## Signing keys

The public key is committed in tauri.conf.json. The private key is stored at the ignored local path `data/updater-signing.key` and in the repository Actions secret. Preserve an independent secure backup. Losing it prevents publishing updates accepted by existing installations. Never commit it, print it in logs or include it in installers. Do not rotate the public key without a migration plan.

Updater signatures are separate from Windows Authenticode and Apple Developer ID. Windows packages are not Authenticode-signed. macOS applications are ad-hoc signed and are not notarized.

## Update behavior

Settings offers an explicit check, release notes, download progress and installation. Windows verifies the setup EXE and invokes passive update mode. macOS verifies an APP archive and replaces the application. Linux uses separate signed AppImage and DEB targets. Tauri 2.14 installs DEB updates through dpkg with system privilege authorization. Physical Linux update installation remains to be verified. The feed uses HTTPS assets from this public repository and contains no GitHub token.

Checking alone never installs an update. Music, catalogs and backups are outside the installation directory. Finish active transfers before updating. Network and signature failures must leave the existing installation usable.

## Validation and limits

`tools/smoke.py` tests source preservation, preparation and occupied-port fallback. `tools/desktop-smoke.py` opens a real desktop window, checks React and native IPC, verifies the default write gate, then closes the window and waits for process exit. Windows and Debian CI additionally install and test the native package. Raw executable smoke tests may use source-tree assets; installed-package tests must find packaged assets.

macOS builds validate packaging; physical macOS window and update behavior still require platform testing. A successful first-release check verifies feed access, not an actual upgrade between two installed versions.
