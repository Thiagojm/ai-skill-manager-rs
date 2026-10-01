# AI Skill Manager

Desktop app for comparison and confirmed installation of complete local skill folders across Codex, Claude Code, Antigravity IDE, and OpenCode. Update and removal belong to Phase 3.

## Run

Restore locked JavaScript dependencies with `npm.cmd ci`, then start the Windows desktop app with `npm.cmd run tauri dev`. The app remembers the last accessible source folder and stores destination overrides and theme in its local app settings file.

## Checks

Run `npm run check`, `npm run build`, and these Rust checks:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Disposable desktop check

Create one temporary source parent and four temporary destination folders. Add a few skill subfolders with `SKILL.md`, then try source and destination selection, all four tabs, refresh, search and status filtering, tree differences, link warnings, keyboard navigation, and both themes. Restart the app to check source and theme persistence.

For Phase 2, select several Missing skills and cancel the batch confirmation first; verify nothing was copied. Then confirm installation and inspect nested files, hidden files, and empty directories. Existing destination folders must remain unchanged. Check progress, individual results, and the refreshed statuses. Switch tabs to check independent selections.

With a disposable external directory, create a junction inside a source skill. Cancel the link acknowledgement first, then acknowledge and install. The installed entries must be ordinary folders/files and the external target must remain unchanged. After opening a confirmation, change a source file or create its destination folder externally; execution should reject that skill and continue other valid entries. During copying, path controls and ordinary window closure must be blocked. Use only disposable folders for all mutation checks.
