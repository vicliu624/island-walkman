use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    language: Option<String>,
}

fn preferences_path() -> std::path::PathBuf {
    let args: Vec<_> = std::env::args().collect();
    args.iter()
        .position(|arg| arg == "--data")
        .and_then(|index| args.get(index + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::service::default_data_directory)
        .join("desktop-preferences.json")
}

#[tauri::command]
fn get_language() -> Option<String> {
    std::fs::read(preferences_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Preferences>(&bytes).ok())
        .and_then(|p| p.language)
}

#[tauri::command]
fn set_language(language: String) -> Result<(), String> {
    if !["en", "zh-CN", "ja"].contains(&language.as_str()) {
        return Err("Unsupported language".into());
    }
    let path = preferences_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(
        path,
        serde_json::to_vec(&Preferences {
            language: Some(language),
        })
        .unwrap(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn verify_release_update(app: tauri::AppHandle) -> Result<usize, String> {
    if !std::env::args().any(|arg| arg == "--verify-release") {
        return Ok(0);
    }
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater_builder()
        .version_comparator(|_, _| true)
        .build()
        .map_err(|error| error.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("No release artifact available")?;
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    // Exercise both acceptance and rejection without installing over the test app.
    update.verify(&bytes).map_err(|error| error.to_string())?;
    let mut tampered = bytes.clone();
    if let Some(byte) = tampered.first_mut() {
        *byte ^= 1;
    }
    if update.verify(&tampered).is_ok() {
        return Err("Tampered update was accepted".into());
    }
    Ok(bytes.len())
}

#[tauri::command]
async fn desktop_smoke_result(app: tauri::AppHandle, error: Option<String>) {
    if !std::env::args().any(|arg| arg == "--desktop-smoke") {
        return;
    }
    let args: Vec<_> = std::env::args().collect();
    if let Some(path) = args
        .iter()
        .position(|arg| arg == "--smoke-report")
        .and_then(|index| args.get(index + 1))
    {
        let _ = std::fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({"passed": error.is_none(), "error": error}))
                .unwrap(),
        );
    }
    if let Some(error) = error {
        eprintln!("Desktop smoke failed: {error}");
        app.exit(1);
    } else if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
}

pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_language,
            set_language,
            verify_release_update,
            desktop_smoke_result
        ])
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let ui = if cfg!(debug_assertions)
                || (std::env::args().any(|arg| arg == "--desktop-smoke")
                    && !app.path().resource_dir()?.join("ui/index.html").exists())
            {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/dist")
            } else {
                app.path().resource_dir()?.join("ui")
            };
            if !ui.join("index.html").exists() {
                return Err(format!("Application UI is missing: {}", ui.display()).into());
            }
            let (ready, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || crate::service::run_desktop(ui, ready));
            let url = receiver
                .recv_timeout(std::time::Duration::from_secs(30))?
                .map_err(anyhow::Error::msg)?;
            app.add_capability(
                tauri::ipc::CapabilityBuilder::new("active-desktop")
                    .window("main")
                    .local(false)
                    .remote(format!("{url}/*"))
                    .permission("core:default")
                    .permission("dialog:allow-open")
                    .permission("updater:default")
                    .permission("process:allow-restart"),
            )?;
            let allowed_origin = url.clone();
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse()?),
            )
            .on_page_load(|webview, payload| {
                if std::env::args().any(|arg| arg == "--desktop-smoke")
                    && payload.event() == tauri::webview::PageLoadEvent::Finished
                {
                    let _ = webview.eval(r#"setTimeout(async () => {
                        let error = null;
                        try {
                            const invoke = window.__TAURI_INTERNALS__.invoke;
                            if (!await invoke('plugin:app|version')) throw Error('Native version unavailable');
                            await invoke('get_language');
                            await invoke('verify_release_update');
                            const state = await (await fetch('/api/state')).json();
                            if (state.device_write_enabled) throw Error('Device writes unexpectedly enabled');
                            if (!document.querySelector('h1')) throw Error('React UI missing');
                            if (!document.querySelector('button')) throw Error('UI controls missing');
                        } catch (e) { error = String(e); }
                        window.__TAURI_INTERNALS__.invoke('desktop_smoke_result', { error });
                    }, 1500);"#);
                }
            })
            .title("Island Walkman")
            .inner_size(1280.0, 840.0)
            .min_inner_size(980.0, 640.0)
            .on_navigation(move |target| target.as_str().starts_with(&format!("{allowed_origin}/")))
            .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())?
        .run(|app, event| {
            // This application has one window. Exit explicitly on every platform,
            // including macOS, without querying the window map during destruction.
            if let tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::CloseRequested { .. },
                ..
            } = event
            {
                app.exit(0);
            }
        });
    Ok(())
}
