# Harness Management Implementation Plan

Status: Phase 1 implemented and reviewed on October 2, 2026 using tjm-multi-agent; automated and mocked browser checks passed, the user reported Phase 1 works and accepted it on October 2, 2026. Planning artifacts were committed and pushed as f5d6948. The user explicitly authorized Phase 1 commit/push and Phase 2 implementation; Phase 2 commit/push and release actions remain separately gated. Source: [approved design](../specs/2026-10-02-harness-management-design.md).

## Goal and prerequisites

Deliver the approved built-in filtering, Explorer opening, and custom harness lifecycle in two independently testable phases. Recheck Git status and repository instructions before each phase; preserve unrelated changes. Keep old settings compatible and the existing filesystem safeguards intact. Use the installed standard-library, Svelte, Tauri, and dialog facilities; no new dependency or plugin is planned.

Design approval and this plan do not authorize implementation, commits, pushes, installers, or release actions. Update project context and work status with evidence after each authorized phase; record the accepted durable registry/visibility choices in DECISIONS without duplicating the spec.

## Phase 1 — Visible built-ins and Explorer opening

**Scope/files:** src-tauri/src/settings.rs, src-tauri/src/lib.rs, src/api.ts, src/App.svelte, and src/app.css only where layout needs adjustment; project context/status/decisions. Keep the existing fixed enum and settings schema in this phase.

1. Factor environment-aware configuration roots out of the current default-destination resolver. Add a testable descriptor builder taking resolved configuration roots and settings. A built-in is visible when its configuration root is a directory or its configured destination differs from the default and is a directory. Return all four descriptors in fixed order through an additive SettingsResponse field; descriptors include id, label, built-in classification, visibility, and destination availability. Keep filesystem probes read-only.
2. Render visible descriptors in App.svelte. On startup and explicit Refresh, reload settings/descriptors before scanning. After settings writes, reload descriptors before choosing the next active tab. Preserve the active visible ID, otherwise choose the first visible ID, otherwise clear scan, focused skill, and stale selections and render a no-harness state with Refresh. Keyboard arrows traverse only visible tabs. Skip scanning when no active harness exists. Do not expose unfinished Add harness controls yet.
3. Add open_harness_folder(harness) in lib.rs and its typed API wrapper. Acquire the existing operation guard, load valid saved settings, resolve the configured destination, and validate/canonicalize an existing directory before passing it as one process argument to explorer.exe on Windows. No cmd.exe, shell command strings, folder creation, generic path argument from the client, or shell plugin. Return clear path/launch errors; other operating systems return an unsupported-platform error.
4. Put Open folder beside Choose destination, disable it when busy or no harness is active, and display command errors through the existing notice. Reuse current button and focus styling. Missing folders produce a clear error without opening an ancestor.

**Automated verification:** Add compact disposable tests for configuration-present/skills-absent visibility, absent configurations, overrides, environment-aware root resolution, no created directories, and Explorer target validation. Separate validation from launching so tests do not start Explorer. Run all commands below. Mocked browser QA covers filtered tabs, keyboard arrows, active-tab fallback after Refresh, empty state, Explorer ID argument/errors, both themes, and busy controls.

**User testing:** Start with the current native app configuration; confirm only locally configured built-ins appear. With a disposable destination override, check Open folder opens that destination and reports its removal. Verify configuration-present/skills-absent behavior through automated disposable tests rather than deleting real harness configuration folders. Native startup alone does not validate Explorer interaction.

**Completion/gate:** Report changes, automated/mock evidence, native checks awaiting user testing, and any limitations; stop. Phase 2 begins only after the user validates Phase 1 and explicitly authorizes Phase 2.

## Phase 2 — Custom harness registration and shared identity

**Scope/files:** src-tauri/src/settings.rs, src-tauri/src/manager.rs, src-tauri/src/operations.rs, src-tauri/src/lib.rs, src/api.ts, src/App.svelte, src/app.css; project context/status/decisions and the README usage guide. Existing dependency manifests and lockfiles need no changes.

1. Replace fixed operational harness identity with strings, preserving codex, claude, antigravity, and open_code exactly. Keep a small fixed built-in table for labels/defaults/detection; introduce no trait hierarchy or plugin layer. Change destinations to a string-keyed map and add custom_harnesses as an ID-to-name map with an empty serde default. Store custom destinations in the existing destination map. Generate custom IDs as custom- plus crypto.randomUUID() in the UI; labels and paths never determine identity. Built-ins precede customs, with customs sorted by stable ID for deterministic ordering.
2. Extend Rust settings validation to reject unknown destination IDs, malformed custom IDs, collisions with built-in IDs, empty trimmed names, and custom entries lacking destinations. Validate accessible directories only when adding a custom entry or changing its path; unchanged unavailable paths remain saveable, including rename/theme changes. Accept duplicate display names and duplicate destinations. Preserve safe writes and malformed-file protection. Older settings load without migration writes and retain all four default destinations/overrides, source, and theme.
3. Adapt scan, operation preparation, held baselines, execution revalidation, logging, and shared-path notices to registered string IDs. Resolve labels and destinations from the shared registry, reject unknown/removed IDs, clone identities only where held state requires ownership, and include customs in shared-path checks. Keep OpenCode notices specific to its built-in ID. Preserve source caching, operation serialization, whole-tree comparisons, overlap checks, link warnings, and recycling behavior. A saved registry change continues to invalidate all scans/plans through save_settings.
4. Make frontend harness/selection state dynamic, with an empty-array fallback per ID and nullable active ID for the empty state. Add an accessible native HTML dialog for Add harness with Name, Choose folder, Add, and Cancel; retain its draft on picker cancellation, save nothing on dialog cancellation, and close/select the new tab only after a successful settings write. Validate names and directory availability in Rust as well as basic UI checks. Failed saves retain draft and existing registry.
5. For the active custom harness, expose Rename harness and Remove harness through a compact management dialog; retain Choose destination for path changes. Confirm removal with explicit no-files-deleted wording. Save metadata/path removal together, clear removed selections, and choose the first remaining visible tab or empty state. Keep custom tabs visible on missing destinations and show the scan/availability error with working destination/removal controls. Block all management controls while busy. Ensure Add harness and Refresh remain available in the empty state. Do not render custom names as HTML.
6. Extend the usage guide and record validation evidence. Do not regenerate screenshots unless needed to explain the new controls; any generated browser screenshots must be labeled demo/mock evidence.

**Automated verification:** Extend compact settings tests for old files, lifecycle/reload, unavailable unchanged paths, invalid registrations, failed writes, and removal with untouched disposable folders. Adapt existing Rust tests and add custom-harness scan/confirmed operation coverage using existing copy/recycle test seams, including stale-plan rejection after removal/path changes and duplicate-destination warnings. Never mutate real harness installations or the real Recycle Bin in unit tests. Run all commands below. Mocked browser QA covers add/rename/remove/cancellation, persistence failures, missing custom folders, duplicate labels, independent selections, accessible dialogs/keyboard navigation, both themes, and busy controls.

**User testing:** Register a disposable skills folder, rename it, restart and verify persistence, change its destination, open it in Explorer, and perform disposable Install/Update/Uninstall checks. Make a custom folder unavailable and verify the tab stays recoverable. Remove its registration and inspect that every remaining file/folder is untouched. Check existing built-in settings still work.

**Completion/gate:** Report automated, mocked browser, and user-reported native evidence separately; stop for final validation. No commit/push or packaging without separate explicit authorization.

## Required checks after each phase

```powershell
npm.cmd run check
npm.cmd run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

## Risks and safeguards

- Visibility detects local configuration, not installed executables. Retained configuration can keep an uninstalled harness visible; this is the explicitly chosen behavior.
- String identity touches both scan and mutation state. Custom operation tests and the complete existing suite must pass before claiming compatibility.
- Explorer launch success does not prove a window displayed the requested folder; actual native interaction remains a user validation step.
- Custom removal changes saved registry only. No recycling or filesystem-removal helper belongs in that flow.
