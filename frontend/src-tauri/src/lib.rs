mod tauri_commands;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_commands::{AppSettingsState, TrayBadgeUpdater};
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // ── Plugins ────────────────────────────────────────────────────────
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        // ── Shared state ───────────────────────────────────────────────────
        .manage(AppSettingsState::default())
        // ── Setup ──────────────────────────────────────────────────────────
        .setup(|app| {
            let handle = app.handle().clone();

            // ── Global shortcuts ──────────────────────────────────────────
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let shortcut_handle = handle.clone();
                app.global_shortcut()
                    .on_shortcut("Ctrl+K", move |_app, _shortcut, _event| {
                        let _ = shortcut_handle.emit("global-search", ());
                    })?;
            }

            // ── System tray ───────────────────────────────────────────────
            #[cfg(desktop)]
            {
                let open_item = MenuItem::with_id(app, "open", "Open Yapper", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let tray_menu = Menu::with_items(app, &[&open_item, &quit_item])?;

                let tray_handle = handle.clone();
                let tray_icon = TrayIconBuilder::with_id("main-tray")
                    .icon(app.default_window_icon().cloned().unwrap())
                    .tooltip("Yapper")
                    .menu(&tray_menu)
                    .on_menu_event(move |_tray, event| match event.id.as_ref() {
                        "open" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = w.unminimize();
                            }
                        }
                        "quit" => {
                            tray_handle.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(move |tray, event| {
                        // Left-click on tray icon → show/focus window
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = w.unminimize();
                            }
                        }
                    })
                    .build(app)?;

                // Badge updater — captures a clone of the tray icon handle.
                // set_tray_badge() Tauri command calls this closure.
                let badge_icon = tray_icon.clone();
                app.manage(TrayBadgeUpdater(Arc::new(move |count: u32| {
                    let tooltip = if count > 0 {
                        format!("Yapper ({count})")
                    } else {
                        "Yapper".to_string()
                    };
                    let _ = badge_icon.set_tooltip(Some(&tooltip));
                })));
            }

            // ── Deep links ────────────────────────────────────────────────
            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let dl_handle = handle.clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        handle_deep_link(&dl_handle, url.as_str());
                    }
                });
            }

            // ── Auto-updater (silent background check) ────────────────────
            #[cfg(desktop)]
            {
                let updater_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    check_for_update(updater_handle).await;
                });
            }

            Ok(())
        })
        // ── Window events ──────────────────────────────────────────────────
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let minimize = app
                    .state::<AppSettingsState>()
                    .lock()
                    .map(|s| s.minimize_to_tray)
                    .unwrap_or(false);

                if minimize {
                    api.prevent_close();
                    let _ = window.hide();
                }
                // Otherwise: let close proceed normally — process exits when all windows close.
            }
        })
        // ── Commands ────────────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            tauri_commands::show_window,
            tauri_commands::hide_window,
            tauri_commands::set_minimize_to_tray,
            tauri_commands::get_minimize_to_tray,
            tauri_commands::set_tray_badge,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

// ─── Deep link router ─────────────────────────────────────────────────────────

/// Parse a `yapper://` URL and emit the appropriate Tauri event to the frontend.
///
/// Supported schemes:
///   yapper://dm/{conversationId}
///   yapper://server/{serverId}
///   yapper://invite/{code}
fn handle_deep_link(app: &tauri::AppHandle, url: &str) {
    // Strip the scheme: "yapper://"
    let path = url
        .strip_prefix("yapper://")
        .unwrap_or(url)
        .trim_start_matches('/');

    let mut parts = path.splitn(2, '/');
    let segment = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");

    match segment {
        "dm" if !rest.is_empty() => {
            let _ = app.emit("deep-link-dm", rest);
        }
        "server" if !rest.is_empty() => {
            let _ = app.emit("deep-link-server", rest);
        }
        "invite" if !rest.is_empty() => {
            let _ = app.emit("deep-link-invite", rest);
        }
        _ => {
            eprintln!("Unknown deep link: {url}");
        }
    }

    // Ensure main window is visible when a deep link is triggered externally
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// ─── Auto-updater ─────────────────────────────────────────────────────────────

/// Silently check for a new version. If one is found, emit `"update-available"`
/// to the frontend with the new version string — the JS UpdateBanner picks this up.
#[cfg(desktop)]
async fn check_for_update(app: tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater();
    let Ok(updater) = updater else { return };

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            println!("Update available: {version}");
            let _ = app.emit("update-available", &version);
            // Store the update so the restart command can install it
            // (download-and-install is triggered from JS via a Tauri command)
        }
        Ok(None) => {
            println!("App is up to date");
        }
        Err(e) => {
            // Non-fatal — updater endpoint may not be reachable in dev
            eprintln!("Update check failed: {e}");
        }
    }
}

// Stub for non-desktop targets (Android/iOS use their own update channels)
#[cfg(not(desktop))]
async fn check_for_update(_app: tauri::AppHandle) {}
