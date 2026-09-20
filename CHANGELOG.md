## 1.9.1

- Restore reliable YouTube Shorts extraction by bundling Deno for yt-dlp's JavaScript challenge solver.
- Use the embedded runtime for metadata, media, and chat-replay requests without requiring a separate installation.

## 1.9.0

- Preserve configured chat lifetimes across Resolve cuts; clarify count/height limits.
- Replace history prefix with configurable font size and opacity in preview/MOV.
- Add export-folder action and explicit MOV/SRT timeline indicator; remember XML toggle.

## 1.8.1

- Fix standard YouTube emoji images hosted on fonts.gstatic.com, including robot and smiling face, in preview and MOV export.

## 1.8.0

- Inline nickname/message flow, width-based wrapping and transparent chat styling with text outline/shadow.
- Preserve YouTube rich emote runs without changing raw archives; show images in chat preview and alpha MOV exports.
- Cache export emote images and render frame-aligned transparent scenes.

# Changelog

All notable changes to this project are documented here. The project follows semantic versioning (`MAJOR.MINOR.PATCH`).

## [1.7.0] - 2026-09-08

- Archive available live-chat replays automatically after new livestream recordings finish; existing recordings can fetch chat separately.
- Preserve raw yt-dlp JSONL and derive normalized message data without modifying the original archive.
- Add a separate Chat Studio window with searchable virtualized messages, raw JSON browsing, paid-message events, participant/type statistics and non-overlapping peaks from 15-second windows at 1-second steps.
- Import Final Cut Pro 7 XML straight-cut timelines; validate source clips, rational frame rates and unsupported timing changes.
- Retime chat to the edit and retain a configurable number of recent messages from removed sections as temporary history.
- Export timed SRT (all, paid messages or highlights) and transparent QuickTime Animation MOV chat widgets with colored authors, sliding messages, size/font/lifetime controls and cancellation.
- Custom emoji and stickers use text labels; actual Resolve import testing remains dependent on the user's exported timeline and Resolve version.

## [1.6.1] - 2026-09-08

- Recover missing completed-video paths from the exact video directory, including Unicode titles with truncated filename IDs; persist recovered paths on startup.
- Avoid selecting ambiguous files, intermediate format streams or unrelated videos in shared folders.
- Force UTF-8 yt-dlp output so Windows text decoding cannot silently stop at non-ASCII paths.

## [1.6.0] - 2026-09-08

- Group compact card actions in Play, Show in Folder, Quality, Download Again, Remove order.
- Add permanent version/package-numbered portable and installer packaging with a shared monotonic counter and individual checksums.

- Put Choose Folder before Open Folder.
- Store new video and thumbnail downloads in an individual title/ID directory.
- Add per-video Show in Folder and a separate native playback window with play, pause, stop, -10/+30 second seeking, volume, fullscreen and up to 10x speed.
- Check recorded media on startup, focus and download completion; display missing video, thumbnail or folder warnings.
- Preserve existing download locations and resumable output templates. Playback codec support depends on Windows WebView2.

## [1.5.0] - 2026-08-23

- Added a green SERDARHOCAM developer button beside About with distinctive typography.
- Updated the About developer field to `SERDARHOCAM - YouTube Live Downloader Project`.
- Both developer elements open `https://serdarhocam.com/` through a fixed, shell-free Rust command.
- Updated English and Turkish documentation.

## [1.4.0] - 2026-08-23

- Added localized Open Folder/Klasörü Aç action backed by a safe direct Windows Explorer process invocation.
- Playlist and channel loading now requests detailed entry metadata instead of flat-only records.
- Added thumbnail fallback from yt-dlp's `thumbnails` array and retained upload/release date fallbacks.
- Added regression coverage for playlist thumbnail and date parsing.
- Updated English and Turkish documentation and large-playlist performance expectations.

## [1.3.0] - 2026-08-23

- Fixed live download progress being suppressed by yt-dlp's implicit quiet mode when an `after_move` print hook is used.
- Added explicit `--progress` and retained JSON machine-readable progress events for percentage, bytes, speed and ETA.
- Added a localized Remove action for all persistent download records, including safe process termination for active jobs.
- Removing a record preserves completed media and partial files instead of silently deleting user data.
- Audited the English README and added a complete Turkish translation in `README_TR.md`.

## [1.2.0] - 2026-08-23

- Added persistent runtime Turkish and English interface selection.
- Localized controls, download states, About content, dates and common UI messages.
- Increased the default window size and introduced a compact responsive layout to avoid an unnecessary startup scrollbar.
- Portable builds now create `SHA256SUMS.txt` automatically; manually recording the checksum is no longer required.
- Updated release, localization, layout and checksum documentation.

## [1.1.0] - 2026-08-23

- Added a single-file portable Windows distribution with hash-validated embedded yt-dlp and FFmpeg extraction.
- Added persistent interruption-resistant download queue, Pause, Resume, Resume All and distinct Cancel behavior.
- Added native fragment retries, application-level retry/backoff and restart recovery.
- Changed progress processing to structured JSON events for live percentage, byte, speed and ETA updates.
- Prevented metadata, version and download child processes from opening console windows.
- Added an in-application About dialog with application and bundled tool versions.
- Expanded build, release, versioning and repository documentation.

## [1.0.0] - 2026-08-23

- Initial Tauri/Rust Windows application.
- Public YouTube video, playlist and archived livestream metadata loading.
- Quality selection, queued downloads, FFmpeg merging, folder selection and local settings.
- Optional NSIS installer distribution.
