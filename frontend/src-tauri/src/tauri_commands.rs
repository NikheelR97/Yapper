use std::{path::PathBuf, sync::{Arc, Mutex}};
use tauri::{AppHandle, Manager, Runtime, State};

/// Shared app state — persists for the lifetime of the process.
#[derive(Default)]
pub struct AppSettings {
    /// When true, closing the main window hides it to the tray instead of quitting.
    pub minimize_to_tray: bool,
}

pub type AppSettingsState = Mutex<AppSettings>;

const STRONGHOLD_FILENAME: &str = "signal-vault.hold";

fn stronghold_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(STRONGHOLD_FILENAME))
        .map_err(|err| err.to_string())
}

// ─── Tray badge state ─────────────────────────────────────────────────────────

/// Callback installed during setup that updates the system tray tooltip with
/// the current unread count. Stored as a boxed fn so commands don't need to
/// carry the Wry runtime generic.
pub struct TrayBadgeUpdater(pub Arc<dyn Fn(u32) + Send + Sync>);

/// Update the system tray tooltip to reflect unread message count.
/// `count = 0` resets the tooltip to "Yapper".
#[tauri::command]
pub fn set_tray_badge(count: u32, updater: State<'_, TrayBadgeUpdater>) {
    (updater.0)(count);
}

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

#[tauri::command]
pub fn stronghold_exists<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    Ok(stronghold_path(&app)?.exists())
}
