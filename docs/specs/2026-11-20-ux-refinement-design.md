# UX refinement design

Status: Prepared November 20, 2026 from a review requested by the user. The user approved this written specification on November 20, 2026, selecting all four scopes, retaining Tailwind and including the source Open folder command, and authorized committing it and writing the [implementation plan](../plans/2026-11-20-ux-refinement-plan.md). No implementation phase is authorized yet; packaging and release actions remain separately gated. This specification changes no application code.

## Context and goal

Version 0.2.0 is functionally safe: guarded whole-folder operations, revision-bound plans, overlap checks, system-trash recycling and stale-callback protection work as specified. A review on November 20, 2026 inspected the source and rendered the interface with a mocked backend in light and dark themes at 1280 × 800 and 900 × 700. `npm run check` passed. Rust tests were not executed because the review container lacked GTK system libraries. No native behavior was observed.

Confirmed findings:

- Acting on the skill shown in details requires ticking its checkbox and returning to the list toolbar; details have no actions.
- With nothing selected, Install/Update/Uninstall look like generic disabled buttons. There is no header "select all" checkbox or hint that selection comes first.
- The source area has three equal-weight buttons and the destination area four or five. The source has no Open folder, unlike destinations.
- Status filters always show zero-count chips (for example Ambiguous 0, Scan error 0).
- Expanding a path disclosure repeats the same path already shown in its summary.
- The `--faint` token measures about 3.6:1 (dark) and 3.2:1 (light) against panels, below WCAG AA 4.5:1 for normal text. It is used for skill folder names and result counts.
- Different and Installed only share the same amber color despite different meanings.
- Most text is 12 px. Icons are Unicode glyphs (☼ ☾ ⌕ ▧ ⌑ ＋) whose rendering depends on installed fonts, notably on Ubuntu.
- Each difference row takes two lines plus a full-size Copy button. Explanatory text uses jargon ("materialize links", "destination-only entries").
- Add harness and Manage harness dialogs do not submit with Enter. Errors raised inside a dialog also appear in the main notice region.
- Search ignores descriptions. The skill list has no arrow-key navigation.
- The sidebar shows no per-harness pending work. Checking one skill across harnesses requires visiting each harness, and every scan invalidates the previous revision (`begin_scan`), so cross-harness information cannot be gathered through repeated `scan_skills` calls.
- `App.svelte` holds the entire interface in 793 lines with very long lines. `app.css` contains an original layer overridden by a later "adaptive" layer, plus unused rules (`.source-icon`, `.filter-field`, `.management-folder`, `.source-copy code.empty`). No Tailwind utility class is used, although Tailwind is an accepted stack decision.

## Scope 1: Behavior-preserving frontend structure

- Split `App.svelte` into focused Svelte components: top bar, source toolbar, harness navigation, destination bar, skill list (toolbar, filters, rows), details pane, and one component per dialog (path, add harness, manage harness, operation). Keep state ownership and IPC calls in one place (the root or a small store module). Components receive props and emit callbacks. Do not change rendered markup semantics, labels, ARIA roles or behavior.
- Keep the current Svelte 5 legacy reactive syntax unless a component is rewritten anyway. A full runes migration is out of scope.
- Consolidate `app.css` into a single layer per concern, removing overridden declarations and unused selectors. The rendered result at the supported viewports must match the pre-refactor screenshots apart from intentional changes in later scopes.
- Tailwind remains installed per the accepted stack decision. The user confirmed keeping it.

## Scope 2: Faster everyday actions

- **Details actions.** The details pane shows the actions eligible for the focused skill (Install, Update or Uninstall, from `eligibleActions`). Clicking one prepares an operation for that single folder through the existing `prepare_operation` and confirmation dialog, without changing the harness's checkbox selection. Same disabled conditions as the toolbar (busy, stale, loading).
- **Selection affordances.** Add a tri-state "select all visible" checkbox in the list header that behaves like Select filtered / clears visible rows. Keep Clear selection and Clear hidden selections. When nothing is selected, show a short hint near the action buttons: "Select skills to install, update or uninstall." Give Install the primary style only when it has eligible items.
- **Fewer equal-weight buttons.** Source toolbar: Choose folder… as the visible primary action; Enter path…, Copy path and Open folder in a "More" menu. Destination bar: Choose destination… and Open folder visible; Enter path…, Copy path and Manage harness (custom only) in a "More" menu. The menu is a native button with a keyboard-operable popup (Arrow keys, Enter, Escape, focus return) and no new dependency.
- **Source Open folder.** Add a narrow backend command `open_source_folder` that resolves the saved source from settings, applying the same validation and platform openers as `open_harness_folder`. No general shell or path argument from the frontend.
- **Filters.** Hide status chips with a zero count, except All and the currently active filter. Search also matches the description.
- **Keyboard.** Up/Down move focus between skill rows and update details; Space toggles the focused row's checkbox. Existing Ctrl+F rules stay unchanged.
- **Dialogs.** Add harness and Manage harness use forms so Enter submits. While a dialog is open, its error is shown only inside the dialog.

## Scope 3: Visual clarity and accessibility

- Adjust `--faint` (and any other text token used for content) to at least 4.5:1 against `--panel` and `--panel-raised` in both themes. Decorative borders are exempt.
- Give Installed only a distinct neutral or violet tone. Different stays amber. Status chips keep their text labels.
- Raise the base interface text to 13 px. Keep supplementary text at 12 px minimum and headings proportionate. Verify the 920 × 620 minimum window still fits without page scroll.
- Replace Unicode glyph icons with small inline SVG icons bundled in the source (theme, search, empty states, add, refresh, copy, more). No icon library dependency.
- Path disclosures show the full path once: summary truncates; expanded state shows only the wrapped full path and a copy button.
- Differences: group rows by kind under one heading per kind with a count, each row a single line with the path and a compact icon Copy button with an accessible label. Keep full paths wrapping.
- Rewrite explanatory copy in plain language, for example: "Update replaces the whole folder in the destination. Files that exist only in the destination will be removed." and "Linked files are copied as normal files, so this skill may still show as Different after updating." Same meaning as the current text; no safety statement removed.

## Scope 4: Cross-harness overview

- Add a read-only backend command `scan_overview` that reads the source once and compares it with every visible harness destination, returning per harness: status counts and per-folder status. It reuses existing discovery, inventory and comparison code and the bounded workers, and reports progress through an optional channel like `scan_skills`.
- `scan_overview` must not call `begin_scan`, record a baseline, issue a revision or invalidate held scans or plans. Its results never authorize operations. Mutations continue to require the active harness's normal scan and confirmation.
- Overview runs only on explicit request (an Overview button and Refresh while the overview is shown). No watcher or automatic background scanning.
- **Sidebar counts.** After an overview, each harness entry shows pending counts (Missing, Different) with an "as of" note. Counts are cleared or marked outdated when settings change or an operation runs; they are never presented as current after that.
- **Matrix view.** A view listing skills (rows) against visible harnesses (columns) with a status symbol plus text alternative per cell, search and status filtering. Selecting a cell switches to that harness, runs its normal scan and focuses that skill. At narrow widths the matrix scrolls horizontally inside its own region; the page itself does not.
- Unavailable destinations and per-harness errors appear as their own cell state rather than failing the whole overview.

## Compatibility and boundaries

No settings schema change or migration. No change to whole-folder comparison, staging verification, stale-input revalidation, recycling, link acknowledgements, serial execution or revision semantics. New backend commands are `open_source_folder` and `scan_overview` only, both without frontend-supplied paths. No new dependencies. Bulk operations across several harnesses at once, content diff viewers, restore from trash, file watching, packaging, version changes, commit/push and publication are excluded.

## Decisions (approved November 20, 2026)

1. **Scopes included:** all four.
2. **Tailwind:** kept, per the accepted stack decision.
3. **Source Open folder:** included as one narrow backend command.

## Acceptance and validation

- Scope 1: screenshot comparison before/after at 920 × 620, 1180 × 780 and 1600 × 1000 in both themes shows no unintended differences; all existing mocked browser checks from the usability phases still pass; zero console errors.
- Scope 2: mocked checks for details actions (single-folder prepare, selection unchanged), tri-state header, hint visibility, More menus by keyboard and pointer, zero-count chip hiding with active filter retained, description search, arrow/Space navigation, Enter submission and single error display. Disposable Rust tests for `open_source_folder` validation (missing, non-directory and unconfigured source).
- Scope 3: measured contrast for every text token in both themes; screenshots at the supported sizes; icons render without relying on system fonts; grouped differences with long paths; reduced motion still respected.
- Scope 4: disposable Rust tests prove `scan_overview` produces the same per-harness statuses as individual scans, reads the source once, tolerates unavailable destinations and per-harness errors, and does not invalidate a held revision or plan (prepare after overview still succeeds). Mocked checks for sidebar counts, outdated marking, matrix navigation and horizontal scroll containment.
- Every implementation phase runs the six repository-prescribed commands and `git diff --check`. Evidence is reported separately as automated, mocked browser, native startup, native interaction and user-reported acceptance. Native Windows and Ubuntu validation by the user is required before claiming native behavior.

## Delivery and authorization

After approval, the plan will deliver one phase per approved scope, in order 1 → 2 → 3 → 4, each ending with tests, native user instructions and a stop for user validation and explicit authorization of the next phase. No later-phase scaffolding is added early.
