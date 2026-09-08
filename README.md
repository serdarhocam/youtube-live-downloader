# YouTube Live Downloader - Portable YouTube Video Downloader for Windows

[![Latest Release](https://img.shields.io/github/v/release/serdarhocam/youtube-live-downloader?label=release)](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**[Download the portable Windows application](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)** · [SHA-256 checksum](https://github.com/serdarhocam/youtube-live-downloader/releases/latest)

Current application version: **1.9.0 / p0008**

[Türkçe README](README_TR.md)

A free, portable YouTube video downloader for Windows 10/11. Download public YouTube videos, playlists, channel streams, and archived livestreams without installing Python, Node.js, yt-dlp, or FFmpeg. The standalone application embeds its own tools and merges separate video and audio streams without unnecessary re-encoding.

Use it only for content you own or have permission to download. It does not bypass DRM, authentication, or CAPTCHA challenges.

## Features

- Video, playlist, and `/streams` URL metadata through yt-dlp JSON
- Thumbnail, title, date, duration, channel, description, resolution, and FPS display
- Individual/multiple selection and a two-download queue
- Best quality or resolution-capped download choices
- Machine-readable byte, speed, ETA, percentage, and merge-stage progress
- Per-download cancellation and yt-dlp `.part` resume support
- Real Pause/Resume/Resume All controls with a restart-persistent queue
- Remove-list-entry action for completed, failed, paused, queued, and active jobs
- Native yt-dlp fragment retries plus application-level transient-error backoff
- Native Windows folder picker and persistent local settings
- One-click opening of the configured destination in Windows Explorer
- Optional JPG thumbnails, safe Windows filenames, and no accidental overwrites
- Anonymous-only operation with human-readable errors and expandable diagnostics
- Daily local logs in the Tauri application log directory
- Runtime Turkish/English language selection, persisted per user
- Compact responsive startup layout; scrolling is reserved for genuinely long result lists
- Linked SERDARHOCAM developer attribution in the header and About dialog

## Architecture

The UI is TypeScript, HTML, and CSS built by Vite. It communicates only through typed Tauri commands and events. Rust performs URL validation, process execution, settings persistence, queueing, cancellation, progress parsing, and logging. Raw URLs are passed as individual process arguments; the application never constructs a shell command from user input.

Important backend modules are under `src-tauri/src/services/`:

- `binaries.rs` resolves packaged executables and development copies.
- `ytdlp.rs` owns metadata/version calls and JSON conversion.
- `downloader.rs` owns format selection, queueing, processes, progress, and cancellation.
- `settings.rs` stores settings as JSON in the normal per-user application configuration directory.

## End-user dependencies

None. Users do not install Node.js, npm, Python, Rust, Tauri, yt-dlp, or FFmpeg. The primary portable executable contains yt-dlp and FFmpeg and extracts validated copies automatically. Windows 10/11 normally includes the required Microsoft Edge WebView2 runtime.

## Developer dependencies

- Windows 10/11
- Node.js 20 or newer
- Rust stable MSVC toolchain
- Visual Studio 2022 Build Tools with Desktop development with C++ and a Windows SDK
- WebView2 runtime
- Git LFS when cloning or publishing the repository (bundled FFmpeg files exceed GitHub's normal per-file limit)

Install JavaScript packages with `npm install`.

For the first GitHub checkout or before the first push:

```powershell
git lfs install
git lfs pull
```

The scoped `.gitattributes` rules store only the bundled executables in LFS. Verify that `src-tauri\binaries\*.exe` are real `MZ` executables-not small LFS pointer text files-before building.

## Bundled executables

Repository copies live in `src-tauri/binaries/`:

- `yt-dlp.exe` - official executable from the [yt-dlp releases](https://github.com/yt-dlp/yt-dlp/releases)
- `ffmpeg.exe` and `ffprobe.exe` - Windows essentials build from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/), built from FFmpeg source
- `VERSIONS.txt` - pinned versions and source URLs

The Rust binary manager links yt-dlp and FFmpeg directly into the application with `include_bytes!`. At startup it computes the embedded payload hash and extracts them under `%LOCALAPPDATA%\com.ytld.desktop\embedded-binaries\v1\<hash>`. Existing files are reused only after size and SHA-256 validation. Extraction uses a cross-process lock, temporary file, flush, verification, and atomic rename. yt-dlp always receives the extracted FFmpeg directory via `--ffmpeg-location`; neither executable is searched in `PATH`.

The same source binaries remain listed as Tauri resources in `tauri.conf.json` so the existing NSIS installer continues to work as an optional distribution. The portable executable does not depend on those external resource copies.

To update yt-dlp, replace `src-tauri/binaries/yt-dlp.exe` with a verified official release, update `VERSIONS.txt`, run its `--version` command, and rebuild. No source change is required.

## Development

```powershell
npm install
npm run check
npm run tauri dev
```

Backend checks:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Build cache and disk usage

Rust stores compiled dependencies, debug artifacts, release intermediates, and incremental compilation data under `src-tauri/target`. Because the portable application embeds the large yt-dlp and FFmpeg payloads with `include_bytes!`, repeated debug and release builds can make this directory unusually large. It is generated cache, not source code or user download data.

Reclaim the space safely with:

```powershell
cargo clean --manifest-path src-tauri/Cargo.toml
```

This removes only Rust build products under `src-tauri/target`. It does not remove source files, `src-tauri/binaries`, the already copied `portable/`, user downloads, settings, or the persistent download queue. The next Rust/Tauri build will take longer because all dependencies must be compiled again. `src-tauri/target/` is excluded from Git by `.gitignore` and must never be committed.

## Versioned distribution packages

- `npm run portable`: Build the portable EXE only.
- `npm run installer`: Build the NSIS installer only.
- `npm run release`: Build both with the same version and package number.

Example outputs:

- `portable/YouTubeLiveDownloader-v1.6.0-p0001-portable.exe`
- `installer/YouTubeLiveDownloader-v1.6.0-p0001-installer.exe`

Each EXE has a matching `.exe.sha256` checksum file. The package counter in `scripts/package-sequence.json` increases for every packaging invocation and never resets on version changes. Failed builds consume a number. Commit the counter with release changes. A lock prevents concurrent packaging and existing packages are never overwritten.

Use these commands for distribution. Direct Tauri commands produce raw intermediate artifacts under `src-tauri/target`; distributable version/package-numbered files live under `portable` and `installer`. Existing unnumbered executables remain old copies and are no longer updated.

Development: `npm run tauri dev`. Before release: `npm run check` and `cargo test --manifest-path src-tauri/Cargo.toml --lib`.

## Versioning and release workflow

Versions follow semantic versioning:

- `PATCH`: bug fix without changing expected behavior.
- `MINOR`: backward-compatible feature or substantial reliability improvement.
- `MAJOR`: breaking settings, queue or user-facing behavior change.

Update all synchronized project version fields with:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/set-version.ps1 -Version 1.2.0
npm install --package-lock-only
```

Then update `CHANGELOG.md`, run the full release checks above, commit, create a Git tag such as `v1.2.0`, and attach the portable executable, its SHA-256 value, and optionally the NSIS installer to the GitHub Release. Application, npm package, Cargo package and Tauri bundle versions must remain identical.

Bundled tool versions are tracked separately in `src-tauri/binaries/VERSIONS.txt`. Updating yt-dlp or FFmpeg does not require changing application code, but a new application patch release is recommended so users can identify the new payload.

## Repository policy

The following files are intentionally committed because a clean checkout needs them to create an offline-capable end-user build:

- Rust, TypeScript, CSS and build scripts
- `package-lock.json` and `src-tauri/Cargo.lock`
- `src-tauri/binaries/yt-dlp.exe`
- `src-tauri/binaries/ffmpeg.exe` and `ffprobe.exe`
- `src-tauri/binaries/VERSIONS.txt`

Generated `node_modules`, `dist`, Rust targets, portable outputs, downloaded archives, validation media, `.part` files, logs and machine-specific runtime state are excluded by `.gitignore`. Do not commit cookies, authentication data, downloaded videos, queue state or user settings.

## Extending the project

The existing service boundaries allow future work without replacing the architecture. Practical additions include browser-cookie authentication as an explicit opt-in, subtitle selection, audio-only downloads, per-playlist filters, download history cleanup, signed updates, localization and automated GitHub Actions release builds. Authentication bypass, DRM circumvention and CAPTCHA bypass remain intentionally out of scope.

When adding features, keep process execution inside the Rust services, pass arguments without a shell, preserve the embedded-binary manager, add new persistent queue fields with backward-compatible Serde defaults, and validate both the portable executable and optional installer.

## Settings and logs

The default destination is `Videos\YouTube Live Downloader` (falling back to Downloads when necessary). Download directory, default quality, concurrency, thumbnail preference, and interface language are stored in Tauri's per-user application configuration location. Logs use Tauri's per-user application log location and do not contain cookies or authentication data.

## Language and layout

The header language selector switches the interface between **Türkçe** and **English** immediately. The choice is saved in `settings.json` and restored on the next launch. Existing settings and persistent queue files created before version 1.2 remain compatible and default to English until the user changes the selection.

The default window is 1280×860 with a compact responsive layout. On shorter displays, card thumbnails, padding and optional description previews shrink automatically so the startup view and a small restored queue fit without an unnecessary vertical scrollbar. Playlists and queues that are genuinely taller than the available screen remain scrollable.

Typical Windows locations for identifier `com.ytld.desktop` are:

- Settings and persistent queue: `%APPDATA%\com.ytld.desktop`
- Extracted embedded tools: `%LOCALAPPDATA%\com.ytld.desktop\embedded-binaries\v1\<payload-hash>`
- Logs: the Tauri application log directory for the current user

These locations are application-owned runtime storage. They are created automatically and are not dependencies from the development repository.

## Console-window policy

The application itself is compiled with the Windows GUI subsystem. Every bundled yt-dlp invocation-metadata loading, version lookup and downloading-and the process-tree cancellation helper uses `CREATE_NO_WINDOW`. Normal Load, About, Download, Pause and Cancel operations therefore do not open PowerShell or Command Prompt windows.

## Live progress

yt-dlp is configured with explicit `--progress`, `--newline`, `--progress-delta 0.25` and a JSON progress template. Explicit `--progress` is important because the `after_move` print hook otherwise enables yt-dlp's quiet behavior and suppresses intermediate output. Rust reads progress from both stdout and stderr, calculates percentage from exact downloaded/total byte fields when available, and emits structured Tauri events. The UI updates the bar, percentage, bytes, speed and ETA without waiting for completion. Some YouTube stages do not report a total size; during those stages bytes and status remain truthful while percentage is omitted until yt-dlp supplies a total.

## Removing queue entries

Every persisted download card has a **Remove** action. Removing an active job first terminates only its related yt-dlp process. The job is then removed from `download-queue.json` and from the visible list. Completed media, `.part` files, fragment state, thumbnails, and other files on disk are deliberately preserved. This avoids silently deleting large user files. Removing a record is therefore different from deleting downloaded data; file cleanup remains an explicit user action outside the application.

## Interruption recovery

Download state is written to `download-queue.json` in Tauri's per-user application configuration directory (normally `%APPDATA%\com.ytld.desktop`). Writes are throttled to at most once every two seconds during progress and use a flushed temporary file plus a backup. URLs, item metadata, quality, destination, output template, progress, timestamps, and retry state are retained; process IDs are never persisted.

Active states found after an application restart become **Interrupted** and wait for explicit Resume or Resume All. Pause terminates only that job's yt-dlp process and keeps `.part` and fragment state. Cancel also retains partial data and stops scheduling; it may be resumed manually. Historical progress is labelled approximate until yt-dlp emits fresh progress.

Every download explicitly uses:

```text
--continue --part --no-overwrites
--retries 10 --fragment-retries 20
--extractor-retries 5 --file-access-retries 5
--retry-sleep http:exp=1:20
--retry-sleep fragment:exp=1:20
--retry-sleep extractor:exp=1:10
--concurrent-fragments 4
```

After yt-dlp exhausts its native retries, the application retries transient failures up to ten times with exponential delays capped at 60 seconds. Permanent availability/authentication/format errors fail without a retry loop. Exhausted transient failures and disk-full errors become resumable **Interrupted** jobs. yt-dlp's deterministic output template lets it reuse completed component streams after an interrupted merge; source streams are removed by yt-dlp only after a successful merge.

## Limitations

- Public and URL-accessible unlisted content only; no login or cookies in v1.
- YouTube changes frequently, so keeping bundled yt-dlp current is necessary.
- Metadata availability varies by item. Missing descriptions, dates, formats, or sizes are displayed gracefully.
- Playlist and channel URLs request detailed entry metadata so thumbnails, upload dates, durations and format information are available when YouTube exposes them. Very large lists can therefore take longer to load, but processing remains off the UI thread. Deleted/private entries and content for which YouTube omits dates still display graceful fallbacks.

## License

YouTube Live Downloader's own source code is available under the [MIT License](LICENSE). Bundled yt-dlp and FFmpeg executables retain their respective upstream licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## 1.6.0: Video folders and playback

New downloads keep each video and its JPG thumbnail together under `Download directory/Video title [video ID]/`. Existing records and partial downloads retain their paths.

Each download card has **Play**, opening an independent desktop window with play, pause, stop, -10/+30 second seeking, 0.25-10x speed, volume and fullscreen controls. Playback depends on Windows WebView2 codec support; unsupported files display an error.

**Show in Folder** opens that record's directory in Explorer. Files are checked on startup, main-window focus and completion. Completed records show missing video, expected JPG thumbnail or directory warnings. Missing videos cannot be played, and missing folders are not silently recreated.

## 1.7.0: Chat archive and Resolve

Chat Studio archives available replays, analyzes 15-second sliding windows, imports straight-cut Final Cut Pro 7 XML and exports retimed SRT or transparent QuickTime Animation MOV overlays. Recent messages from removed sections can remain visible as temporary history. See the [Turkish workflow guide](CHAT_WORKFLOW_TR.md) for settings and limitations.

## 1.9.0 / p0008

- [Portable EXE](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.0/YouTubeLiveDownloader-v1.9.0-p0008-portable.exe)
- [Installer EXE](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.0/YouTubeLiveDownloader-v1.9.0-p0008-installer.exe)
- [SHA-256](https://github.com/serdarhocam/youtube-live-downloader/releases/download/v1.9.0/SHA256SUMS.txt)
- [Chat ve Resolve rehberi](CHAT_WORKFLOW_TR.md)

Mesaj süreleri kesim sınırlarında kısalmaz. Geçmiş mesajlar için font ve opaklık ayarları bulunur. MOV ve SRT, Resolve kesimleri etkinse yeni zaman çizelgesini kullanır.

## Publishing updates to GitHub

1. Run `npm run release` once for portable and installer outputs sharing one package number.
2. Test the numbered files and verify their `.sha256` sidecars. Keep build outputs outside Git history.
3. Update the version, numbered download links and release URL in `docs/index.html` and both READMEs.
4. Commit source, documentation and `scripts/package-sequence.json`, then push `main` without force.
5. Publish a GitHub Release with a version tag targeting that commit. Attach the numbered portable and installer files, their checksum sidecars, and `SHA256SUMS.txt`. Mark it as latest.
6. Verify the Release assets and Pages deployment. Pages builds automatically from `main:/docs`.

Do not replace an older release asset or create a new unnumbered executable alias. Preserve old release downloads.
