use crate::{models::{AppInfo, DownloadJob, DownloadRequest, Settings, VideoItem}, AppState};
use tauri::{AppHandle, State};
use std::process::{Command, Stdio};

#[tauri::command]
pub async fn load_metadata(app: AppHandle, url: String) -> Result<Vec<VideoItem>, String> { crate::services::ytdlp::metadata(&app, &url).await }
#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings { crate::services::settings::load(&app) }
#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> { crate::services::settings::save(&app, &settings) }
#[tauri::command]
pub fn open_download_folder(app: AppHandle) -> Result<(), String> {
    let settings = crate::services::settings::load(&app);
    let path = std::path::PathBuf::from(settings.download_directory);
    std::fs::create_dir_all(&path).map_err(|e| format!("Cannot open the download directory: {e}"))?;
    if !path.is_dir() { return Err("The download destination is not a folder.".into()) }
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe").arg(&path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(0x08000000).spawn().map_err(|e| format!("Could not open Windows Explorer: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))] { Err("Opening the download directory is currently supported only on Windows.".into()) }
}
#[tauri::command]
pub fn open_developer_website() -> Result<(), String> {
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe").arg("https://serdarhocam.com/").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(0x08000000).spawn().map_err(|e| format!("Could not open the developer website: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))] { Err("Opening the developer website is currently supported only on Windows.".into()) }
}
#[tauri::command]
pub async fn queue_downloads(app: AppHandle, state: State<'_, AppState>, requests: Vec<DownloadRequest>) -> Result<Vec<String>, String> { let s=crate::services::settings::load(&app); state.downloads.enqueue(app, requests, s).await }
#[tauri::command]
pub async fn cancel_download(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<(), String> { state.downloads.cancel(&app,&job_id).await }
#[tauri::command]
pub async fn remove_download_job(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<(), String> { state.downloads.remove(&app,&job_id).await }
#[tauri::command]
pub async fn pause_download(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<(), String> { state.downloads.pause(&app,&job_id).await }
#[tauri::command]
pub async fn resume_download(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<(), String> { state.downloads.resume(app,&job_id).await }
#[tauri::command]
pub async fn resume_all_downloads(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> { state.downloads.resume_all(app).await }
#[tauri::command]
pub async fn list_download_jobs(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> { state.downloads.list(&app).await }
#[tauri::command]
pub async fn get_app_info(app: AppHandle) -> AppInfo {
    let y = crate::services::ytdlp::version(&app).await.unwrap_or_else(|_| "Unavailable".into());
    let versions = include_str!("../../binaries/VERSIONS.txt");
    let f = versions.lines().find_map(|line| line.strip_prefix("FFmpeg: ")).unwrap_or("Bundled").to_string();
    AppInfo { version: env!("CARGO_PKG_VERSION").into(), ytdlp_version: y, ffmpeg_version: f }
}
