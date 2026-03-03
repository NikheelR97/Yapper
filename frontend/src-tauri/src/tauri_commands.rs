use tauri::{AppHandle, Manager, Runtime, State};
use std::sync::Mutex;

/// Shared app state — persists for the lifetime of the process.
#[derive(Default)]
pub struct AppSettings {
    /// When true, closing the main window hides it to the tray instead of quitting.
    pub minimize_to_tray: bool,
}

pub type AppSettingsState = Mutex<AppSettings>;

// ─── Window commands ──────────────────────────────────────────────────────────

/// Show and focus the main window (called from tray click or notification click).
#[tauri::command]
pub fn show_window<R: Runtime>(app: AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.unminimize();
    }
}

/// Hide the main window to tray without quitting the process.
#[tauri::command]
pub fn hide_window<R: Runtime>(app: AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

// ─── Settings commands ────────────────────────────────────────────────────────

/// Toggle the "minimize to tray on close" preference.
/// Called from the Desktop section of the Settings page.
#[tauri::command]
pub fn set_minimize_to_tray(
    enabled: bool,
    settings: State<'_, AppSettingsState>,
) {
    if let Ok(mut s) = settings.lock() {
        s.minimize_to_tray = enabled;
    }
}

/// Read back the current minimize-to-tray setting (used to initialise the UI toggle).
#[tauri::command]
pub fn get_minimize_to_tray(settings: State<'_, AppSettingsState>) -> bool {
    settings.lock().map(|s| s.minimize_to_tray).unwrap_or(false)
}
