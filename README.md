# AI Skill Manager

Desktop app for read-only comparison of local skill folders across Codex, Claude Code, Antigravity IDE, and OpenCode.

## Run

Install the JavaScript dependencies with `npm install`, then start the Windows desktop app with `npm run tauri dev`. The app remembers the last accessible source folder and stores destination overrides and theme in its local app settings file.

## Checks

Run `npm run check`, `npm run build`, and these Rust checks:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Disposable desktop check

Create one temporary source parent and four temporary destination folders. Add a few skill subfolders with `SKILL.md`, then try source and destination selection, all four tabs, refresh, search and status filtering, tree differences, link warnings, keyboard navigation, and both themes. Restart the app to check source and theme persistence. Keep the test folders disposable; Phase 1 does not change skill directories.
