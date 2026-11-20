# UX refinement implementation plan

Status: Prepared November 20, 2026 against the [approved specification](../specs/2026-11-20-ux-refinement-design.md). The user approved all four scopes, retained Tailwind and included the source Open folder command. No phase is authorized for implementation yet. Commit/push of later implementation work, packaging, version changes and publication remain separately gated.

## Goal, prerequisites and boundaries

Deliver the four approved scopes as four independently usable phases, in order. Before each phase, read AGENTS.md, project context, work status, decisions, the specification and this plan, and inspect the active checkout. Keep the English interface/documents, current dependencies (Tailwind included), settings schema, revision semantics and every whole-folder operation safeguard.

Each phase requires an explicit implementation request, the six prescribed commands plus `git diff --check`, a report separating automated, mocked browser, native startup, native interaction and user-reported evidence, and a stop for user validation. Accepting one phase does not authorize the next. Do not add later-phase scaffolding. UI eligibility checks never replace backend safeguards.

Mocked browser checks reuse the approach from earlier phases: Vite dev server plus Playwright with a mocked `window.__TAURI_INTERNALS__` (invoke, transformCallback) returning synthetic settings, scans and operation events. Keep the mock in the scratchpad unless the user asks to version it.

## Phase 1 — Behavior-preserving frontend structure

**Goal:** Smaller, readable components and one consistent stylesheet, with no visible or behavioral change.

**Targets:** `src/App.svelte`, new files under `src/components/`, optional `src/state.ts`, `src/app.css`.

1. Before editing, capture baseline mocked screenshots in both themes at 920 × 620, 1180 × 780, 1600 × 1000 and 1100/1101 px, including: populated list with a Different skill focused, empty source, no harness, long paths/notices, each dialog open, a running and a finished operation.
2. Extract components: `TopBar`, `SourceToolbar`, `HarnessNavigation`, `DestinationBar`, `NoticeRegion`, `SkillList` (toolbar, search, status filters, selection controls, rows), `DetailsPane`, `PathDialog`, `AddHarnessDialog`, `ManageHarnessDialog`, `OperationDialog`. Keep scan/operation/settings state, sequence guards and IPC calls in `App.svelte` (or a small `state.ts` if that clearly reduces prop passing). Components receive props and call callbacks; dialogs expose `showModal`/`close` through a bound element or exported function.
3. Preserve markup semantics exactly: labels, ARIA attributes, roles, live regions, disabled conditions, Ctrl+F handling, breakpoint focus transfer, dialog cancel blocking and the order of focusable elements.
4. Merge the two CSS layers in `app.css`: keep the final effective values, remove overridden declarations and unused selectors (`.source-icon`, `.filter-field*`, `.management-folder*`, `.source-copy code.empty`, any others confirmed unused by search). Keep tokens on `:root` / `[data-theme="light"]`. Component-scoped `<style>` blocks are allowed only when they move existing rules without changing them.
5. Keep the `@import "tailwindcss"` entry and Tailwind dependencies unchanged.

**Automated acceptance:** `npm run check` and `npm run build` pass; Rust commands pass unchanged. Re-capture the screenshots from step 1 and compare pixel-by-pixel; any difference must be explained and removed. Re-run the earlier mocked checks: selection union/clear, status counts, stale blocking, scan progress, copy status, operation summaries, Ctrl+F, reduced motion, no page overflow. Zero console errors.

**Native user check:** Run `npm.cmd run tauri dev`, repeat a normal session with disposable folders (choose source, switch harnesses, install/update/uninstall, add/rename/remove a custom harness, toggle theme) and confirm nothing changed.

**Completion gate:** Report the screenshot comparison and checks; stop for Phase 1 acceptance and explicit Phase 2 authorization.

## Phase 2 — Faster everyday actions

**Goal:** Common actions take fewer steps and the main controls are clearer.

**Targets:** `src-tauri/src/lib.rs` (and `settings.rs` if a helper is needed), `src/api.ts`, components from Phase 1, `src/app.css`.

1. **Backend `open_source_folder`.** New command with no arguments besides the app handle: take the operation guard, load settings, return the settings error if present, require a configured source ("Skill library folder is not configured"), then reuse `folder_target` and the existing platform openers (Explorer on Windows, `open_linux_folder` on Linux). Refactor the opener branch of `open_harness_folder` into a shared function so both commands behave identically. Register it in `invoke_handler`; add `openSourceFolder()` to `api.ts`. No capability changes.
2. **Details actions.** In `DetailsPane`, render one button per `eligibleActions` entry of the focused skill, labelled "Install this skill" / "Update this skill" / "Uninstall this skill". Clicking calls a new root function that prepares with `[focusedFolder]` instead of the harness selection, then opens the existing confirmation. After execution, clear only that folder from the selection if it was selected (current behavior clears all selections on success; keep that for toolbar-initiated batches only). Same disabled conditions as the toolbar.
3. **Selection header.** Add a tri-state checkbox (checked/unchecked/`indeterminate`) labelled "Select all visible skills" above the rows. Checked state = all filtered rows selected; clicking when not fully checked does the existing filtered union; clicking when checked removes the visible rows from the selection, keeping hidden ones. Keep the three existing selection buttons.
4. **Action hint and emphasis.** When selection is empty, show "Select skills to install, update or uninstall." next to the action buttons (polite, not an alert). Install uses the primary style only when `installSelectedCount > 0`; otherwise the secondary style.
5. **More menus.** Build one reusable `MoreMenu` component: trigger button with `aria-haspopup="menu"` and `aria-expanded`; popup with `role="menu"` and `role="menuitem"` buttons; Arrow Up/Down/Home/End move focus, Enter/Space activate, Escape and outside click close and return focus to the trigger; Tab closes. Source toolbar: visible "Choose folder…"; menu items Enter path…, Copy path, Open folder. Destination bar: visible "Choose destination…" and "Open folder"; menu items Enter path…, Copy path, Manage harness… (custom only). Disabled items keep their existing conditions.
6. **Filters and search.** Render status chips only when count > 0, plus All and the active filter always. Search also matches `description` (case-insensitive, same normalized query).
7. **Keyboard rows.** In `SkillList`, Up/Down on a focused row summary moves focus to the previous/next visible row summary and sets it as focused details; Space on a row summary toggles its checkbox (respecting disabled conditions). Do not intercept keys inside the search field.
8. **Dialogs.** Wrap Add harness and Manage harness (rename view and remove confirmation) in `<form>` elements so Enter submits the primary action; keep Cancel buttons as `type="button"`. Introduce a `dialogError` separate from `loadError`: functions called from an open dialog write to `dialogError`; the main notice region shows only `loadError`. Clear `dialogError` when a dialog opens or closes.

**Automated acceptance:** Disposable Rust tests for the shared opener target validation: unconfigured source, missing path, file instead of directory, and a valid directory reaching the opener through an injectable program (as `open_linux_folder` already allows on Linux). Mocked browser checks: single-folder prepare from details leaves the selection unchanged and sends one folder; tri-state header in all three states with hidden selections preserved; hint visibility; both More menus by keyboard and pointer including focus return; zero-count chips hidden while the active filter stays visible; description search; Up/Down/Space navigation; Enter submission of both dialogs; dialog errors shown once. Repeat Phase 1 layout checks at the supported sizes in both themes.

**Native user check:** On Windows and Ubuntu, open the source folder from the menu; install, update and uninstall one skill from the details pane; use the More menus and keyboard navigation; add and rename a custom harness with Enter.

**Completion gate:** Stop for Phase 2 acceptance and explicit Phase 3 authorization.

## Phase 3 — Visual clarity and accessibility

**Goal:** Readable text, distinct statuses, consistent icons and plain-language explanations.

**Targets:** `src/app.css`, components, new `src/components/Icon.svelte`.

1. **Contrast.** Compute contrast for every text-colored token (`--text`, `--muted`, `--faint`, `--accent`, `--green`, `--amber`, `--red`, and the new token below) against `--panel`, `--panel-raised` and the tinted chip backgrounds, in both themes, with a small script in the scratchpad. Adjust values to reach at least 4.5:1, changing hue as little as possible. Record the final ratios in the phase report.
2. **Installed only color.** Add `--violet` (or neutral) tokens for both themes and a `.status-installed_only` rule using them; keep Different amber.
3. **Type scale.** Raise base UI text (buttons, rows, details body, notices) from 12 px to 13 px; keep 12 px for eyebrows, chip counts and other supplementary text. Re-verify 920 × 620 fits without page scroll; reduce paddings rather than font size if needed.
4. **Icons.** `Icon.svelte` renders inline SVG paths by name (`sun`, `moon`, `search`, `plus`, `refresh`, `copy`, `more`, `folder`, `empty`, `check`, `alert`, `info`), `aria-hidden="true"`, `currentColor`, 16 px default. Replace every Unicode glyph used as an icon. No icon package.
5. **Path disclosures.** Expanded state shows only the wrapped full path and a compact Copy icon button with an accessible label; the summary keeps the truncated path.
6. **Differences.** Group by kind in fixed order (added, changed, type changed, removed); each group has a heading with its destination-oriented label and count; each row shows the wrapped path and a compact icon Copy button labelled "Copy path {path}". Keep the list scroll region and full paths.
7. **Plain-language copy.** Replace the differences explanation, link explanation, link acknowledgement and recovery notice with the plainer wording from the specification, keeping every safety fact (complete replacement, destination-only removal, link materialization, trash recovery, external refresh/restart). List each old/new string pair in the phase report for review.

**Automated acceptance:** Contrast script output meets 4.5:1 for all text tokens in both themes. Screenshots at 920 × 620, 1180 × 780, 1600 × 1000 and 1100/1101 px in both themes with long names, paths, warnings and 50+ differences; no page overflow; icons render with fonts restricted to a generic sans-serif. Reduced motion and keyboard focus visibility unchanged. Zero console errors.

**Native user check:** Review both themes on Windows and Ubuntu, especially icon rendering and readability at the minimum window size.

**Completion gate:** Stop for Phase 3 acceptance and explicit Phase 4 authorization.

## Phase 4 — Cross-harness overview

**Goal:** See pending work across all harnesses without weakening operation guarantees.

**Targets:** `src-tauri/src/manager.rs`, `src-tauri/src/lib.rs`, `src/api.ts`, new `src/components/OverviewMatrix.svelte`, `HarnessNavigation`, `src/app.css`.

1. **Backend.** Add `manager::scan_overview(settings, harnesses, progress)` returning `OverviewResponse { sourcePath, generatedAt, harnesses: [{ harness, destinationPath, available, error, counts: {status: n}, rows: [{ folderName, name, status }] }] }`. Read and inventory the source once (reuse the source discovery/inventory and `SourceCache` path), then for each visible harness run the existing destination discovery/inventory and comparison. A per-harness failure fills `error` for that harness and continues. Progress reuses `ScanProgress` stages with an added optional `harness` field (frontend type updated; existing events unchanged when absent).
2. **Command.** `scan_overview` command loads settings and visible harnesses. It must not call `operations::begin_scan`, `record_scan` or touch `SCANS`/`PLANS`, and returns no revision. It must not hold the operation guard longer than needed and must not run while an operation executes (same check `scan_skills` uses).
3. **Frontend state.** Hold `overview` plus `overviewOutdated`. Mark outdated on any successful settings save that changes source/destinations/registry, after any operation, and after a scan of a harness whose counts differ from the overview. Never clear outdated except by a new overview.
4. **Sidebar counts.** In the wide sidebar, under each harness label show Missing and Different counts from the overview (text, e.g. "2 missing · 1 different"), or "Unavailable"/"Error" states; add "Outdated" text when stale. The compact select appends counts to option text.
5. **Matrix view.** Add an "Overview" toggle in the destination area. The matrix replaces the comparison panel while shown: sticky header row of harness labels, sticky first column of skill names, cells with icon plus short text status (accessible name "Skill X in Harness Y: Missing"), search and status filter reused from the list. Clicking a cell leaves overview mode, switches harness through the existing `switchHarness`, waits for its scan and focuses that folder. The matrix scrolls in its own region in both axes; the page never scrolls horizontally. Refresh while in overview reruns the overview.
6. Show overview progress in the existing scan progress line, including the harness label.

**Automated acceptance:** Disposable Rust tests: overview statuses and counts equal individual `scan` results for each harness on the same tree; source inventory runs once (instrument through the existing progress events or a test counter); an unavailable destination and an unreadable destination produce per-harness errors while others succeed; a revision recorded before `scan_overview` still prepares and executes successfully afterwards; overview is refused while an operation runs. Mocked browser checks: sidebar counts in both navigation modes, outdated marking after save/operation, matrix filters, cell navigation focusing the right skill, sticky headers, horizontal scroll containment at 920 px with 10+ harnesses, keyboard reachability of cells, both themes.

**Native user check:** With disposable folders and several harnesses (including one unavailable), run Overview, read sidebar counts, open a cell, perform an install, and confirm counts are marked outdated until Overview runs again.

**Completion gate:** Stop for Phase 4 acceptance. Commit/push, version changes, packaging and release require separate explicit authorization.
