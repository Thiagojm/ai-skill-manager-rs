# Usability and performance implementation plan

Status: Prepared October 3, 2026 against the [approved specification](../specs/2026-10-03-usability-performance-design.md). The user approved all written design requirements. No implementation phase is authorized or started.

## Goal, prerequisites and boundaries

Deliver the approved improvements in three independently usable phases. Read AGENTS.md, current project context, work status, decisions, the approved specification and this plan before implementation. Inspect the active checkout and preserve unrelated work, including Grok. Keep the English interface/documents, current dependencies, settings schema, IPC safety boundaries and complete-folder operation contracts.

Each phase requires an explicit implementation request, all prescribed checks, a report distinguishing automated/browser/native evidence and a stop for user testing. Acceptance of one phase does not authorize the next. No commits, pushes, packaging, version changes or publication are included. Do not add future-phase scaffolding. Do not replace the filesystem safeguards with UI eligibility checks.

## Phase 1 — Theme persistence, freshness and indexing

**Goal:** Theme changes preserve the current comparison; outdated results cannot initiate mutations; selection computations avoid repeated full-list searches.

**Targets:** `src-tauri/src/settings.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/operations.rs` (regression coverage), `src/App.svelte`, and `src/app.css` (stale notice only).

1. Make settings persistence report an internal operational-change boolean only after a successful write. Compare effective previous/new source, destinations and custom registry, excluding theme. Apply the same missing-built-in defaults normalization used by loading before comparison. Keep the public `save_settings` result `Result<(), String>` and existing JSON unchanged. Under its existing operation guard, invalidate held scans/plans only when the returned boolean is true. Keep existing validation and safe-write behavior; do not swallow malformed/unreadable settings errors.
2. Separate theme handling from the frontend path/registry save flow. Use the existing settings IPC, hold the save busy state, and change local theme only after success. Do not call loadSettings/scanSkills, reset focusedFolder, change filters/selections, or touch source reuse on a theme-only save. Keep failed-save errors visible and the prior theme active.
3. Add explicit stale comparison state, initially false. Mark retained rows stale when the post-operation scan fails; show an alert and disable Install/Update/Uninstall, preparation and selection mutation. Leave inspection, filters, Refresh and path repair usable after the operation ends. Clear stale state only after a successful scan or when discarding the displayed comparison; a successful theme save cannot clear it. Retain ordinary scan error behavior.
4. Derive a folder-to-row Map, selected-name Set, normalized search query and one-pass action counts. Use them for focused details, row checkbox state and action eligibility counts. Rebuild on reactive inputs; preserve folder identity, ordering, per-harness selections and current refresh pruning. Do not introduce a shared state library or extract unrelated components.

**Automated acceptance:** Disposable regression tests prove theme/no-op saves preserve a held scan revision and a usable prepared token; operational changes invalidate them; failed/malformed saves preserve bytes and baseline; old settings missing Grok/default entries normalize without false invalidation; changed file contents are still refused during execution after a theme change. Use existing simulated recycling seams, never real installations. Mocked browser IPC verifies zero new scan calls during theme toggles, retained focus/filter/selection, save failure, stale alert/disabled actions and successful refresh recovery. A large synthetic list checks count equivalence without claiming measured performance gains.

**Native user check:** With disposable source/destination folders, select/filter/focus a skill, toggle both themes, restart to check persistence and confirm an eligible operation still works. Verify a displayed stale-refresh error, where reproducible, blocks actions until Refresh succeeds. Automated fault injection covers failures that cannot be reproduced natively.

**Completion gate:** All automated checks pass or limitations are explicit; report native instructions and stop for Phase 1 acceptance and explicit Phase 2 authorization.

## Phase 2 — Navigation, adaptive workspace and batch selection

**Goal:** Harness navigation scales, the workspace uses available height, and filtered batch selection is explicit and accessible.

**Targets:** `src/App.svelte` and `src/app.css`; use `src/api.ts` types without changing backend interfaces.

1. Replace tablist navigation with a labeled sidebar of native buttons at widths greater than 1100 px, fixed at 184 px, and a labeled native select at 1100 px and below. Render only the active navigation variant; use matchMedia with cleanup, keeping CSS consistent with the same breakpoint. Sidebar buttons identify the active harness with aria-current. When crossing the breakpoint while navigation has focus, move focus to the corresponding active button/select. Keep Add, active-custom Manage, repair/open-folder controls, identity-based switching, missing custom harnesses and no-harness state.
2. Replace the large source card with a compact toolbar. Use a viewport-height grid/flex shell, `min-height: 0` for shrinking grid children and independent list/details scrollers. Keep filters/actions and source/destination controls outside those scrollers. Keep two columns at 920 px and above; stack below 920 px as a defensive fallback. Keep the native window minimum unchanged. Bound notice-region height at 96 px with overflow scrolling when needed; preserve all notice text and alerts. Keep modal scrolling and dialog action access.
3. Add Select filtered (union), Clear selection (all active-harness selections) and Clear hidden selections (only outside filtered results). Show total and hidden selected counts and disable empty/no-op controls as appropriate. Preserve existing busy/stale blocks and independent harness selections. Confirmation always receives the entire selected set and retains backend eligibility/skipped rules.
4. Replace the status select with wrapping accessible filter buttons: All followed by Missing, Different, Identical, Installed only, Invalid source, Ambiguous and Scan error. Compute counts from the complete scan, unaffected by search; use aria-pressed for the single active filter and combine it with the existing search. Do not clear selections on filter changes. Preserve the existing filter reset on harness switches.
5. Enforce supplementary text at least 12 px and search/result text at least 13 px. Add Ctrl+F to focus/select search only with no modal open and outside unrelated editable controls; leave native behavior untouched otherwise. Preserve visible focus, accessible labels, textual statuses and disabled controls. Honor reduced motion without removing textual busy indications. Provide full source/destination path access through focusable text disclosure now; copy buttons belong to Phase 3.

**Browser acceptance:** Inspect both themes at 920 × 620, 1180 × 780 and 1600 × 1000, and 1100/1101 px. No supported viewport has horizontal page overflow or unreachable actions. Verify independent scrolling, long notices/paths/names, many custom harnesses, breakpoint focus transfer, empty states, source/path/custom dialogs, registration lifecycle, keyboard navigation, Ctrl+F and reduced motion. Verify filtered union, duplicate prevention, clear-hidden/all behavior, zero results, unchanged status counts under search, hidden selections reaching confirmation and ineligible skips. Expect no unexpected console errors. Run the full command checklist.

**Native user check:** Resize the app through both navigation modes, use many harnesses, both themes and keyboard controls; select skills, hide some with filters, inspect counts, clear hidden items and review a batch. Confirm source/destination pickers and manual entry still work on Windows and Ubuntu when available.

**Completion gate:** Report rendered browser observations separately from native testing, then stop for Phase 2 acceptance and explicit Phase 3 authorization.

## Phase 3 — Progress, differences and operation summaries

**Goal:** Scans report actual work, destination differences are understandable, paths are copyable and batch outcomes are explicit.

**Targets:** `src-tauri/src/manager.rs`, `src-tauri/src/lib.rs`, `src/api.ts`, `src/App.svelte`, and `src/app.css`. Do not change capability files or dependencies.

1. Add a serde camelCase ScanProgress payload and matching TypeScript interface: stage (`discover_source`, `source`, `discover_destination`, `destination`, `compare`), completed number, total number or null, reusedSource boolean. Extend scan_skills with an optional `onProgress` Channel; keep existing response/revision/reuseSource semantics and no-channel callers. The TypeScript wrapper accepts an optional callback and creates a channel only when supplied.
2. Keep the current test/no-progress scan entry point as a wrapper around a progress-capable scan. Inventory-stage totals count paths actually submitted to the workers; comparison totals count union identities. Emit discovery with completed=0/total=null, then known stage totals starting at zero and completions through total. Empty stages report 0/0. Cached source reuse reports completed=total for cached inventories and reusedSource=true; no additional source traversal. Comparison reports each processed row, including ambiguous/error branches.
3. Serialize the worker completion count increment and callback delivery under one stage-local mutex to prevent regressions; retain at most four workers and existing inventory ordering/results. Progress-send errors are ignored without changing scan success/errors. Retain existing timing logs. Frontend callbacks accept events only for their scan sequence; reset stage state on new requests and finish/failure, including post-operation refresh. Render stage/count/reuse text with a polite live region; show an indeterminate indicator only while total is unknown. Do not derive overall or byte percentages.
4. Translate the four existing difference kinds to the exact destination-oriented labels in the specification. Explain complete replacement and destination-only removal, including the materialized-link caveat. Wrap full difference paths; retain raw kind values and ordering. Add explicit-click Copy controls for source, destination and each relative difference path using navigator.clipboard.writeText. Disabled missing paths do not copy placeholder text. Report success/failure inline through polite status text; never log copied paths. A native clipboard failure remains a reported limitation, not grounds for adding capabilities or a plugin.
5. Track the latest operation progress separately and remove the reverse-array copy in rendering. Retain full result logs and recovery messages. Summarize successful/failed result events and preparation-skipped identities separately, using events for this run only. Command-level failures remain prominent and do not classify missing results as successes. Keep confirmation, link acknowledgement, serial execution, close blocking and post-operation fresh scanning unchanged.

**Automated acceptance:** Disposable scan tests cover stage order, empty stages, individual inventory failure, monotonic completion with concurrent workers, cached-source reuse, all comparison branches, progress sink failure and response equivalence to a no-progress scan. Browser mocks cover delayed progress, stale callbacks, failure/cleanup, cache text, all difference kinds, long literal paths, clipboard success/rejection, successful/partial/command-level failed batches and skipped selections without misleading totals. Repeat layout/keyboard/theme checks and all prescribed commands.

**Native user check:** Scan disposable trees and switch harnesses to observe cache reuse; Refresh must perform a fresh source scan. Copy source/destination/difference paths and paste into a text editor to verify literal contents. Review destination-only removal in Update and final outcome summaries. Test Windows and Ubuntu WebView behavior separately; native availability gaps remain explicit.

**Completion gate:** Every approved requirement has automated/browser evidence or an explicit native validation gap. Update user-facing README behavior and context, report results and stop for final acceptance. No packaging or release follows automatically.

## Common verification and evidence

Run once per coherent phase after final changes; rerun affected checks only if subsequent edits or failures require it:

```powershell
npm.cmd run check
npm.cmd run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
git diff --check
```

Use existing disposable Rust testing patterns and simulated recycling. Browser checks must mock Tauri IPC and channel delivery, including settings/operation failures, rather than acting on real harness installations; a dev server may be used without committing generated fixtures. Meaningful frontend state scenarios can be tested through those browser checks without adding a test dependency. Inspect screenshots, not only DOM assertions. Native validation uses `npm.cmd run tauri dev` with disposable folders.

At each checkpoint update `docs/TODO.md` with phase status and authorization boundaries, `docs/PROJECT_CONTEXT.md` with actual product changes and evidence, and `docs/DECISIONS.md` only for durable choices. Keep the approved spec and this plan authoritative; record material deviations and obtain renewed approval. Do not claim Linux/native/clipboard coverage from Windows Rust or mocked browser tests, and do not advertise quantitative scan speed gains without measurements.
