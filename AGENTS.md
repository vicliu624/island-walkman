# Release discipline

Application versions follow Semantic Versioning. New backward-compatible features increment MINOR; compatible fixes increment PATCH; incompatible public-contract changes need an explicit compatibility/migration decision. Development/hardware-unverified candidates must use a prerelease version, not the previous stable version plus a renamed filename.

Use tools/version.py --set VERSION to synchronize Cargo.toml, Cargo.lock root package, tauri.conf.json, ui/package.json and ui/package-lock.json. Add the matching CHANGELOG entry and run --check. Do not upgrade dependencies as part of a version-only operation. Never hardcode UI version strings. Native/backend/frontend versions must match in desktop smoke.

Never overwrite published versions, assets or tags. Do not publish releases, push tags or alter the stable updater feed without user authorization. Preserve superseded artifacts for audit and mark them clearly. Application SemVer is separate from R14 firmware revisions and disk-app package versions.
