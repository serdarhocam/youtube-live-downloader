use crate::{
    models::{DownloadJob, DownloadRequest, Settings},
    services::binaries::BinaryPaths,
};
use std::{
    collections::{HashMap, VecDeque}, fs, path::PathBuf, process::Stdio,
    sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    io::{AsyncBufReadExt, BufReader}, process::{Child, Command},
    sync::Mutex,
};

const DEFAULT_MAX_RETRIES: u32 = 10;
const PROGRESS_SAVE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct DownloadManager {
    jobs: Arc<Mutex<HashMap<String, DownloadJob>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
    active: Arc<Mutex<HashMap<String, ActiveProcess>>>,
    pumping: Arc<Mutex<bool>>,
    initialized: Arc<AtomicBool>,
    last_progress_save: Arc<Mutex<Instant>>,
}

#[derive(Clone)]
struct ActiveProcess { child: Arc<Mutex<Child>>, pid: u32 }

impl Default for DownloadManager {
    fn default() -> Self {
        Self {
            jobs: Default::default(), queue: Default::default(), active: Default::default(),
            pumping: Default::default(), initialized: Arc::new(AtomicBool::new(false)),
            last_progress_save: Arc::new(Mutex::new(Instant::now() - PROGRESS_SAVE_INTERVAL)),
        }
    }
}

impl DownloadManager {
    pub async fn initialize(&self, app: &AppHandle) -> Result<(), String> {
        if self.initialized.swap(true, Ordering::SeqCst) { return Ok(()) }
        let mut restored = load_jobs(app)?;
        let now = chrono::Utc::now().timestamp();
        for job in restored.values_mut() {
            if job.status == "completed" { job.output_path = super::media::inspect(job).video_path.or(job.output_path.clone()); }
            if matches!(job.status.as_str(), "queued" | "preparing" | "downloading" | "pausing" | "retrying" | "merging") {
                job.status = "interrupted".into();
                job.stage = "Interrupted — Resume available".into();
                job.speed = None; job.eta = None; job.next_retry_at = None;
                job.message = Some("Previous activity ended unexpectedly. Displayed progress is historical until resumed.".into());
                job.updated_at = now;
            }
        }
        *self.jobs.lock().await = restored;
        self.persist_now(app).await
    }

    pub async fn list(&self, app: &AppHandle) -> Result<Vec<DownloadJob>, String> {
        self.initialize(app).await?;
        let mut jobs: Vec<_> = self.jobs.lock().await.values().cloned().collect();
        jobs.sort_by_key(|j| j.created_at);
        Ok(jobs)
    }

    pub async fn enqueue(&self, app: AppHandle, requests: Vec<DownloadRequest>, settings: Settings) -> Result<Vec<String>, String> {
        self.initialize(&app).await?;
        validate_destination(&settings.download_directory)?;
        let now = chrono::Utc::now().timestamp();
        let mut ids = Vec::new();
        for request in requests {
            if self.has_unfinished_for(&request.item.id).await {
                return Err(format!("An unfinished download already exists for '{}'. Resume or cancel that job first.", request.item.title));
            }
            let id = uuid::Uuid::new_v4().to_string();
            let output_template = super::media::output_template(&settings.download_directory, &request.item.title, &request.item.id);
            let job = DownloadJob {
                job_id: id.clone(), request, settings: settings.clone(), output_template,
                status: "queued".into(), stage: "Queued".into(), percent: None,
                downloaded_bytes: None, total_bytes: None, speed: None, eta: None,
                created_at: now, updated_at: now, retry_count: 0, max_retries: DEFAULT_MAX_RETRIES,
                next_retry_at: None, message: None, output_path: None, technical_details: None,
            };
            emit_job(&app, &job); self.jobs.lock().await.insert(id.clone(), job);
            self.queue.lock().await.push_back(id.clone()); ids.push(id);
        }
        self.persist_now(&app).await?;
        self.start_pump(app);
        Ok(ids)
    }

    async fn has_unfinished_for(&self, video_id: &str) -> bool {
        self.jobs.lock().await.values().any(|j| j.request.item.id == video_id && !matches!(j.status.as_str(), "completed" | "failed" | "cancelled"))
    }

    fn start_pump(&self, app: AppHandle) {
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut flag = manager.pumping.lock().await;
            if *flag { return } *flag = true; drop(flag);
            loop {
                let next = manager.queue.lock().await.front().cloned();
                let Some(job_id) = next else { break };
                let max = manager.jobs.lock().await.get(&job_id).map(|j| j.settings.max_concurrent_downloads).unwrap_or(2);
                let active_count = manager.active_state_count().await;
                if active_count >= max { tokio::time::sleep(Duration::from_millis(200)).await; continue }
                manager.queue.lock().await.pop_front();
                if !manager.transition(&app, &job_id, "preparing", "Preparing", None).await { continue }
                let worker = manager.clone(); let worker_app = app.clone();
                tauri::async_runtime::spawn(async move { worker.run_job(worker_app, job_id).await; });
            }
            *manager.pumping.lock().await = false;
        });
    }

    async fn active_state_count(&self) -> usize {
        self.jobs.lock().await.values().filter(|j| matches!(j.status.as_str(), "preparing" | "downloading" | "pausing" | "merging")).count()
    }

    async fn run_job(&self, app: AppHandle, job_id: String) {
        let Some(job) = self.jobs.lock().await.get(&job_id).cloned() else { return };
        let bins = match BinaryPaths::resolve(&app) {
            Ok(value) => value,
            Err(error) => { self.finish_interrupted(&app, &job_id, error).await; return }
        };
        let quality = format_selector(&job.request.quality);
        // JSON progress avoids locale/spacing differences in yt-dlp's human-readable output.
        let progress_template = "download:PROGRESS_JSON:%(progress)j";
        let mut command = Command::new(&bins.ytdlp);
        command.args([
            "--encoding", "utf-8", "--newline", "--no-color", "--continue", "--part", "--no-overwrites",
            "--retries", "10", "--fragment-retries", "20", "--extractor-retries", "5",
            "--file-access-retries", "5", "--retry-sleep", "http:exp=1:20",
            "--retry-sleep", "fragment:exp=1:20", "--retry-sleep", "extractor:exp=1:10",
            "--concurrent-fragments", "4", "--progress-delta", "0.25", "--windows-filenames", "--trim-filenames", "200",
            "--ffmpeg-location",
        ]).arg(bins.ffmpeg_dir()).args([
            "--format", &quality, "--merge-output-format", "mp4", "--progress-template", progress_template,
            "--print", "after_move:FILE:%(filepath)s", "--progress",
        ]).args(if job.settings.save_thumbnail { vec!["--write-thumbnail", "--convert-thumbnails", "jpg"] } else { vec![] })
            .arg("--output").arg(&job.output_template).arg(&job.request.item.webpage_url)
            .stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        #[cfg(windows)] { use std::os::windows::process::CommandExt; command.as_std_mut().creation_flags(0x08000000); }

        tracing::info!(job_id=%job_id, video_id=%job.request.item.id, retry=job.retry_count, "download process start");
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => { self.handle_failure(&app, &job_id, format!("Could not start yt-dlp: {error}")).await; return }
        };
        let child = Arc::new(Mutex::new(child));
        let pid = child.lock().await.id().unwrap_or_default();
        self.active.lock().await.insert(job_id.clone(), ActiveProcess { child: child.clone(), pid });
        self.transition(&app, &job_id, "downloading", if job.percent.is_some() { "Downloading — checking partial data" } else { "Downloading" }, None).await;

        let (stdout, stderr) = { let mut guard = child.lock().await; (guard.stdout.take(), guard.stderr.take()) };
        let stderr_text = Arc::new(Mutex::new(String::new()));
        let stderr_task = stderr.map(|stream| {
            let collected = stderr_text.clone(); let manager=self.clone(); let stderr_app=app.clone(); let stderr_job=job_id.clone();
            tauri::async_runtime::spawn(async move {
                let mut lines = BufReader::new(stream).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.starts_with("PROGRESS_JSON:") { manager.apply_progress(&stderr_app,&stderr_job,&line).await; }
                    else {
                        if line.contains("[Merger]") || line.contains("Merging formats") { manager.transition(&stderr_app,&stderr_job,"merging","Merging",None).await; }
                        let mut text=collected.lock().await;text.push_str(&line);text.push('\n');
                    }
                }
            })
        });

        let mut final_path = None;
        if let Some(stream) = stdout {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(path) = line.strip_prefix("FILE:") { final_path = Some(path.to_string()); }
                else if line.starts_with("PROGRESS_JSON:") { self.apply_progress(&app, &job_id, &line).await; }
            }
        }
        let status = child.lock().await.wait().await;
        if let Some(task) = stderr_task { let _ = task.await; }
        self.active.lock().await.remove(&job_id);
        let details = stderr_text.lock().await.clone();
        let desired = self.jobs.lock().await.get(&job_id).map(|j| j.status.clone()).unwrap_or_default();
        match status {
            _ if desired == "pausing" => { self.transition(&app, &job_id, "paused", "Paused", Some("Partial files were preserved.".into())).await; }
            _ if desired == "cancelled" => { self.transition(&app, &job_id, "cancelled", "Cancelled", Some("Partial files were preserved and can be resumed.".into())).await; }
            Ok(code) if code.success() => {
                self.update_job(&app, &job_id, |j| { j.status="completed".into(); j.stage="Completed".into(); j.percent=Some(100.0); j.speed=None; j.eta=None; j.output_path=final_path; j.output_path=super::media::inspect(j).video_path.or(j.output_path.clone()); j.message=None; }).await;
                let _ = self.persist_now(&app).await; tracing::info!(job_id=%job_id, "download complete");
                if job.request.item.was_live == Some(true) {
                    if let Some(saved) = self.jobs.lock().await.get(&job_id).cloned() {
                        let chat_app=app.clone();
                        tauri::async_runtime::spawn(async move { if let Err(e)=super::chat::download(&chat_app,&saved).await { tracing::warn!(error=%e,"chat download failed; video remains complete"); } });
                    }
                }

            }
            Ok(_) | Err(_) => self.handle_failure(&app, &job_id, details).await,
        }
        self.start_pump(app);
    }

    async fn apply_progress(&self, app: &AppHandle, job_id: &str, line: &str) {
        let Some(payload) = line.strip_prefix("PROGRESS_JSON:") else { return };
        let Ok(progress) = serde_json::from_str::<serde_json::Value>(payload) else {
            tracing::warn!(job_id=%job_id, "yt-dlp returned malformed progress JSON");
            return;
        };
        let finished = progress.get("status").and_then(|v| v.as_str()) == Some("finished");
        let downloaded = json_u64(&progress, "downloaded_bytes");
        let total = json_u64(&progress, "total_bytes").or_else(|| json_u64(&progress, "total_bytes_estimate"));
        let speed = json_f64(&progress, "speed");
        let eta = json_u64(&progress, "eta");
        let percent = downloaded.zip(total).filter(|(_, total)| *total > 0)
            .map(|(downloaded, total)| downloaded as f64 * 100.0 / total as f64)
            .or_else(|| progress.get("_percent_str").and_then(|v| v.as_str()).and_then(parse_percent));
        self.update_job(app, job_id, |job| {
            job.status = if finished { "merging" } else { "downloading" }.into();
            job.stage = if finished { "Merging" } else { "Downloading" }.into();
            job.downloaded_bytes = downloaded; job.total_bytes = total;
            job.speed = speed; job.eta = eta; job.percent = percent;
            job.message = None;
        }).await;
        self.persist_throttled(app).await;
    }

    async fn handle_failure(&self, app: &AppHandle, job_id: &str, details: String) {
        if is_permanent_error(&details) {
            self.update_job(app, job_id, |j| { j.status="failed".into(); j.stage="Failed".into(); j.speed=None; j.eta=None; j.technical_details=Some(details.clone()); j.message=Some(permanent_message(&details)); }).await;
            let _ = self.persist_now(app).await; return;
        }
        if is_disk_full(&details) {
            self.finish_interrupted(app, job_id, "Insufficient disk space. Free space and resume the download.".into()).await; return;
        }
        let (retry_count, max_retries) = {
            let jobs = self.jobs.lock().await; let Some(j)=jobs.get(job_id) else{return}; (j.retry_count, j.max_retries)
        };
        if retry_count >= max_retries {
            self.finish_interrupted(app, job_id, format!("Retry limit reached. Partial files were preserved.\n{details}")).await; return;
        }
        let attempt = retry_count + 1;
        let delay = 2_u64.saturating_pow(attempt.min(6)).min(60);
        self.update_job(app, job_id, |j| {
            j.status="retrying".into(); j.stage=format!("Retrying — attempt {attempt}/{max_retries}");
            j.retry_count=attempt; j.next_retry_at=Some(chrono::Utc::now().timestamp()+delay as i64);
            j.speed=None; j.eta=Some(delay); j.message=Some(format!("Next retry in {delay} seconds")); j.technical_details=Some(details.clone());
        }).await;
        let _ = self.persist_now(app).await;
        let manager=self.clone(); let retry_app=app.clone(); let retry_id=job_id.to_string();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(delay)).await;
            let still_retrying=manager.jobs.lock().await.get(&retry_id).is_some_and(|j|j.status=="retrying");
            if still_retrying {
                manager.transition(&retry_app,&retry_id,"queued","Queued after retry",None).await;
                manager.queue.lock().await.push_back(retry_id); let _=manager.persist_now(&retry_app).await; manager.start_pump(retry_app);
            }
        });
    }

    async fn finish_interrupted(&self, app:&AppHandle, job_id:&str, message:String) {
        self.update_job(app,job_id,|j|{j.status="interrupted".into();j.stage="Interrupted — Resume available".into();j.speed=None;j.eta=None;j.next_retry_at=None;j.message=Some(message.clone());}).await;
        let _=self.persist_now(app).await;
    }

    pub async fn pause(&self, app:&AppHandle, job_id:&str)->Result<(),String>{
        self.initialize(app).await?;
        if remove_from_queue(&self.queue,job_id).await { self.transition(app,job_id,"paused","Paused",Some("Partial files were preserved.".into())).await; self.persist_now(app).await?; return Ok(()) }
        if self.jobs.lock().await.get(job_id).is_some_and(|j|j.status=="retrying") { self.transition(app,job_id,"paused","Paused",Some("Automatic retry stopped; partial files were preserved.".into())).await; self.persist_now(app).await?; return Ok(()) }
        let process=self.active.lock().await.get(job_id).cloned().ok_or("Only an active, queued, or retrying download can be paused.")?;
        self.transition(app,job_id,"pausing","Pausing",None).await;
        terminate_process(process).await
    }

    pub async fn resume(&self, app:AppHandle, job_id:&str)->Result<(),String>{
        self.initialize(&app).await?;
        if self.active.lock().await.contains_key(job_id) || self.queue.lock().await.contains(&job_id.to_string()) { return Err("This download is already active or queued.".into()) }
        let allowed=self.jobs.lock().await.get(job_id).is_some_and(|j|matches!(j.status.as_str(),"paused"|"interrupted"|"cancelled"|"failed"));
        if !allowed{return Err("This download cannot be resumed from its current state.".into())}
        self.transition(&app,job_id,"queued","Queued for resume",Some("Existing .part and fragment state will be reused.".into())).await;
        self.queue.lock().await.push_back(job_id.to_string()); self.persist_now(&app).await?; self.start_pump(app); Ok(())
    }

    pub async fn resume_all(&self, app:AppHandle)->Result<usize,String>{
        self.initialize(&app).await?;
        let ids:Vec<String>=self.jobs.lock().await.values().filter(|j|matches!(j.status.as_str(),"paused"|"interrupted")).map(|j|j.job_id.clone()).collect();
        for id in &ids { self.transition(&app,id,"queued","Queued for resume",Some("Existing partial data will be reused.".into())).await; self.queue.lock().await.push_back(id.clone()); }
        self.persist_now(&app).await?; if !ids.is_empty(){self.start_pump(app)} Ok(ids.len())
    }

    pub async fn cancel(&self, app:&AppHandle, job_id:&str)->Result<(),String>{
        self.initialize(app).await?;
        if remove_from_queue(&self.queue,job_id).await {
            self.transition(app,job_id,"cancelled","Cancelled",Some("Partial files were preserved and can be resumed.".into())).await; self.persist_now(app).await?; return Ok(())
        }
        let retrying=self.jobs.lock().await.get(job_id).is_some_and(|j|j.status=="retrying");
        if retrying { self.transition(app,job_id,"cancelled","Cancelled",Some("Partial files were preserved and can be resumed.".into())).await; self.persist_now(app).await?; return Ok(()) }
        let process=self.active.lock().await.get(job_id).cloned().ok_or("Download job is not active or queued.")?;
        self.transition(app,job_id,"cancelled","Cancelled",Some("Stopping process; partial files will be preserved.".into())).await;
        terminate_process(process).await
    }

    pub async fn remove(&self, app:&AppHandle, job_id:&str)->Result<(),String>{
        self.initialize(app).await?;
        remove_from_queue(&self.queue,job_id).await;
        if let Some(process)=self.active.lock().await.get(job_id).cloned(){
            terminate_process(process).await?;
        }
        let removed=self.jobs.lock().await.remove(job_id);
        if removed.is_none(){return Err("Download job was not found.".into())}
        self.persist_now(app).await?;
        tracing::info!(job_id=%job_id,"download job removed from persistent list; media and partial files preserved");
        Ok(())
    }

    async fn transition(&self,app:&AppHandle,id:&str,status:&str,stage:&str,message:Option<String>)->bool{
        self.update_job(app,id,|j|{j.status=status.into();j.stage=stage.into();j.message=message;j.updated_at=chrono::Utc::now().timestamp();}).await
    }
    async fn update_job<F:FnOnce(&mut DownloadJob)>(&self,app:&AppHandle,id:&str,update:F)->bool{
        let mut jobs=self.jobs.lock().await; let Some(job)=jobs.get_mut(id) else{return false}; update(job);job.updated_at=chrono::Utc::now().timestamp();emit_job(app,job);true
    }
    async fn persist_throttled(&self,app:&AppHandle){let mut last=self.last_progress_save.lock().await;if last.elapsed()<PROGRESS_SAVE_INTERVAL{return}*last=Instant::now();drop(last);let _=self.persist_now(app).await;}
    async fn persist_now(&self,app:&AppHandle)->Result<(),String>{let snapshot:Vec<_>=self.jobs.lock().await.values().cloned().collect();save_jobs(app,&snapshot)}
}

async fn remove_from_queue(queue:&Mutex<VecDeque<String>>,id:&str)->bool{let mut q=queue.lock().await;if let Some(pos)=q.iter().position(|v|v==id){q.remove(pos);true}else{false}}
#[cfg(windows)]
async fn terminate_process(process:ActiveProcess)->Result<(),String>{
    if process.pid==0{return Err("The download process identifier is unavailable.".into())}
    let status=Command::new("taskkill.exe").args(["/PID",&process.pid.to_string(),"/T","/F"]).creation_flags(0x08000000).status().await.map_err(|e|format!("Could not stop download process: {e}"))?;
    if status.success(){Ok(())}else{process.child.lock().await.start_kill().map_err(|e|e.to_string())}
}
#[cfg(not(windows))]
async fn terminate_process(process:ActiveProcess)->Result<(),String>{process.child.lock().await.kill().await.map_err(|e|e.to_string())}
fn format_selector(quality:&str)->String{if quality=="best"{"bestvideo*+bestaudio/best".into()}else{let h=quality.trim_end_matches('p');format!("bestvideo*[height<={h}]+bestaudio/best[height<={h}]/best")}}
fn json_u64(value:&serde_json::Value,key:&str)->Option<u64>{value.get(key).and_then(|v|v.as_u64().or_else(||v.as_f64().map(|n|n.max(0.0) as u64)))}
fn json_f64(value:&serde_json::Value,key:&str)->Option<f64>{value.get(key).and_then(serde_json::Value::as_f64)}
fn parse_percent(value:&str)->Option<f64>{value.trim().trim_end_matches('%').trim().parse().ok()}
fn emit_job(app:&AppHandle,job:&DownloadJob){if let Err(error)=app.emit("download-progress",job.progress()){tracing::warn!(job_id=%job.job_id,%error,"could not emit download progress");}}
fn validate_destination(directory:&str)->Result<(),String>{let dir=PathBuf::from(directory);fs::create_dir_all(&dir).map_err(|e|format!("Cannot use download directory: {e}"))?;if !dir.is_dir(){return Err("The download destination is not a folder.".into())}Ok(())}

fn is_permanent_error(stderr:&str)->bool{let s=stderr.to_lowercase();["private video","video unavailable","has been removed","sign in to confirm","login required","unsupported url","requested format is not available","no video formats found","this video is not available"].iter().any(|v|s.contains(v))}
fn permanent_message(stderr:&str)->String{let s=stderr.to_lowercase();if s.contains("private")||s.contains("sign in")||s.contains("login required"){"This video requires authentication and cannot be downloaded in anonymous mode.".into()}else if s.contains("format"){"The requested format is not available for this video.".into()}else{"This video is unavailable and cannot be retried automatically.".into()}}
fn is_disk_full(stderr:&str)->bool{let s=stderr.to_lowercase();s.contains("no space left")||s.contains("disk full")||s.contains("not enough space")||s.contains("os error 112")}

fn queue_path(app:&AppHandle)->Result<PathBuf,String>{let dir=app.path().app_config_dir().map_err(|e|e.to_string())?;fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir.join("download-queue.json"))}
fn load_jobs(app:&AppHandle)->Result<HashMap<String,DownloadJob>,String>{let path=queue_path(app)?;let backup=path.with_extension("json.bak");let bytes=fs::read(&path).or_else(|_|fs::read(&backup));match bytes{Ok(data)=>{let jobs:Vec<DownloadJob>=serde_json::from_slice(&data).map_err(|e|format!("Cannot read persisted download queue: {e}"))?;Ok(jobs.into_iter().map(|j|(j.job_id.clone(),j)).collect())},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Ok(HashMap::new()),Err(e)=>Err(e.to_string())}}
fn save_jobs(app:&AppHandle,jobs:&[DownloadJob])->Result<(),String>{let path=queue_path(app)?;let temp=path.with_extension("json.tmp");let backup=path.with_extension("json.bak");let bytes=serde_json::to_vec_pretty(jobs).map_err(|e|e.to_string())?;{let mut file=fs::File::create(&temp).map_err(|e|e.to_string())?;use std::io::Write;file.write_all(&bytes).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;}if path.exists(){let _=fs::copy(&path,&backup);fs::remove_file(&path).map_err(|e|e.to_string())?;}fs::rename(&temp,&path).map_err(|e|e.to_string())}
