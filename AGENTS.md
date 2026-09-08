# Release packaging

- Use `npm run portable`, `npm run installer`, or `npm run release` for user-facing build outputs. `release` creates both distributions with one shared package number.
- Keep the version and package number in distribution filenames. Do not create or update an unnumbered portable alias.
- `scripts/build-release.ps1` reserves the next number in `scripts/package-sequence.json`; never reset or decrement it. Include the counter in release commits.
- Raw files under `src-tauri/target` are intermediate build outputs, not the distributable packages. Deliver the numbered files under `portable/` and `installer/` with their checksum files.
