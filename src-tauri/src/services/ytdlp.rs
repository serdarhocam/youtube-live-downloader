use crate::{models::VideoItem, services::binaries::BinaryPaths};
use serde_json::Value;
use std::process::Stdio;
use tauri::AppHandle;
use tokio::process::Command;
use url::Url;

pub fn validate_url(input: &str) -> Result<String, String> {
    let parsed = Url::parse(input.trim()).map_err(|_| "Enter a valid HTTP or HTTPS URL.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") { return Err("Only HTTP and HTTPS URLs are supported.".into()); }
    Ok(parsed.into())
}

fn friendly_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("sign in") || lower.contains("authentication") || lower.contains("private video") { "This video requires authentication and cannot be downloaded in anonymous mode.".into() }
    else if lower.contains("video unavailable") || lower.contains("has been removed") { "This video is unavailable or has been removed.".into() }
    else if lower.contains("unsupported url") { "This URL is not supported by yt-dlp.".into() }
    else if lower.contains("unable to download") || lower.contains("network") { "A network error prevented YouTube metadata from loading.".into() }
    else { "yt-dlp could not load this URL. Open technical details for more information.".into() }
}

fn text(v: &Value, keys: &[&str]) -> Option<String> { keys.iter().find_map(|k| v.get(k)?.as_str().map(str::to_owned)) }
fn num(v: &Value, key: &str) -> Option<f64> { v.get(key).and_then(Value::as_f64) }

fn thumbnail(v: &Value) -> Option<String> {
    text(v, &["thumbnail"]).or_else(|| {
        v.get("thumbnails").and_then(Value::as_array).and_then(|items| {
            items.iter().rev().find_map(|item| item.get("url").and_then(Value::as_str).map(str::to_owned))
        })
    })
}

fn parse_item(v: &Value, parent_url: &str) -> VideoItem {
    let formats = v.get("formats").and_then(Value::as_array);
    let mut qualities: Vec<u32> = formats.into_iter().flatten().filter_map(|f| f.get("height").and_then(Value::as_u64).map(|n| n as u32)).collect();
    if qualities.is_empty() { if let Some(h) = v.get("height").and_then(Value::as_u64) { qualities.push(h as u32); } }
    qualities.sort_unstable(); qualities.dedup(); qualities.reverse();
    let max_height = qualities.first().copied();
    let max_fps = formats.into_iter().flatten().filter_map(|f| num(f, "fps")).reduce(f64::max).or_else(|| num(v, "fps"));
    let id = text(v, &["id"]).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let webpage_url = text(v, &["webpage_url", "url"]).filter(|s| s.starts_with("http")).unwrap_or_else(|| {
        if id.len() == 11 { format!("https://www.youtube.com/watch?v={id}") } else { parent_url.to_owned() }
    });
    VideoItem {
        was_live: v.get("was_live").and_then(Value::as_bool).or_else(|| v.get("live_status").and_then(Value::as_str).map(|s| matches!(s, "was_live" | "post_live" | "is_live"))),
        id,
        title: text(v, &["title"]).unwrap_or_else(|| "Untitled video".into()),
        description: text(v, &["description"]),
        thumbnail: thumbnail(v),
        duration: num(v, "duration"),
        timestamp: v.get("release_timestamp").or_else(|| v.get("timestamp")).and_then(Value::as_i64),
        upload_date: text(v, &["upload_date", "release_date"]),
        uploader: text(v, &["channel", "uploader"]), webpage_url, max_height, max_fps, qualities,
    }
}

pub async fn metadata(app: &AppHandle, input: &str) -> Result<Vec<VideoItem>, String> {
    let url = validate_url(input)?;
    let bins = BinaryPaths::resolve(app)?;
    tracing::info!("yt-dlp metadata invocation");
    let mut command = Command::new(&bins.ytdlp);
    command.args(["--js-runtimes", &bins.deno_runtime_arg(), "--remote-components", "ejs:github", "--dump-single-json", "--no-flat-playlist", "--no-warnings", "--ignore-errors", &url])
        .stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    hide_console(&mut command);
    let output = command.output().await.map_err(|e| format!("Could not start bundled yt-dlp: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() && output.stdout.is_empty() { return Err(format!("{}\n\nTechnical details:\n{}", friendly_error(&stderr), stderr.trim())); }
    let root: Value = serde_json::from_slice(&output.stdout).map_err(|e| format!("yt-dlp returned invalid metadata: {e}\n{stderr}"))?;
    let items = root.get("entries").and_then(Value::as_array).map(|entries| entries.iter().filter(|v| !v.is_null()).map(|v| parse_item(v, &url)).collect()).unwrap_or_else(|| vec![parse_item(&root, &url)]);
    Ok(items)
}

pub async fn version(app: &AppHandle) -> Result<String, String> {
    let bins = BinaryPaths::resolve(app)?;
    let mut command = Command::new(bins.ytdlp);
    command.arg("--version").stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null());
    hide_console(&mut command);
    let out = command.output().await.map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(0x08000000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_item_uses_thumbnail_array_and_upload_date() {
        let value = serde_json::json!({
            "id": "abcdefghijk",
            "title": "Playlist item",
            "upload_date": "20260823",
            "thumbnails": [
                {"url": "https://example.invalid/small.jpg", "width": 120},
                {"url": "https://example.invalid/large.jpg", "width": 1280}
            ]
        });
        let item = parse_item(&value, "https://www.youtube.com/playlist?list=test");
        assert_eq!(item.thumbnail.as_deref(), Some("https://example.invalid/large.jpg"));
        assert_eq!(item.upload_date.as_deref(), Some("20260823"));
        assert_eq!(item.webpage_url, "https://www.youtube.com/watch?v=abcdefghijk");
    }
}
