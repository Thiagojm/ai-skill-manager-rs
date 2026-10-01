# AI Skill Manager — Phased Implementation Plan

Status: Planning complete; implementation not authorized. Source of truth: [approved design](../specs/2026-10-01-ai-skill-manager-design.md).

## Goal and execution rules

Deliver the approved Windows application through three independently testable, user-visible phases. Stop after each phase, report automated and native evidence separately, provide disposable-directory testing instructions, and wait for explicit authorization for the next phase. Approval of this plan does not authorize Phase 1, later phases, commits, pushes, signing, installation, or release publication.

Preserve all design boundaries, particularly full-folder operations, automatic last-source persistence, configured-destination-only management, Recycle Bin removal, confirmed link materialization, and English UI. Do not implement future-phase commands/buttons as placeholders.

## Prerequisites and implementation choices

1. Recheck the active checkout, instructions, and Git status before implementation. Rust/Cargo, Node/npm, and pnpm executables were observed during planning; versions, Windows build prerequisites, and WebView2 readiness remain unverified.
2. Use npm with `package-lock.json` and `npm.cmd` on Windows; commit no files without separate authorization. Scaffold Tauri 2 with the Svelte/TypeScript Vite template only after Phase 1 authorization. Use Svelte 5 and Tailwind 4 via `@tailwindcss/vite`, selecting mutually compatible stable packages and recording resolved versions in lockfiles. Do not force dependency conflicts or use prereleases.
3. Use Tauri's dialog plugin for pickers, Rust `std::fs` for filesystem work, `serde`/`serde_json` for settings and IPC, `serde-saphyr` for YAML frontmatter, and `sha2` for streamed tree fingerprints shared by comparison, staging verification, and stale-input checks. Introduce `trash` only in Phase 3. Use `tempfile` as a dev dependency for disposable tests. No frontend filesystem/shell plugin, database, UI component library, router, or global state library.
4. Keep domain functions in the existing application crate, not a new workspace library. Initial custom files: `src-tauri/src/manager.rs`, `src-tauri/src/settings.rs`, `src/api.ts`, `src/App.svelte`, and `src/app.css`; add `src-tauri/src/operations.rs` only in Phase 2. Reuse template entrypoints and configuration.
5. Store settings in `settings.json` under Tauri's OS-specific app configuration directory. Persist source, destination overrides, and theme. Default theme is dark. Use safe replacement writes; malformed settings show an error and require explicit user correction rather than silently overwriting the file.

## Phase 1 — Read-only comparison application

**Goal:** Launch the actual desktop app, select directories, compare complete skills, and persist preferences without changing skill directories.

**Files:** Template manifests/lockfiles, `index.html`, `vite.config.ts`, `tsconfig*.json`, `src/main.ts`, `src/App.svelte`, `src/app.css`, `src/api.ts`; `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/settings.rs`, and `src-tauri/src/manager.rs`. Keep tests beside the Rust functions they exercise.

**Steps:**

1. Create the template, integrate Tailwind, and expose only required Tauri capabilities. Bundle assets locally, use a restrictive production CSP, and avoid arbitrary shell/file access from JavaScript. Give the app the stable identifier `com.tjm.ai-skill-manager`.
2. Implement settings and OS-aware harness defaults. Source selection automatically persists a validated directory; cancellation is a no-op. Saved unavailable directories remain visible. Destination overrides do not create directories during discovery. Use native pickers.
3. Implement immediate-folder discovery, readable source checks, metadata parsing with folder-name fallback, Windows identity matching, and installed-only enumeration. Keep invalid entries and comparison errors visible, without exposing unrecognized folders as uninstallable skills.
4. Implement one complete-tree inventory/fingerprint routine, including entry types and empty directories. Reuse it for equality and relative-path differences. Detect links/junctions and materialize their contents for comparison while retaining warnings; use the active canonical ancestor chain to detect cycles, permitting repeated noncyclic targets. Report broken/unsupported entries instead of skipping them. A link's presence remains visible even if its materialized bytes match.
5. Build source/destination controls, four tabs, independent selection, search, status filter, details, refresh, English copy, and persistent theme. Add shared-discovery notices. No operation buttons or mutation command in this phase.
6. Expose narrow typed commands through `src/api.ts`: `load_settings`, `save_settings`, and `scan_skills`. Scan returns folder identity, optional metadata, status/error, relative-path differences, link warnings, and an opaque scan revision. Rust owns revisions; the frontend cannot authorize paths by supplying a fingerprint. Offload blocking traversal from the UI/async executor.

**Automated checks:** `npm.cmd run check` (Svelte/TypeScript), `npm.cmd run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

Tests cover the four statuses, same-size changed contents, nested/hidden resources, empty directories, type changes, installed-only entries, malformed metadata, case collisions, unreadable/missing inputs, link cycles, settings persistence, and cancelled selection. Consolidate related cases into focused table-driven tests rather than creating a framework or per-function suite.

**User testing:** Run `npm.cmd run tauri dev`. Use a disposable source and four disposable destination folders containing examples of each status. Verify tabs, paths, metadata warnings, file list, refresh, keyboard navigation, dark default, theme persistence, and remembered source after restart. Confirm no skill directory changed. Browser checks alone do not validate the native picker.

**Completion/gate:** Comparison and preferences work in the Windows app; checks pass or concrete blockers are reported. Stop for user validation and explicit Phase 2 authorization.

## Phase 2 — Confirmed full-folder installation

**Goal:** Install selected Missing skills safely into the active configured destination, with batch results and link warnings.

**Files:** Add `src-tauri/src/operations.rs`; modify `src-tauri/src/manager.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`/lockfile as required, `src/api.ts`, and `src/App.svelte`. Update capabilities only if a necessary permission is demonstrated.

**Steps:**

1. Add `prepare_operation` for Install: validate the scan revision and selected folder identities, resolve configured paths in Rust, reject destructive overlap, and return a backend-held opaque plan token plus eligible/skipped entries and warnings. Display one confirmation for the eligible batch. Do not trust client-provided destination paths or approval details.
2. Add `execute_operation` using the confirmed token. Serialize mutations with a single app-wide operation guard; run selected skills sequentially and stream progress/results using a Tauri channel. Changing settings or rescanning invalidates pending plans; execution rechecks source and destination baselines before each mutation.
3. Create a uniquely app-owned staging directory on the destination filesystem, outside skill enumeration. Copy every entry, materializing links only after explicit warning acknowledgement. Fail cycles, broken links, inaccessible content, unsupported reparse points, or traversal into the destination/staging tree. Verify staged content against the source fingerprint before placement.
4. Place each prepared folder only if its destination is still absent. If another process creates it, report failure without overwriting. Clean only owned staging copies; do not touch unrelated directories. Create a missing configured destination only as part of the confirmed operation.
5. Add Install for eligible selections, one confirmation, current/total progress, individual results, and post-operation refresh. Disable conflicting controls and prevent ordinary window closure while a batch is running. Continue after individual failures. No Update or Uninstall controls yet.

**Automated checks:** Repeat Phase 1 commands after changes. Focused operation tests cover complete nested copies, no overwrite, stale source/destination rejection, resolved overlap, link materialization/cycles, stage verification, owned cleanup, cancelled/unacknowledged warnings, operation serialization, and partial batch success. Inject copy/placement failures through a small private test seam or deterministic test conditions; do not add a general filesystem abstraction.

**User testing:** Install several disposable skills in one destination, inspect every copied file, and refresh another tab to verify independent selections. Cancel a confirmation and a link warning. Test an external junction and confirm the result is ordinary content. Make one input fail and verify other skills complete. Check that the source and external link targets remain unchanged.

**Completion/gate:** Installation operates safely in the actual Windows app; focused tests and native copy/junction evidence are reported separately. Stop for user validation and explicit Phase 3 authorization.

## Phase 3 — Recycle Bin updates and uninstallation

**Goal:** Complete v1 with full-folder replacement and recoverable removal through Windows Recycle Bin.

**Files:** Modify `src-tauri/Cargo.toml`/lockfile, `src-tauri/src/operations.rs`, `src-tauri/src/lib.rs`, `src/api.ts`, and `src/App.svelte`; add `README.md` with setup and disposable native verification instructions.

**Steps:**

1. Add `trash` and extend the existing operation preparation/execution flow to Update and Uninstall. Confirm installed identities, current destination baseline, recycle behavior, shared-path impact, and relevant link warnings. Allow installed-only and malformed installed skills to be uninstalled; never include unidentified destination folders.
2. Update reuses Phase 2 staging and verification, then recycles the original folder and places the replacement. Never fall back to permanent removal if recycling fails. If placement fails after recycling, preserve/report the prepared copy's location and explain manual recovery of the original via Recycle Bin; do not quietly erase the remaining recovery material.
3. Uninstall recycles the selected whole folder. Explicitly verify that links/junctions are removed as entries without deleting their external targets. Unsupported or unsafe recycling behavior must return an error, not substitute recursive deletion.
4. Add eligible Update/Uninstall actions and Recycle Bin wording to the single batch confirmation. Reuse progress, continued processing, results, and refresh. Explain that external harnesses may need refresh/restart and that recovery/retention depends on Windows.
5. Complete keyboard/focus/error/theme checks, document defaults and limitations, and run a local Windows build with `npm.cmd run tauri build -- --no-bundle`. Do not generate/publish a signed installer or modify real harness installations for validation.

**Automated checks:** Repeat the established check/build/Rust commands. Test complete replacement and obsolete-file removal, installed-only removal, stale plan rejection, preparation failure, recycling failure without permanent fallback, placement failure after recycling, source/external-target preservation, and partial batch results. Unit tests use a minimal private recycle function seam and disposable paths, avoiding actual trash modification.

**Native Windows checks:** With disposable directories only, confirm the entire old folder appears in Recycle Bin after Update and Uninstall. Manually restore it to verify its contents. Test symbolic links where permissions allow and directory junctions; verify external target contents remain intact. Exercise unavailable recycling or locked-folder behavior and ensure no permanent fallback. Native observations are required before claiming these safeguards are validated.

**Completion/gate:** Every approved acceptance criterion is addressed; automated, desktop, junction, and actual recycling results are recorded separately, along with limitations. Stop for final user validation. Commits, packaging, signing, release, and Linux work require separate requests.

## References

- [Tauri commands](https://v2.tauri.app/develop/calling-rust/) and [frontend channels](https://v2.tauri.app/develop/calling-frontend/).
- [Tailwind with Vite](https://tailwindcss.com/docs/installation/using-vite).
- [YAML frontmatter parser](https://docs.rs/serde-saphyr/latest/serde_saphyr/) and [system trash library](https://docs.rs/trash/latest/trash/).
