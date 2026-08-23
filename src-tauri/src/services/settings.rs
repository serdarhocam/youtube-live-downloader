use crate::models::Settings;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

pub fn defaults() -> Settings {
    let base = dirs::video_dir().or_else(dirs::download_dir).unwrap_or_else(|| PathBuf::from("."));
    Settings { download_directory: base.join("YouTube Live Downloader").to_string_lossy().into(), default_quality: "best".into(), max_concurrent_downloads: 2, save_thumbnail: false, language: "en".into() }
}

pub fn load(app: &AppHandle) -> Settings {
    settings_path(app).ok().and_then(|p| fs::read_to_string(p).ok()).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_else(defaults)
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if settings.max_concurrent_downloads == 0 || settings.max_concurrent_downloads > 8 { return Err("Concurrent downloads must be between 1 and 8".into()); }
    if !matches!(settings.language.as_str(), "en" | "tr") { return Err("Unsupported application language".into()); }
    let path = settings_path(app)?;
    fs::write(path, serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
