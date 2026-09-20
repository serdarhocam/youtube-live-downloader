use sha2::{Digest, Sha256};
use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}, sync::OnceLock, thread, time::{Duration, SystemTime}};
use tauri::{AppHandle, Manager};

// Linked into the Rust executable: the release .exe does not need repository or sidecar files.
const EMBEDDED_YTDLP: &[u8] = include_bytes!("../../binaries/yt-dlp.exe");
const EMBEDDED_FFMPEG: &[u8] = include_bytes!("../../binaries/ffmpeg.exe");
const EMBEDDED_DENO: &[u8] = include_bytes!("../../binaries/deno.exe");
const CACHE_LAYOUT_VERSION: &str = "v2";
static RESOLVED: OnceLock<Result<BinaryPaths, String>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct BinaryPaths { pub ytdlp: PathBuf, pub ffmpeg: PathBuf, pub deno: PathBuf }

impl BinaryPaths {
    pub fn resolve(app: &AppHandle) -> Result<Self, String> {
        RESOLVED.get_or_init(|| EmbeddedBinaryManager::new(app).and_then(|m| m.ensure())).clone()
    }
    pub fn ffmpeg_dir(&self) -> &Path { self.ffmpeg.parent().unwrap_or(Path::new(".")) }
    pub fn deno_runtime_arg(&self) -> String { format!("deno:{}", self.deno.display()) }
}

struct EmbeddedBinaryManager { version_dir: PathBuf }

impl EmbeddedBinaryManager {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let root = app.path().app_local_data_dir()
            .map_err(|e| format!("Cannot locate the application cache directory: {e}"))?
            .join("embedded-binaries").join(CACHE_LAYOUT_VERSION);
        let combined = hash_parts(&[EMBEDDED_YTDLP, EMBEDDED_FFMPEG, EMBEDDED_DENO]);
        Ok(Self { version_dir: root.join(&combined[..20]) })
    }

    fn ensure(&self) -> Result<BinaryPaths, String> {
        fs::create_dir_all(&self.version_dir).map_err(|e| format!("Cannot create the embedded binary cache: {e}"))?;
        let ytdlp = self.version_dir.join("yt-dlp.exe");
        let ffmpeg = self.version_dir.join("ffmpeg.exe");
        let deno = self.version_dir.join("deno.exe");
        if validates(&ytdlp, EMBEDDED_YTDLP) && validates(&ffmpeg, EMBEDDED_FFMPEG) && validates(&deno, EMBEDDED_DENO) {
            return Ok(BinaryPaths { ytdlp, ffmpeg, deno });
        }
        let lock_path = self.version_dir.join("extraction.lock");
        let _lock = acquire_lock(&lock_path, &[(&ytdlp, EMBEDDED_YTDLP), (&ffmpeg, EMBEDDED_FFMPEG), (&deno, EMBEDDED_DENO)])?;
        ensure_file(&ytdlp, EMBEDDED_YTDLP)?;
        ensure_file(&ffmpeg, EMBEDDED_FFMPEG)?;
        ensure_file(&deno, EMBEDDED_DENO)?;
        if !validates(&ytdlp, EMBEDDED_YTDLP) || !validates(&ffmpeg, EMBEDDED_FFMPEG) || !validates(&deno, EMBEDDED_DENO) {
            return Err("Embedded tools failed their post-extraction integrity check.".into());
        }
        Ok(BinaryPaths { ytdlp, ffmpeg, deno })
    }
}

struct ExtractionLock(PathBuf);
impl Drop for ExtractionLock { fn drop(&mut self) { if !self.0.as_os_str().is_empty() { let _ = fs::remove_file(&self.0); } } }

fn acquire_lock(lock_path: &Path, tools: &[(&Path, &[u8])]) -> Result<ExtractionLock, String> {
    let deadline = SystemTime::now() + Duration::from_secs(90);
    loop {
        match OpenOptions::new().write(true).create_new(true).open(lock_path) {
            Ok(mut lock) => {
                let _ = writeln!(lock, "pid={}", std::process::id());
                let _ = lock.sync_all();
                return Ok(ExtractionLock(lock_path.to_path_buf()));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if tools.iter().all(|(path, embedded)| validates(path, embedded)) {
                    return Ok(ExtractionLock(PathBuf::new()));
                }
                if lock_is_stale(lock_path) { let _ = fs::remove_file(lock_path); continue; }
                if SystemTime::now() >= deadline {
                    return Err("Timed out waiting for another application instance to extract embedded tools.".into());
                }
                thread::sleep(Duration::from_millis(150));
            }
            Err(e) => return Err(format!("Cannot lock the embedded binary cache: {e}")),
        }
    }
}

fn lock_is_stale(path: &Path) -> bool {
    fs::metadata(path).and_then(|m| m.modified())
        .and_then(|t| t.elapsed().map_err(std::io::Error::other))
        .is_ok_and(|age| age > Duration::from_secs(180))
}

fn ensure_file(path: &Path, embedded: &[u8]) -> Result<(), String> {
    if validates(path, embedded) { return Ok(()) }
    let temp = path.with_extension(format!("exe.{}.tmp", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = File::create(&temp).map_err(|e| format!("Cannot create {}: {e}", temp.display()))?;
        file.write_all(embedded).map_err(|e| format!("Cannot extract {}: {e}", path.display()))?;
        file.sync_all().map_err(|e| format!("Cannot flush {}: {e}", path.display()))?;
        drop(file);
        if !validates(&temp, embedded) { return Err(format!("Integrity validation failed while extracting {}", path.display())) }
        if path.exists() { fs::remove_file(path).map_err(|e| format!("Cannot replace invalid cached tool {}: {e}", path.display()))?; }
        fs::rename(&temp, path).map_err(|e| format!("Cannot activate extracted tool {}: {e}", path.display()))
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

fn validates(path: &Path, embedded: &[u8]) -> bool {
    let Ok(metadata) = fs::metadata(path) else { return false };
    if metadata.len() != embedded.len() as u64 { return false }
    let Ok(mut file) = File::open(path) else { return false };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop { match file.read(&mut buffer) { Ok(0) => break, Ok(n) => hasher.update(&buffer[..n]), Err(_) => return false } }
    let actual: [u8; 32] = hasher.finalize().into();
    let expected: [u8; 32] = Sha256::digest(embedded).into();
    actual == expected
}

fn hash_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts { hasher.update(part); }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_payloads_are_windows_executables() {
        assert!(EMBEDDED_YTDLP.starts_with(b"MZ"));
        assert!(EMBEDDED_FFMPEG.starts_with(b"MZ"));
        assert!(EMBEDDED_DENO.starts_with(b"MZ"));
        assert!(EMBEDDED_YTDLP.len() > 1_000_000);
        assert!(EMBEDDED_FFMPEG.len() > 10_000_000);
        assert!(EMBEDDED_DENO.len() > 10_000_000);
    }
}
