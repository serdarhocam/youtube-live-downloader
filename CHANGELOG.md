# Changelog

All notable changes to this project are documented here. The project follows semantic versioning (`MAJOR.MINOR.PATCH`).

## [1.5.0] - 2026-08-23

- Added a green SERDARHOCAM developer button beside About with distinctive typography.
- Updated the About developer field to `SERDARHOCAM — YouTube Live Downloader Project`.
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
