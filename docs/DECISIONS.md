# Durable decisions

## Usability extension direction — October 3, 2026

The user selected all reviewed improvements, harness sidebar navigation and a top selector for smaller windows, then approved both the conversational design and [written specification](specs/2026-10-03-usability-performance-design.md). Preserve side-by-side list/details at the desktop minimum, disclose hidden selections without silently excluding them from operations, and retain existing whole-folder safety contracts. The [implementation plan](plans/2026-10-03-usability-performance-plan.md) retains separate phase gates; The user subsequently authorized planning commit/push and Phase 1, then accepted Phase 1 and authorized Phase 2 on October 3, 2026. The user subsequently accepted Phase 2 and authorized Phase 3 on October 3, 2026. Phase 3 uses the existing bounded workers and an optional Tauri channel; destination difference labels describe Update replacement. Result summaries count received result events, with preparation skips and command failures separate. On October 3, 2026, the user explicitly authorized the 0.2.0 version bump, commit/push and new release after accepting Phase 3.

## Grok built-in harness — October 3, 2026

The user requested Grok as another default harness. Use stable ID `grok`, label Grok and the documented user-scoped `~/.grok/skills` destination, retaining existing detection and override behavior. Do not create configuration directories during discovery or manage Grok's bundled skills. Explain additional shared agent and Claude Code/Cursor discovery without claiming runtime isolation. Existing settings load missing built-in defaults in memory; no schema migration is required.

## Validated Ubuntu scope — October 2, 2026

The user reported all native acceptance tests passing on Ubuntu 22.04 x64. Public documentation names that validated configuration; Ubuntu 24.04 and other distributions remain unvalidated. Record native results as user-reported evidence separately from Actions compilation/tests, package inspection and downloaded checksum verification. The lack of a 24.04 environment is a documented validation gap, not evidence of failure or approval to claim support there.

## Manual folder paths and Linux desktop metadata — October 2, 2026

Keep native folder pickers and offer literal absolute path entry through the existing backend settings flow. Do not replace GTK or add a dialog dependency to control its window geometry. Preserve structural link comparisons and explain materialization instead of claiming linked sources are identical to ordinary copies. Linux icons use ordinary-density filenames so Tauri's physical-size-based layout matches their PNG dimensions. Normalize only the generated deb desktop filename in the manual Packages workflow, retaining the existing product name, version and identifier; do not rename the application to influence packaging.

## Ubuntu support and CI — approved October 2, 2026

The [Ubuntu extension design](specs/2026-10-02-ubuntu-support-design.md) targets Ubuntu 22.04+ x64 alongside Windows, with actual desktop validation required before support claims. Keep the existing operation/settings architecture and dependencies. Use xdg-open with one validated absolute argument on Linux and retain Explorer on Windows. Linux ordinary regular-file permissions (`0o777`) participate in comparison, stage verification, and stale-input protection; copies use stdlib permission preservation. Directory modes follow creation/umask; ownership/ACLs/xattrs are excluded. System-trash wording covers both platforms without permanent-delete fallback.

Automatic CI checks Windows 2022 and Ubuntu 22.04. Separately gated manual packaging will generate NSIS/deb plus checksums as Actions artifacts, with no automatic tags/releases or publication. Debian, ARM, and AppImage are deferred. Phase 1 is authorized; later phases, commit/push, and release uploads remain separately gated.

## Harness visibility and registry extension — accepted October 2, 2026

The [harness management design](specs/2026-10-02-harness-management-design.md) extends v1: built-ins appear when their default configuration directory or an overridden destination exists, without executable probing or directory creation. A missing skills folder alone does not hide a configured harness. Open folder remains a narrow backend command resolving the saved destination from harness identity, with no general shell permission. Phase 1 implements these decisions.

The approved Phase 2 design adds persistent custom identities independent of names/paths, rename and registration removal without deleting folders, and visible unavailable custom destinations that can be corrected. The user accepted Phase 1 and explicitly authorized Phase 2 on October 2, 2026; these features are now implemented and accepted after the user reported Phase 2 works on October 2, 2026 and authorized its commit/push. Automated, mocked browser, and user-reported native evidence are recorded separately in project context. Existing whole-folder and operation safeguards remain binding.

The following contracts were approved on October 1, 2026 in the [design specification](specs/2026-10-01-ai-skill-manager-design.md). The specification remains authoritative for detailed safeguards.

## Local desktop stack — accepted

Use Tauri 2/Rust with Svelte/TypeScript/Vite/Tailwind. Rust owns filesystem behavior; the frontend displays results and requests narrow commands. Settings use a local JSON file, without a database or service. This keeps the local workflow small and independently testable.

## Whole-folder identity and operations — accepted

Folder name identifies a skill, case-insensitively on Windows. Metadata names do not merge folders. Compare the complete tree and use its contents, not timestamps. Install copies the whole folder using verified staging. Update replaces it completely, including removing obsolete destination-only files.

## Recoverable removal and links — accepted and implemented

Updates and uninstallations send the old complete folder to Windows Recycle Bin. Recycling failure has no permanent-delete fallback. Links/junctions warn and allow the user to continue or cancel; confirmed copies materialize their content, while removal must preserve external targets. The detailed overlap, stale-input, staging, and recovery contracts are in the design and Phase 2/3 plan.

## Preferences and discovery — accepted

Remember the last successfully selected accessible source automatically and keep `Choose folder…` available. There is no Save-as-default button. Persist the last theme, with dark as the initial default. Manage only each tab's configured destination; show shared-path/discovery notices without claiming harness isolation.

## Phased delivery — accepted

Deliver read-only comparison, then installation, then replacement/removal. Each phase stops for user validation and explicit authorization. Acceptance or commit/push of Phase 1 does not authorize later phases or release actions.

## Windows recycling — implementation safeguard

Use Windows `IFileOperation` with an absolute lexical item path and explicit recycle-on-delete flags. The generic `trash` Windows implementation canonicalizes the item path, which can follow a skill-root junction; its undo/warning flags also do not express the required force-recycle contract. Never resolve the final skill entry to its external target before recycling. Native disposable recycling and restoration evidence must remain separate from injected unit tests and desktop interaction evidence.

## Repository metadata discovery — post-v1 adjustment

Exclude immediate `.git` entries from source/destination skill discovery, as requested on October 1, 2026. This exclusion does not change complete-tree treatment of resources inside actual skills. Reuse successful scan inventories for operation baselines; mutation-time revalidation remains mandatory.

## Scan concurrency and source snapshot - accepted correction

Use at most four standard-library workers for independent skill inventories. Keep command-level operation serialization. While a scan runs, prevent competing UI commands rather than queueing redundant scans. Reuse a single in-memory source snapshot only on tab switches; Refresh, settings/path changes, and post-operation scans rebuild it. Always rescan the active destination and retain complete mutation-time revalidation. Log inventory and total scan durations for diagnosis; no timestamp-only equality or persistent cache.

Accepted after the user reported excellent scan behavior on October 1, 2026 and authorized commit/push. Latency acceptance is qualitative; no benchmark timings were supplied.

## Manual validation packages — October 2, 2026

The user authorized Ubuntu Phase 2 after accepting the Phase 1 checkpoint. Packages uses workflow_dispatch only, checks out github.sha, runs CI checks and explicit nsis/deb bundling, validates matching frontend/Tauri/Cargo versions, and retains separate package/checksum/source-commit artifacts for 14 days. The deb adds xdg-utils while retaining generated GTK/WebKit dependencies and existing icons. This does not authorize publication, version changes, or Phase 3. Commit/push of Phase 2 remains separately gated.
