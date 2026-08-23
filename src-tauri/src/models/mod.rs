use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub timestamp: Option<i64>,
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub webpage_url: String,
    pub max_height: Option<u32>,
    pub max_fps: Option<f64>,
    pub qualities: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub download_directory: String,
    pub default_quality: String,
    pub max_concurrent_downloads: usize,
    pub save_thumbnail: bool,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String { "en".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub item: VideoItem,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub job_id: String,
    pub video_id: String,
    pub status: String,
    pub stage: String,
    pub percent: Option<f64>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub message: Option<String>,
    pub output_path: Option<String>,
    pub technical_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub job_id: String,
    pub request: DownloadRequest,
    pub settings: Settings,
    pub output_template: String,
    pub status: String,
    pub stage: String,
    pub percent: Option<f64>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub retry_count: u32,
    pub max_retries: u32,
    pub next_retry_at: Option<i64>,
    pub message: Option<String>,
    pub output_path: Option<String>,
    pub technical_details: Option<String>,
}

impl DownloadJob {
    pub fn progress(&self) -> DownloadProgress {
        DownloadProgress {
            job_id: self.job_id.clone(), video_id: self.request.item.id.clone(),
            status: self.status.clone(), stage: self.stage.clone(), percent: self.percent,
            downloaded_bytes: self.downloaded_bytes, total_bytes: self.total_bytes,
            speed: self.speed, eta: self.eta, message: self.message.clone(),
            output_path: self.output_path.clone(), technical_details: self.technical_details.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo { pub version: String, pub ytdlp_version: String, pub ffmpeg_version: String }
