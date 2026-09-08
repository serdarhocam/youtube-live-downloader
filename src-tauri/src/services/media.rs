use crate::models::DownloadJob;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFiles {
    pub job_id: String,
    pub folder: String,
    pub folder_exists: bool,
    pub video_path: Option<String>,
    pub video_exists: bool,
    pub thumbnail_exists: bool,
    pub thumbnail_expected: bool,
}

pub fn inspect(job: &DownloadJob) -> MediaFiles {
    let recorded = job.output_path.as_ref().map(PathBuf::from);
    let folder = recorded.as_ref().and_then(|p| p.parent()).or_else(|| std::path::Path::new(&job.output_template).parent()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(&job.settings.download_directory));
    let video = recorded.clone().filter(|p| p.is_file()).or_else(|| {
        if job.status != "completed" { return None }
        let dedicated = folder.file_name().is_some_and(|name| name.to_string_lossy().ends_with(&format!("[{}]", job.request.item.id)));
        let mut matches = std::fs::read_dir(&folder).ok()?.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let fragment = stem.rsplit('.').next().is_some_and(|part| part.strip_prefix('f').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())));
            path.is_file() && matches!(extension.as_str(), "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v") && !fragment
                && (dedicated || stem.ends_with(&format!("[{}]", job.request.item.id)))
        });
        let candidate = matches.next()?;
        if matches.next().is_some() { None } else { Some(candidate) }
    });
    let thumbnail_exists = video.as_ref().or(recorded.as_ref()).is_some_and(|p| p.with_extension("jpg").is_file());
    MediaFiles { job_id: job.job_id.clone(), folder: folder.to_string_lossy().into_owned(), folder_exists: folder.is_dir(), video_exists: video.is_some(), video_path: video.map(|p| p.to_string_lossy().into_owned()), thumbnail_exists, thumbnail_expected: job.settings.save_thumbnail }
}

pub fn output_template(directory: &str, title: &str, id: &str) -> String {
    let clean = |s: &str, max: usize| s.chars().map(|c| if c.is_control() || "<>:\"/\\|?*%".contains(c) { '_' } else { c }).take(max).collect::<String>().trim_matches([' ', '.']).to_string();
    let folder = format!("{} [{}]", clean(title, 60), clean(id, 24));
    PathBuf::from(directory).join(folder).join("%(title).100B [%(id)s].%(ext)s").to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovers_missing_path_with_unicode_and_truncated_id_without_guessing() {
        let root = std::env::temp_dir().join(format!("ytld-test-{}", uuid::Uuid::new_v4()));
        let folder = root.join("Türkiye İnternet [F1cBHEyfgpQ]");
        std::fs::create_dir_all(&folder).unwrap();
        let video = folder.join("Türkiye — uzun başlık？ [F1cBH.mp4");
        let thumb = video.with_extension("jpg");
        let mut job: DownloadJob = serde_json::from_value(serde_json::json!({
            "jobId":"test", "request":{"item":{"id":"F1cBHEyfgpQ","title":"Türkiye","webpageUrl":"https://youtube.com/watch?v=F1cBHEyfgpQ","qualities":[]},"quality":"best"},
            "settings":{"downloadDirectory":root,"defaultQuality":"best","maxConcurrentDownloads":1,"saveThumbnail":true},
            "outputTemplate":folder.join("%(title)s.%(ext)s"),"status":"completed","stage":"Completed",
            "createdAt":0,"updatedAt":0,"retryCount":0,"maxRetries":10,"outputPath":null
        })).unwrap();
        std::fs::write(&video, b"video").unwrap();
        std::fs::write(&thumb, b"image").unwrap();
        assert_eq!(inspect(&job).video_path.as_deref(), video.to_str());
        assert!(inspect(&job).thumbnail_exists);
        let partial = folder.join("fragment.f137.mp4");
        std::fs::write(&partial, b"partial").unwrap();
        assert!(inspect(&job).video_exists);
        let other = folder.join("other.mp4");
        std::fs::write(&other, b"other").unwrap();
        assert!(!inspect(&job).video_exists, "ambiguous files must not be guessed");
        std::fs::remove_file(&other).unwrap();
        job.request.item.id = "unrelated".into();
        assert!(!inspect(&job).video_exists, "shared folders require the exact video ID");
        for file in [video, thumb, partial] { std::fs::remove_file(file).unwrap(); }
        std::fs::remove_dir(folder).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn detects_deleted_media_and_folder_in_legacy_records() {
        let root = std::env::temp_dir().join(format!("ytld-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let video = root.join("sample.mp4");
        let thumb = root.join("sample.jpg");
        let job: DownloadJob = serde_json::from_value(serde_json::json!({
            "jobId":"test", "request":{"item":{"id":"test","title":"Test","webpageUrl":"https://youtube.com/watch?v=test","qualities":[]},"quality":"best"},
            "settings":{"downloadDirectory":root,"defaultQuality":"best","maxConcurrentDownloads":1,"saveThumbnail":true},
            "outputTemplate":root.join("%(title)s.%(ext)s"),"status":"completed","stage":"Completed",
            "createdAt":0,"updatedAt":0,"retryCount":0,"maxRetries":10,"outputPath":video
        })).unwrap();
        std::fs::write(&video, b"video").unwrap();
        std::fs::write(&thumb, b"image").unwrap();
        assert!(inspect(&job).video_exists && inspect(&job).thumbnail_exists);
        std::fs::remove_file(&thumb).unwrap();
        assert!(inspect(&job).video_exists && !inspect(&job).thumbnail_exists);
        std::fs::remove_file(&video).unwrap();
        assert!(!inspect(&job).video_exists && inspect(&job).folder_exists);
        std::fs::remove_dir(&root).unwrap();
        assert!(!inspect(&job).folder_exists);
    }
    #[test]
    fn template_keeps_untrusted_titles_in_one_folder() {
        let template = output_template("downloads", "../bad\\title:%(id)s", "abc/123");
        let path = std::path::Path::new(&template);
        assert_eq!(path.parent().unwrap().parent().unwrap(), std::path::Path::new("downloads"));
        assert!(!path.parent().unwrap().file_name().unwrap().to_string_lossy().contains('%'));
    }
}
