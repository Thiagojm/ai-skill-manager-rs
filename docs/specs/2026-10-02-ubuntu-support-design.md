# Ubuntu Support and Windows/Ubuntu CI

Status: Design and phased plan approved in chat on October 2, 2026 through the user's explicit implementation request. Phase 1 is authorized; later phases require acceptance and separate authorization. This extends the Windows-only v1 design without weakening its operation safeguards.

## Goal and boundaries

Support Ubuntu 22.04+ x64 alongside Windows. Add automatic checks on both platforms and, in a separately gated phase, manual generation of Windows NSIS and Ubuntu `.deb` packages with SHA-256 checksums. Debian, ARM, AppImage, signing, automatic publication, and changes to existing published assets are excluded. Native support claims require actual desktop evidence.

## Behavior

- Keep narrow backend commands and the existing settings schema and guarded whole-folder operations. No platform framework or new application dependency.
- Open folder validates the configured destination in Rust. Windows keeps Explorer; Linux uses `xdg-open` with one absolute argument and no shell. Missing commands and nonzero exits are errors.
- Resolve home from `USERPROFILE` on Windows and `HOME` on Linux; preserve `CODEX_HOME`, `XDG_CONFIG_HOME`, and destination overrides.
- Describe removal as system trash, with manual recovery and system-controlled retention. Keep Windows recycling and Linux's existing `trash` implementation; never permanently delete as a fallback.
- Preserve ordinary regular-file Linux permissions through `std::fs::copy`, including executable files and materialized file symlinks. Include `0o777` permission bits in comparison, staged verification, and stale-input detection. Directory permissions follow creation/umask. Ownership, ACLs, and extended attributes are excluded.
- Linux folder identity is case-sensitive. Preserve literal backslashes in Unix relative filenames so they cannot collide with nested paths. Existing symlink warnings, cycle/overlap checks, and external-target protection remain mandatory.

## CI and packaging

Automatic CI runs on main pushes, pull requests, and manual dispatch, using Windows 2022 and Ubuntu 22.04 x64 runners, Node 24, Rust stable with rustfmt/clippy, npm/Cargo caches, read-only repository permissions, and superseded-run cancellation. Restore locked dependencies, run frontend checks/build and Rust formatting/clippy/tests, then compile optimized Tauri without bundling. Install Tauri development prerequisites and `xdg-utils` on Ubuntu.

The later manual packaging workflow builds the exact dispatched commit, checks matching frontend/Tauri/Cargo versions, and explicitly selects NSIS or deb. Keep the version and identifier. Use existing icons and generated desktop entry/GTK/WebKit dependencies, adding `xdg-utils` to deb runtime dependencies. Inspect amd64 metadata, version, executable, icons, desktop entry, and dependencies. Upload separate per-platform package/checksum/source-commit artifacts with 14-day retention. Create no tags or releases and upload no release assets.

## Acceptance and validation

Disposable tests cover case-sensitive identities, file permissions and permission-only differences, staged verification, stale scans/confirmations, materialized symlinks, external-target preservation, and recycling failures without permanent fallback. Folder-opener tests use a disposable executable, not the real desktop. Unit tests never alter actual system trash or real harness installations.

CI success requires actual Actions logs; a workflow file is not execution evidence. Native checks on Ubuntu 22.04 and 24.04 cover package installation/removal, launcher/icon, picker, folder opener, settings, built-in/custom harnesses, keyboard/themes, full operations, executable files, symlinks, and actual trash restoration. Removing the application package must retain skills/settings. Distinguish automated, mocked, native interaction, packaging, and restoration evidence.
