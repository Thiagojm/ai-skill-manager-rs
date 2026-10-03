# Usability and performance design

Status: The user approved the conversational design and subsequently explicitly approved this written specification on October 3, 2026. Approval includes all review recommendations, a harness sidebar, and a top selector for smaller windows. The [implementation plan](../plans/2026-10-03-usability-performance-plan.md) is prepared; the user subsequently authorized Phase 1 on October 3, 2026. The user accepted Phase 1 and authorized Phase 2, then reported Phase 2 testing excellent and authorized Phase 3. All three phases are implemented locally; Phase 3 awaits final native user acceptance. Commit/push and release actions remain separately gated.

## Context and goal

The application already supports guarded whole-folder operations on Windows and Ubuntu. This extension improves everyday navigation, batch selection, readability and feedback without changing filesystem operation semantics. The existing v1 and harness-management contracts remain binding.

Confirmed source findings: theme changes currently use full settings persistence followed by a fresh scan and reset focused details; every settings save invalidates held operation state; action counts repeatedly search all rows; filtered-out selections remain selected; skill-list height is capped at 415 px; some auxiliary text is 9–11 px; and a failed post-operation refresh leaves the previous comparison displayed without an explicit stale marker. No measured performance regression or new native validation is claimed.

## Navigation and layout

- Keep the English interface and current light/dark visual identity. Replace horizontal harness tabs with a 184 px sidebar when the viewport is wider than 1100 px. Use a labeled native harness selector at 1100 px and below, with the same active identity and settings flow. Avoid duplicate interactive navigation surfaces.
- Preserve built-in detection, custom harness lifecycle, missing-destination repair, no-harness state and independent per-harness selections. Keep Add harness available in either navigation mode; keep Manage available for the active custom harness.
- At the supported minimum 920 × 620, preserve side-by-side skill list and details without horizontal page scrolling. Keep the configured window minimum unchanged. At widths below the desktop minimum, stack list and details as a defensive browser fallback.
- Replace the source hero card with a compact toolbar showing the selected source, Choose folder and Enter path controls. When no source is selected, include concise guidance. Expose the full source and destination paths on hover/focus or through a labeled copy control.
- Use the remaining viewport height for the comparison. Keep source/destination controls and list filters/actions outside the pane scrollers. Independently scroll the skill list and details; keep dialogs independently scrollable. Notices must remain available and must not make controls unreachable at minimum height: constrain long notice groups in a labeled scrollable region.

## Selection, filters and efficiency

- Add Select filtered, Clear selection, and Clear hidden selections. Select filtered unions every currently visible row with the active harness selection, without changing action eligibility. Clear selection removes all selections for the active harness; Clear hidden selections removes only selected identities outside the current filtered results. Disable selection mutations while existing busy conditions apply and when the comparison is stale.
- Keep selections across search/status changes. Display total selected and hidden selected counts whenever selection is nonempty. Operations review all selected identities, including hidden selections, through the existing confirmation and skipped-item rules. Never silently restrict execution to visible rows.
- Add clickable counts for All and all seven existing statuses. Compute counts from the complete scan, independent of search. Selecting a count sets the single status filter and combines it with search; replace the redundant status dropdown. Preserve current folder/name search semantics.
- Compute the normalized query once per reactive update. Index rows by folder identity and selected identities with Set/Map structures for action counts, selection checks and focused details. Do not introduce virtualization, persistent caches, additional hashing workers or dependencies without measurement and a separate scope.

## Theme persistence and comparison freshness

- Retain the save_settings IPC contract and settings JSON shape. Under the existing backend operation guard, compare the previous effective settings with the successfully persisted settings. Changes to source, destination mappings or custom harness registrations invalidate held scans/plans. A theme-only change or effective no-op does not invalidate them. Missing built-in defaults must be normalized consistently before comparison.
- Keep the existing settings validation, malformed-file protection and safe replacement writes. Failed saves must not change in-memory frontend settings or invalidate a usable baseline. Theme changes remain disabled during scanning, picking, saving, preparation or execution.
- Give theme changes a frontend save path that updates the displayed theme only after successful persistence, without reloading the registry, scanning, resetting focus/filters, clearing selections or changing the source snapshot. Show a save failure with the previous theme retained.
- Track comparison freshness explicitly. If post-operation refresh fails, retain the old rows as reference, mark them with an alert stating that they are outdated, and disable mutation actions and selection mutations until a successful fresh scan. Refresh and path repair remain available. General scan failures continue using the existing error/empty behavior; never present retained rows as current.

## Differences, paths and accessibility

- Keep Added/Removed/Changed/Type changed values in the existing scan response. Display destination-oriented labels: Will be added to destination, Will be removed from destination, Content or permissions changed, and Entry type changed. Explain that this is the complete replacement outcome for Update, including destination-only removal; this section does not preview Uninstall.
- Show complete difference paths through wrapping, rather than inaccessible ellipsis. Provide labeled Copy controls for source, destination and individual difference paths, copying the literal underlying strings. Use the WebView clipboard API on explicit clicks, report failures inline, and add no general clipboard or filesystem capability. Missing paths have no enabled copy control.
- Use at least 12 px for supplementary paths, warnings and labels and 13 px for search/results text. Preserve visible keyboard focus and text status labels. Sidebar navigation uses native buttons with aria-current for the active harness; the compact selector uses native keyboard behavior. Do not retain tablist semantics after removing the tab panels.
- Ctrl+F focuses and selects the skill search only when no modal is open. Do not intercept typing in unrelated inputs. Respect prefers-reduced-motion for spinners and decorative transitions. Announce scan and operation updates politely without moving keyboard focus.

## Scan progress and operation results

- Extend scan_skills with an optional progress Channel, retaining its response, revision, reuseSource behavior and callers that do not request progress. Define ScanProgress with stage (discover_source, source, discover_destination, destination, compare), completed, total (nullable until discovery finishes) and reusedSource. Counts represent skills within the current stage, not bytes or an overall percentage.
- Emit discovery stage transitions, then per-skill inventory completion from the existing bounded workers. Serialize completion counter updates and event delivery so counts cannot regress. A reused source stage reports its full count and reusedSource=true without rereading source inventories. Emit comparison completion as rows are processed. Failed individual inventories still count as completed and retain their existing row errors.
- Channel delivery failure must not fail filesystem scanning. Guard frontend progress callbacks with the existing scan sequence and clear progress on finish/failure, including post-operation refresh. Do not expose paths in progress events; show stage, counts and source reuse. Keep existing timing logs.
- Preserve sequential operations and full per-item recovery messages. Show final counts for successful result events, failed result events and preparation-skipped selections as separate categories. A command-level failure is displayed separately and must not infer success for items without result events.
- Retain a latest-progress value rather than reverse-copying the entire event array during rendering. Keep all result messages in the dialog for this scope, including staging recovery paths. Do not add a persisted operation history, retry executor or in-flight cancellation.

## Compatibility, boundaries and alternatives

No settings migration, dependency addition, new filesystem privilege or change to whole-folder comparison, staging verification, stale-input revalidation, recycling, link acknowledgements or serial execution. Refresh remains explicit; there is no watcher or automatic synchronization. Content diff viewers, remote downloads, restore, packaging, version changes, commit/push and publication are excluded.

The user chose sidebar navigation over incremental tab improvements and a top selector over overlaid details at smaller widths. A narrowly scoped theme save could also avoid invalidation, but classifying operational changes within the existing guarded settings command avoids a second persistence API and retains current error protection. Scan counts use existing inventory work rather than an additional traversal to estimate progress.

## Acceptance and validation

- Use disposable trees only. Verify theme-only/no-op persistence retains a usable backend scan baseline; source/destination/registration changes still invalidate it; malformed and failed saves preserve settings and baseline; and external content changes still fail mutation-time revalidation after a theme change.
- Verify filtered selection union, clear-all/clear-hidden behavior, independent harness selections, all status counts, hidden-selection disclosure and full-selection confirmation with ineligible skips. Include an empty filter, unavailable destination and many custom harnesses.
- Verify post-operation refresh failure shows stale rows and blocks mutation; successful Refresh restores eligibility. Exercise partial operation failures and command-level failures without counting unreported items as successful.
- Verify scan discovery/count transitions, cached source reuse, individual inventory failures, monotonic concurrent events, stale callback rejection, channel delivery failure and unchanged returned comparison. A cached switch must not introduce extra source reads.
- Inspect rendered light/dark interfaces at 1180 × 780, 920 × 620 and 1600 × 1000, plus the 1100 px breakpoint. Exercise long names/paths/warnings, no source/harness/results, modal scrolling, keyboard navigation, Ctrl+F, reduced motion and copy success/failure. Require zero unexpected console errors.
- Run all six repository-prescribed frontend/Rust commands and git diff --check for implementation changes. Distinguish automated disposable tests, mocked browser interactions, direct native interactions and user-reported acceptance. Clipboard and window/layout behavior on Windows and Ubuntu require native acceptance before claiming them verified.

## Delivery and authorization

The implementation plan divides delivery into three usable checkpoints: (1) theme persistence, comparison freshness and indexing; (2) sidebar/adaptive workspace, batch selection, status filters and accessibility; (3) scan progress, destination-oriented differences/copy controls and operation summaries. Each checkpoint ends with tests, native user instructions and a stop for user validation and explicit next-phase authorization. No later-phase scaffolding should be added early. Phase 1 implementation requires a separate explicit request; this specification changes no application code.
