mod commands;
mod models;
mod services;

pub struct AppState { downloads: services::downloader::DownloadManager }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).manage(AppState { downloads: Default::default() })
        .setup(|app| {
            use tauri::Manager;
            let log_dir=app.path().app_log_dir()?; std::fs::create_dir_all(&log_dir)?;
            let file=tracing_appender::rolling::daily(log_dir,"youtube-live-downloader.log");
            tracing_subscriber::fmt().with_writer(file).with_ansi(false).init();
            let binaries = services::binaries::BinaryPaths::resolve(app.handle()).map_err(std::io::Error::other)?;
            tracing::info!(ytdlp=%binaries.ytdlp.display(), ffmpeg=%binaries.ffmpeg.display(), "application startup; embedded tools ready");
            let state = app.state::<AppState>();
            tauri::async_runtime::block_on(state.downloads.initialize(app.handle())).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::load_metadata,commands::get_settings,commands::save_settings,commands::open_download_folder,commands::open_developer_website,commands::queue_downloads,commands::cancel_download,commands::remove_download_job,commands::pause_download,commands::resume_download,commands::resume_all_downloads,commands::list_download_jobs,commands::get_app_info])
        .run(tauri::generate_context!()).expect("error while running application");
}
