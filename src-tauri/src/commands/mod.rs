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

#[tauri::command]
pub async fn check_download_files(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<crate::services::media::MediaFiles>, String> {
    Ok(state.downloads.list(&app).await?.iter().map(crate::services::media::inspect).collect())
}

async fn media_job(app: &AppHandle, state: &AppState, id: &str) -> Result<DownloadJob, String> {
    state.downloads.list(app).await?.into_iter().find(|j| j.job_id == id).ok_or_else(|| "Download record not found.".into())
}

#[tauri::command]
pub async fn open_job_folder(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    let files = crate::services::media::inspect(&media_job(&app, &state, &job_id).await?);
    if !files.folder_exists { return Err("Klasör bulunamadı / Folder not found.".into()) }
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe").arg(&files.folder).creation_flags(0x08000000).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))] { Err("Windows only".into()) }
}

#[tauri::command]
pub async fn get_playback_media(app: AppHandle, state: State<'_, AppState>, job_id: String) -> Result<crate::services::media::MediaFiles, String> {
    use tauri::Manager;
    let job = media_job(&app, &state, &job_id).await?;
    let files = crate::services::media::inspect(&job);
    if job.status != "completed" || !files.video_exists { return Err("Video bulunamadı / Video not available.".into()) }
    app.asset_protocol_scope().allow_file(files.video_path.as_ref().unwrap()).map_err(|e| e.to_string())?;
    Ok(files)
}

#[tauri::command]
pub async fn open_player(app: AppHandle, state: State<'_, AppState>, job_id: String, seconds: Option<f64>) -> Result<(), String> {
    use tauri::Manager;
    let job = media_job(&app, &state, &job_id).await?;
    get_playback_media(app.clone(), state, job_id.clone()).await?;
    let label = format!("player-{job_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.unminimize().map_err(|e| e.to_string())?;
        if let Some(seconds) = seconds.filter(|n|n.is_finite() && *n >= 0.0) { use tauri::Emitter; let _=app.emit_to(&label,"player-seek",seconds); }
        return window.set_focus().map_err(|e| e.to_string());
    }
    let query: String = url::form_urlencoded::Serializer::new(String::new()).append_pair("player", &job_id).append_pair("lang", &job.settings.language).append_pair("seconds", &seconds.unwrap_or(0.0).max(0.0).to_string()).finish();
    tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::App(format!("index.html?{query}").into()))
        .title(&job.request.item.title).inner_size(1000.0, 700.0).min_inner_size(640.0, 440.0)
        .build().map_err(|e| e.to_string())?;
    Ok(())
}


#[tauri::command]
pub async fn open_chat(app:AppHandle,state:State<'_,AppState>,job_id:String)->Result<(),String>{
    use tauri::Manager;
    let job=media_job(&app,&state,&job_id).await?;
    let label=format!("chat-{job_id}");
    if let Some(w)=app.get_webview_window(&label){w.unminimize().map_err(|e|e.to_string())?;return w.set_focus().map_err(|e|e.to_string())}
    let query:String=url::form_urlencoded::Serializer::new(String::new()).append_pair("chat",&job_id).append_pair("lang",&crate::services::settings::load(&app).language).finish();
    tauri::WebviewWindowBuilder::new(&app,label,tauri::WebviewUrl::App(format!("index.html?{query}").into())).title(format!("Chat — {}",job.request.item.title)).inner_size(1300.0,880.0).min_inner_size(950.0,650.0).build().map_err(|e|e.to_string())?;Ok(())
}
#[tauri::command]
pub async fn get_chat(app:AppHandle,state:State<'_,AppState>,job_id:String)->Result<crate::services::chat::ChatReport,String>{
    let job=media_job(&app,&state,&job_id).await?;
    tauri::async_runtime::spawn_blocking(move||crate::services::chat::load(&job)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn download_chat(app:AppHandle,state:State<'_,AppState>,job_id:String)->Result<crate::services::chat::ChatReport,String>{
    let job=media_job(&app,&state,&job_id).await?;
    if job.status!="completed" {return Err("Önce video indirmesini tamamlayın.".into())}
    crate::services::chat::download(&app,&job).await
}
#[tauri::command]
pub fn cancel_chat(job_id:String){crate::services::chat::cancel(&job_id)}
#[tauri::command]
pub async fn get_chat_status(app:AppHandle,state:State<'_,AppState>,job_id:String)->Result<Option<crate::services::chat::ChatStatus>,String>{
    Ok(crate::services::chat::saved_status(&media_job(&app,&state,&job_id).await?))
}
#[tauri::command]
pub async fn import_resolve_xml(app:AppHandle,state:State<'_,AppState>,job_id:String,path:String)->Result<crate::services::chat_edit::EditMap,String>{
    let job=media_job(&app,&state,&job_id).await?;
    let media=crate::services::media::inspect(&job);
    let source=media.video_path.ok_or("Video dosyası bulunamadı")?;
    if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>20_000_000{return Err("XML dosyası çok büyük (20 MB sınırı).".into())}
    let xml=std::fs::read_to_string(&path).map_err(|e|e.to_string())?;
    let edit=crate::services::chat_edit::parse_xml(&xml,&source,job.request.item.duration.ok_or("Video süresi bilinmiyor")?)?;
    let dir=crate::services::chat::folder(&job);
    std::fs::write(dir.join(format!("{}-{}.resolve.xml",crate::services::chat::base(&job),uuid::Uuid::new_v4())),xml).map_err(|e|e.to_string())?;
    std::fs::write(crate::services::chat::edit_path(&job),serde_json::to_vec_pretty(&edit).unwrap()).map_err(|e|e.to_string())?;
    Ok(edit)
}
#[tauri::command]
pub async fn get_chat_timing(app:AppHandle,state:State<'_,AppState>,job_id:String,options:crate::services::chat_export::WidgetOptions,use_edit:bool)->Result<Vec<crate::services::chat_export::TimedMessage>,String>{
    let job=media_job(&app,&state,&job_id).await?;
    tauri::async_runtime::spawn_blocking(move||crate::services::chat_export::prepare(&job,&options,use_edit).map(|(m,_)|m)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn export_chat_text(app:AppHandle,state:State<'_,AppState>,job_id:String,options:crate::services::chat_export::WidgetOptions,use_edit:bool,kind:String)->Result<String,String>{
    let job=media_job(&app,&state,&job_id).await?;
    tauri::async_runtime::spawn_blocking(move||crate::services::chat_export::export_text(&job,&options,use_edit,&kind)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn render_chat_overlay(app:AppHandle,state:State<'_,AppState>,job_id:String,options:crate::services::chat_export::WidgetOptions,use_edit:bool,preview:bool)->Result<String,String>{
    let job=media_job(&app,&state,&job_id).await?;
    crate::services::chat_export::render(&app,&job,options,use_edit,preview).await
}
#[tauri::command]
pub async fn chat_raw_page(app:AppHandle,state:State<'_,AppState>,job_id:String,page:usize)->Result<String,String>{
    use std::io::BufRead;
    let job=media_job(&app,&state,&job_id).await?;
    if page>100000{return Err("Invalid page".into())}
    let file=std::fs::File::open(crate::services::chat::raw_path(&job)).map_err(|e|e.to_string())?;
    let lines=std::io::BufReader::new(file).lines().skip(page*20).take(20).collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(lines.join("\n"))
}

#[tauri::command]
pub async fn open_chat_export_folder(app:AppHandle,state:State<'_,AppState>,job_id:String,path:String)->Result<(),String>{
    let job=media_job(&app,&state,&job_id).await?;
    let root=crate::services::chat::folder(&job).canonicalize().map_err(|e|e.to_string())?;
    let file=std::path::PathBuf::from(path).canonicalize().map_err(|_|"Çıktı dosyası bulunamadı.".to_string())?;
    if !file.is_file()||!file.starts_with(&root){return Err("Geçersiz çıktı dosyası.".into())}
    let folder=file.parent().ok_or("Çıktı klasörü bulunamadı.")?;
    #[cfg(windows)]{use std::os::windows::process::CommandExt;Command::new("explorer.exe").arg(folder).creation_flags(0x08000000).spawn().map_err(|e|e.to_string())?;Ok(())}
    #[cfg(not(windows))]{Err("Windows only".into())}
}
