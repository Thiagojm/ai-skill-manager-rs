# Project context

AI Skill Manager is a Windows desktop application for managing whole local skill folders across Codex, Claude Code, Antigravity IDE, and OpenCode. The approved v1 scope is defined in the [design](specs/2026-10-01-ai-skill-manager-design.md) and [phased plan](plans/2026-10-01-ai-skill-manager-plan.md).

## Current application

Phase 1 implements read-only discovery and complete-tree comparison. Each harness tab scans only its configured destination. Folder names establish identity; metadata is descriptive. Comparisons include hidden/nested files, contents, empty directories, entry types, and link/junction warnings. Invalid sources and scan failures remain visible.

The app remembers the last accessible source, destination overrides, and theme in `settings.json` under Tauri's app configuration directory. Dark is the default; the user can change theme or select another folder. Cancelling a picker does not save settings. Unavailable remembered sources remain displayed. Malformed settings are reported and protected against automatic overwrite.

There are no installation, update, or removal commands in the current app. Phase 2 adds confirmed installation; Phase 3 adds full replacement and Windows Recycle Bin removal. Both require separate authorization.

## Implementation map

- `src/App.svelte`, `src/app.css`: interface and local state.
- `src/api.ts`: typed calls to the three Rust commands.
- `src-tauri/src/lib.rs`: Tauri command wrappers; scans run on a blocking worker.
- `src-tauri/src/manager.rs`: discovery, metadata, complete-tree comparison, Windows reparse handling, and disposable tests.
- `src-tauri/src/settings.rs`: defaults, settings validation, safe replacement writes, and persistence tests.
- `src-tauri/tauri.conf.json`: identifier `com.tjm.ai-skill-manager`, local assets, CSP, and window limits.

The stack is Tauri 2, Rust, Svelte 5, TypeScript, Vite 8, and Tailwind 4. Dependencies are resolved in npm and Cargo lockfiles. Only native folder-open permission is added to the default Tauri capability; JavaScript has no general filesystem or shell permission.

## Validation evidence

October 1, 2026, Windows: frontend check/build, Rust formatting, clippy with warnings denied, 14 Rust tests, and a debug executable build passed. Tests exercised real temporary Windows junctions, cycles, repeated targets, and external-target preservation.

Playwright/Edge at `http://127.0.0.1:1420` passed mocked-Tauri interface checks at 1180×780 and 920×620: search/filter/details, independent selections, cancelled picker, theme switching and reload, console health, and visual inspection. This does not validate native dialogs or actual IPC.

The executable started and created a Windows window. Native folder selection and a full desktop interaction/restart check remain unverified. The user accepted the current checkpoint provisionally and requested commit/push; this does not authorize Phase 2.

## Workspace note

Unused `.vite-template`, `static`, and local `.vscode` folders are ignored. Their cleanup was blocked by shell policy. Generated build/schema output is ignored; npm and Cargo lockfiles remain versioned.
