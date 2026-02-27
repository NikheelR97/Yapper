use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // Register deep link protocol: yapper://
            #[cfg(desktop)]
            {
                let _ = app.handle().plugin(
                    tauri_plugin_deep_link::init()
                );
            }

            // Register global shortcuts
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
                let app_handle = app.handle().clone();
                app.global_shortcut().on_shortcut("Ctrl+K", move |_app, _shortcut, _event| {
                    // Focus search — emitted as a Svelte store event
                    let _ = app_handle.emit("global-search", ());
                })?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
