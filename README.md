# AI Skill Manager

Windows desktop app for comparing, installing, updating, and uninstalling complete local skill folders for Codex, Claude Code, Antigravity IDE, and OpenCode. Operations affect only the configured destination shown in the active tab.

## Run

Restore locked JavaScript dependencies with `npm.cmd ci`, then start the app with `npm.cmd run tauri dev`. The app remembers the last accessible source folder and stores destination overrides and theme in its local app settings file.

## Checks

Run `npm.cmd run check`, `npm.cmd run build`, and these Rust checks:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

## Disposable desktop check

Create a temporary source parent and four temporary destination folders. Add several skill folders with `SKILL.md`, then try source and destination selection, all four tabs, refresh, search, status filtering, tree differences, keyboard navigation, and both themes. Restart the app to check source and theme persistence.

Install a Missing skill, then update a Different skill after adding and removing files in the source. Confirm that the replacement contains the complete source tree and obsolete destination-only files are gone. Uninstall an Installed only skill and an installed folder whose `SKILL.md` is malformed or unreadable. The unidentified folders without `SKILL.md` must remain in place. Cancel each confirmation once and verify the destination did not change. Use only disposable paths.

For links, create a disposable external directory and add a directory junction inside an installed skill. Confirm Update or Uninstall after reviewing the warning. The skill folder should move to the Windows Recycle Bin while the external target remains intact. Manually restore the item from Recycle Bin and inspect its files. Repeat with a top-level destination skill junction if Windows permits; the link entry should be recycled while its target remains. If recycling is unavailable or a folder is locked, the operation must report failure and leave the original installed folder in place. Do not use real harness directories for this check.

After opening a confirmation, change the installed tree and verify the operation requires a refresh. For Update, change a source file as well and verify no replacement occurs. To check recovery reporting, use a disposable condition that makes placement fail after recycling; the prepared replacement must remain at the reported staging path, and the old folder must be recoverable manually from Recycle Bin. Windows decides Recycle Bin retention and external harnesses may need a refresh or restart. The app has no in-app restore feature.

Windows recycling calls the Shell on the selected directory entry itself and requests recycle-only behavior. This avoids canonicalizing a top-level junction to its external target; unsupported or failed Shell operations return errors without a permanent-delete fallback.
