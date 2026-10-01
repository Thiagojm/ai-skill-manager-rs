# AI Skill Manager — v1 Design Specification

Status: Approved by the user on October 1, 2026. Approval includes automatic persistence of the last selected source and a permanent `Choose folder…` control. Implementation is not authorized.

## Goal and boundaries

Build a Windows desktop application that compares and manages complete skill folders for Codex, Claude Code, Antigravity IDE, and OpenCode. The repository initially contains only Git metadata and has no commits or application code.

Include local source selection, configurable global destinations, comparison, installation, full-folder updates, and uninstallation. Exclude project-local management, CLI, remote downloads, skill editing, file-content diff viewers, automatic synchronization, backup management, and Linux release/testing.

## Stack and responsibilities

- Tauri 2 and Rust backend; Svelte, TypeScript, Vite, and Tailwind CSS frontend.
- English interface, dark theme by default, persistent light/dark switch.
- Rust owns discovery, validation, comparison, copying, and recycling. Filesystem functions remain independent of the UI and Tauri commands. Resolve paths by operating system to support future Linux work; do not create a separate framework or plugin architecture.
- The frontend displays results and requests operations. It never executes skill scripts or renders skill text as executable HTML.
- Use native folder selection and a local configuration file. No database or web service.

## Source and destinations

The source is a parent directory containing immediate skill subfolders. Automatically save the last successfully selected, accessible source directory and preselect it on subsequent launches. Keep `Choose folder…` available at all times outside an active operation. There is no `Save as default` button. If the saved directory becomes unavailable, retain its displayed path, show an error, and allow another selection. Cancelling a picker does not change saved settings.

Each harness has one managed destination, with persistent user overrides:

| Harness | Initial destination |
| --- | --- |
| Codex | `$CODEX_HOME/skills`, falling back to `~/.codex/skills` |
| Claude Code | `~/.claude/skills` |
| Antigravity IDE | `~/.gemini/config/skills` |
| OpenCode | `~/.config/opencode/skills` |

Resolve the user home and applicable environment variables at runtime; never hardcode this user's paths. Respect `XDG_CONFIG_HOME` for the OpenCode config directory where applicable. A nonexistent destination is shown as missing and may be created as part of a confirmed installation. Do not create harness directories merely by opening the app.

Only the configured destination is scanned and managed in each tab. Additional discovery directories are informational. OpenCode also reads the Claude and shared agent directories; changes made under Claude's default directory may therefore affect OpenCode. Show this notice and flag destinations that resolve to the same directory. Do not claim harness runtime isolation or complete enumeration of every skill a harness can load.

The Codex default follows the verified local installer convention, independently of additional discovery paths. Antigravity refers to the IDE, not its CLI or legacy destination.

## Identity and validation

Folder name is the skill identity, with case-insensitive matching on Windows. Frontmatter `name` and `description` are display metadata. Duplicate metadata names do not merge distinct folders. Ambiguous filesystem identities are reported and cannot be mutated until resolved.

A source folder must have a readable `SKILL.md`. Report missing or unreadable files as invalid and do not offer installation/update. Missing, malformed, or unusable metadata produces a warning but allows confirmation with the folder name as fallback. Do not enforce every harness-specific metadata rule.

Installed folders with an identifiable `SKILL.md` remain uninstallable even if its content is malformed or unreadable. Unrecognized destination folders are not deletion candidates. Scan immediate skill directories, not nested collections or installed plugins.

## Interface and comparison

The window contains the source path and picker, four harness tabs, the active destination and its picker, search by folder/display name, status filter, explicit refresh, selectable skill rows, a details panel, and eligible batch actions. Selection is independent per harness; changing source or a destination clears affected selections and invalidates pending confirmations.

Details show description, folder identity, paths, warnings, and relative paths added, removed, changed, or type-changed when replacing the destination with the source. No content diff viewer.

Compare complete trees: relative paths, entry types, empty directories, and file contents, including hidden files and supporting scripts/resources. No extension exclusions. Timestamps do not establish equality or newer/older versions. Unreadable content produces a comparison error rather than an equality result.

| Status | Meaning | Eligible actions |
| --- | --- | --- |
| Missing | Source only | Install |
| Identical | Equivalent complete source/destination trees | Uninstall |
| Different | Both exist with different content or structure | Update, Uninstall |
| Installed only | Managed destination only | Uninstall |

Comparison errors disable install/update where the required source or baseline cannot be verified; identifiable installed folders remain uninstallable with an appropriate warning. An action confirmation contains only eligible selected rows and identifies skipped/ineligible selections.

Refresh after operations and on explicit request. No filesystem watcher in v1. Provide keyboard access, visible focus, accessible labels, and readable themes.

## Operations and safeguards

One confirmation per batch lists action, skill identities, destination, and warnings. Updates/uninstalls state that existing folders will move to the system Recycle Bin. Run skills sequentially, showing current skill and completed/total count. Prevent overlapping operations and path/settings changes during execution. Report each result and continue after isolated failures. No in-flight cancellation control in v1; confirmation can always be cancelled.

- Install: prepare and verify the complete copy, then place it at a previously absent destination without overwriting another folder.
- Update: prepare and verify the complete replacement, recycle the old folder, then place the new one. Destination-only files leave with the old folder. No merge.
- Uninstall: recycle the complete installed folder.

Temporary preparation belongs on the destination filesystem, outside enumerated skill directories. Only app-owned temporary copies may be cleaned automatically. Successful operations retain no app-managed backup.

Preparation failures leave the installation intact. A recycling failure aborts the affected operation without permanent deletion fallback. Placement failure after successful recycling is reported explicitly, with the original folder's path and instructions for manual recovery through Windows Recycle Bin. Retention and successful restoration are not guaranteed; there is no restore feature in the app. An interrupted operation may also require manual recovery; do not advertise atomic replacement.

Symbolic links and junctions produce warnings identifying locations and targets. Users may continue or cancel. Confirmed copies materialize pointed-to content as ordinary entries. Removal never traverses links to delete external targets. Broken links, cycles, inaccessible targets, or unsupported reparse points fail the affected copy with a clear error; do not silently omit entries.

Reject destructive source/destination overlap, including overlap revealed by link resolution. Prevent a copied link from recursively incorporating the destination or staging tree. Validate identities and targets in Rust. Recheck confirmed inputs before mutation; material changes invalidate the affected operation and require refresh/reconfirmation. External processes are not locked, so filesystem errors remain possible and must be reported.

## Acceptance and validation

Verify settings persistence and unavailable paths; each harness's configured destination; all four comparison states and supporting-file differences; complete replacement including obsolete files; installed-only removal; malformed metadata warnings; invalid source restrictions; link cancellation/materialization and external-target preservation; preparation/recycling/placement failures; overlap and stale confirmation safeguards; partial batch results; keyboard operation and both themes.

Backend tests use disposable temporary directories. Actual Recycle Bin, Windows junction, and desktop behavior require native Windows checks with disposable skills. Automated tests must not modify real user harness installations. Distinguish automated, browser, native desktop, and actual recycling evidence in phase reports.

## Evidence and defaults

- [Tauri frontend configuration](https://v2.tauri.app/start/frontend/) and [native dialogs](https://v2.tauri.app/plugin/dialog/).
- [Tailwind Vite integration](https://tailwindcss.com/docs/installation/using-vite).
- [Claude Code skills](https://code.claude.com/docs/en/skills), [OpenCode skills](https://opencode.ai/docs/skills/), and [Antigravity skills](https://antigravity.google/docs/skills).
- Codex installation convention verified in `C:/Users/tjmpl/.codex/skills/.system/skill-installer/SKILL.md` and its installer script; this machine's path is evidence, not a value to hardcode.
- [Rust system trash support](https://docs.rs/trash/latest/trash/).
- No application files, installation actions, or commits were created during design discovery.
