# Durable decisions

The following contracts were approved on October 1, 2026 in the [design specification](specs/2026-10-01-ai-skill-manager-design.md). The specification remains authoritative for detailed safeguards.

## Local desktop stack — accepted

Use Tauri 2/Rust with Svelte/TypeScript/Vite/Tailwind. Rust owns filesystem behavior; the frontend displays results and requests narrow commands. Settings use a local JSON file, without a database or service. This keeps the local workflow small and independently testable.

## Whole-folder identity and operations — accepted

Folder name identifies a skill, case-insensitively on Windows. Metadata names do not merge folders. Compare the complete tree and use its contents, not timestamps. Install copies the whole folder using verified staging. Update will replace it completely, including removing obsolete destination-only files, in Phase 3.

## Recoverable removal and links — accepted, pending implementation

Updates and uninstallations send the old complete folder to Windows Recycle Bin. Recycling failure has no permanent-delete fallback. Links/junctions warn and allow the user to continue or cancel; confirmed copies materialize their content, while removal must preserve external targets. The detailed overlap, stale-input, staging, and recovery contracts are in the design and Phase 2/3 plan.

## Preferences and discovery — accepted

Remember the last successfully selected accessible source automatically and keep `Choose folder…` available. There is no Save-as-default button. Persist the last theme, with dark as the initial default. Manage only each tab's configured destination; show shared-path/discovery notices without claiming harness isolation.

## Phased delivery — accepted

Deliver read-only comparison, then installation, then replacement/removal. Each phase stops for user validation and explicit authorization. Acceptance or commit/push of Phase 1 does not authorize later phases or release actions.
