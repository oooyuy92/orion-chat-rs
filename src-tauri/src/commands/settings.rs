use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct AppPaths {
    pub data_dir: String,
    pub log_dir: String,
    pub db_path: String,
    pub cache_dir: String,
}

/// Return common app directory paths.
#[tauri::command]
pub async fn get_app_paths(app: tauri::AppHandle) -> AppResult<AppPaths> {
    use tauri::Manager;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;

    let log_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;

    Ok(AppPaths {
        data_dir: data_dir.to_string_lossy().to_string(),
        log_dir: log_dir.to_string_lossy().to_string(),
        db_path: data_dir.join("orion.db").to_string_lossy().to_string(),
        cache_dir: cache_dir.to_string_lossy().to_string(),
    })
}

/// Open a directory or file in the system file manager / default app.
#[tauri::command]
pub async fn open_path(app: tauri::AppHandle, path: String) -> AppResult<()> {
    app.shell()
        .open(&path, None)
        .map_err(|e| AppError::Provider(e.to_string()))
}

/// Show a folder-picker dialog and return the selected path (or None if cancelled).
#[tauri::command]
pub async fn pick_directory(app: tauri::AppHandle) -> AppResult<Option<String>> {
    let path = app
        .dialog()
        .file()
        .blocking_pick_folder();

    Ok(path.map(|p| p.to_string()))
}

/// Return cache directory size as a human-readable string.
#[tauri::command]
pub async fn get_cache_size(app: tauri::AppHandle) -> AppResult<String> {
    use tauri::Manager;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;

    if !cache_dir.exists() {
        return Ok("0 KB".to_string());
    }

    let total = dir_size(&cache_dir);
    Ok(format_size(total))
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Delete all files in the app cache directory.
#[tauri::command]
pub async fn clear_cache(app: tauri::AppHandle) -> AppResult<()> {
    use tauri::Manager;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;

    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)
            .map_err(|e| AppError::Provider(e.to_string()))?;
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| AppError::Provider(e.to_string()))?;
    }
    Ok(())
}

/// Delete all conversations, messages, and assistants from the database.
/// Providers and their models are kept.
#[tauri::command]
pub async fn reset_app_data(state: State<'_, Arc<AppState>>) -> AppResult<()> {
    state.db.with_conn(|conn| {
        conn.execute_batch(
            "DELETE FROM messages;
             DELETE FROM conversations;
             DELETE FROM assistants;",
        )?;
        Ok(())
    })?;
    Ok(())
}

/// Copy the SQLite database to the given destination path as a backup.
#[tauri::command]
pub async fn local_backup(app: tauri::AppHandle, dest_path: String) -> AppResult<()> {
    use tauri::Manager;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Provider(e.to_string()))?;
    let db_path = data_dir.join("orion.db");

    std::fs::copy(&db_path, &dest_path)
        .map_err(|e| AppError::Provider(e.to_string()))?;
    Ok(())
}

/// Return whether the app is set to launch at startup.
#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> AppResult<bool> {
    let enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Provider(e.to_string()))?;
    Ok(enabled)
}

/// Enable or disable launching at startup.
#[tauri::command]
pub async fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> AppResult<()> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch
            .enable()
            .map_err(|e| AppError::Provider(e.to_string()))?;
    } else {
        autolaunch
            .disable()
            .map_err(|e| AppError::Provider(e.to_string()))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySetting {
    pub mode: String, // "system" | "none"
}

/// Persist proxy setting (applied to reqwest via AppState).
#[tauri::command]
pub async fn set_proxy_mode(
    state: State<'_, Arc<AppState>>,
    mode: String,
) -> AppResult<()> {
    *state.proxy_mode.lock().await = mode;
    Ok(())
}

/// Get current proxy mode from state.
#[tauri::command]
pub async fn get_proxy_mode(state: State<'_, Arc<AppState>>) -> AppResult<String> {
    Ok(state.proxy_mode.lock().await.clone())
}
