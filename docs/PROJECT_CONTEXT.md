# Project context

AI Skill Manager is a Windows desktop application for managing whole local skill folders across Codex, Claude Code, Antigravity IDE, and OpenCode. The approved v1 scope is defined in the [design](specs/2026-10-01-ai-skill-manager-design.md) and [phased plan](plans/2026-10-01-ai-skill-manager-plan.md).

## Current application

Phase 1 implements discovery and complete-tree comparison. Phase 2 adds confirmed batch installation of Missing skills. Phase 3 adds complete-folder replacement of Different skills and recycling of identifiable installed skills. Each harness tab scans only its configured destination. Folder names establish identity; metadata is descriptive. Comparisons include hidden/nested files, contents, empty directories, entry types, and link/junction warnings. Invalid sources and scan failures remain visible.

The app remembers the last accessible source, destination overrides, and theme in `settings.json` under Tauri's app configuration directory. Dark is the default; the user can change theme or select another folder. Cancelling a picker does not save settings. Unavailable remembered sources remain displayed. Malformed settings are reported and protected against automatic overwrite.

All actions use backend-held scan baselines and one-use confirmation tokens, serial operations, and individual results. Install and Update use complete staged copies and content verification. Update recycles the complete old folder before placing its replacement; placement failures retain the prepared copy and report original and staging paths for manual recovery. Uninstall recycles the identified installed entry, including installed-only, malformed, and readable-baseline error cases. Unidentified folders and directory-valued `SKILL.md` entries remain ineligible.

Link/junction content is materialized only after explicit acknowledgement. Windows recycling uses `IFileOperation` with recycle-on-delete/early-failure flags and a lexical final item path, preserving external link targets. There is no permanent-delete fallback or in-app restore. Unreadable-file removal baselines use entry metadata and hashes where readable; inaccessible/uninspectable trees disable that mutation. The app refreshes after execution and blocks ordinary closure during a running batch.

## Implementation map

- `src/App.svelte`, `src/app.css`: interface and local state.
- `src/api.ts`: typed settings, scan, preparation, and execution commands with progress channels.
- `src-tauri/src/lib.rs`: Tauri command wrappers; scans run on a blocking worker.
- `src-tauri/src/manager.rs`: discovery, metadata, complete-tree comparison, Windows reparse handling, and disposable tests.
- `src-tauri/src/operations.rs`: held scan baselines/confirmation tokens, operation guard, staging and verified installation, and disposable operation tests.
- `src-tauri/src/settings.rs`: defaults, settings validation, safe replacement writes, and persistence tests.
- `src-tauri/tauri.conf.json`: identifier `com.tjm.ai-skill-manager`, local assets, CSP, and window limits.

The stack is Tauri 2, Rust, Svelte 5, TypeScript, Vite 8, and Tailwind 4. Dependencies are resolved in npm and Cargo lockfiles. Only native folder-open permission is added to the default Tauri capability; JavaScript has no general filesystem or shell permission.

## Validation evidence

October 1, 2026, Windows: frontend check/build, Rust formatting, clippy with warnings denied, 14 Rust tests, and a debug executable build passed. Tests exercised real temporary Windows junctions, cycles, repeated targets, and external-target preservation.

Playwright/Edge at `http://127.0.0.1:1420` passed mocked-Tauri interface checks at 1180×780 and 920×620: search/filter/details, independent selections, cancelled picker, theme switching and reload, console health, and visual inspection. This does not validate native dialogs or actual IPC.

The executable started and created a Windows window. On October 1, 2026, the user reported that manual Phase 1 testing looked OK and explicitly authorized Phase 2. This is user-reported native evidence; no detailed native interaction logs were collected.

Phase 2, October 1, 2026: frontend check/build, Rust formatting/clippy with warnings denied, 23 Rust tests, and debug build passed. Tests cover confirmed one-use tokens, unacknowledged link refusal before destination creation, complete hidden/nested/empty-directory copies, real Windows junction materialization and target preservation, changed sources, competing destination folders, source junction retargeting, resolved overlap, staged corruption, owned cleanup, plan invalidation, and serialization.

Mocked-Tauri Playwright/Edge QA passed at 1180×780 and 920×620: confirmation warnings/skipped selections, cancellation without execution, Escape behavior, disabled path controls during execution, partial results, refresh, source picker saving, and theme switching. Console/overlay checks and screenshot inspection passed. These checks do not validate actual IPC or desktop close prevention. The user subsequently reported successful manual Phase 2 testing. Cleanup failures report the remaining staging path; there is no in-flight cancellation.

Phase 3, October 1, 2026: frontend check/build, Rust formatting/clippy with warnings denied, 29 Rust tests, debug build, and `npm.cmd run tauri build -- --no-bundle` passed. The local optimized executable is `src-tauri/target/release/ai-skill-manager.exe`; no installer was generated. Tests exercise the unified production flow, full replacement/obsolete-file removal, partial batches, stale inputs/settings, source overlap, identifiable invalid-source uninstallation, recycling failure, and placement failure with preserved recovery paths. Recycling and placement failures use small private test seams; unit tests do not touch the system Recycle Bin.

Mocked-Tauri Edge QA passed at 1180×780 and 920×620 for action eligibility, Recycle Bin warnings, skipped selections, link acknowledgement, cancellation, Escape, disabled controls, partial results, refresh, and themes. Visual inspection confirmed the wrapped toolbar and light/dark readability. On October 1, 2026, the user reported successful manual Phase 3 testing and accepted the phase. This is user-reported desktop evidence; no detailed interaction logs or itemized coverage were supplied.

A separate temporary native probe compiled the extracted production Windows recycling function, with a canonical-parent resolver for its existing disposable parent paths. On C: and D:, actual Recycle Bin removal and programmatic restore preserved complete folders, hidden resources, empty directories, root/internal junction entries, and external targets; locked-folder recycling was refused with the original retained. This validates the native primitive, not the complete desktop operation. Symbolic-link creation was unavailable (Windows error 1314); unavailable/non-recyclable volumes and manual Explorer restoration remain unverified.

## Workspace note

Phase 1 is committed as `cf52e50`; Phase 2 is committed as `b3f85be` and pushed to `origin/main`. On October 1, 2026, the user reported successful manual Phase 3 testing and authorized its commit/push and continuation. All three phases of the approved plan are accepted. No Phase 4 exists in that plan; the next scope must be defined before implementation. Packaging, signing, and release publication remain separately gated.

Unused `.vite-template`, `static`, and local `.vscode` folders are ignored. Their cleanup was blocked by shell policy. Generated build/schema output is ignored; npm and Cargo lockfiles remain versioned.
