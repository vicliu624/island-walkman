fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_language",
            "set_language",
            "desktop_smoke_result",
            "verify_release_update",
        ]),
    ))
    .expect("Could not generate desktop command permissions");
}
