# Harness Management Design

Status: Approved in conversation on October 2, 2026. This document records the approved conversational design and decisions. Implementation requires separate phase authorization.

## Context and goals

The application currently displays four fixed tabs. Harness identity is a Rust enum and a TypeScript union; settings persist source, destinations, and theme. Scanning, confirmation, and execution resolve destinations from saved settings and enforce stale-input, overlap, link, and recycling safeguards.

Show only locally configured built-in harnesses, support persistent custom harness registrations, and open the inspected harness's skills folder in Windows Explorer. Keep the English interface and existing whole-folder management behavior.

## Approved behavior

- Built-ins remain Codex, Claude Code, Antigravity IDE, and OpenCode, in that order. Show a built-in when its default configuration directory exists or its explicitly overridden destination exists as a directory. Configuration-directory detection is a proxy for local installation, not executable detection.
- Resolve configuration directories using the existing environment-aware defaults: Codex uses CODEX_HOME or the home .codex directory; Claude uses home .claude; Antigravity uses home .gemini/config; OpenCode uses XDG_CONFIG_HOME or home .config, followed by opencode. A missing skills directory does not hide a harness whose configuration directory exists.
- Recheck visibility at startup and explicit Refresh, and after destination changes. Never create directories during discovery. If the active harness disappears, select the first visible harness; if none remain, clear comparison and show an empty state. Keep Refresh and, once implemented, Add harness available in that state.
- Add harness opens an accessible dialog with a name field and native skills-folder picker. Add requires a nonempty trimmed name and an accessible existing directory. Cancelling the dialog or picker does not save a registration. Persist only after successful validation and settings write; select the new harness afterward.
- Custom registrations have stable identities independent of names and paths. Allow renaming, changing the destination through the existing control, and removing the registration. Confirm removal with explicit wording that folders and skills will remain untouched. Built-ins cannot be renamed or removed.
- Keep registered custom harnesses visible when their destination becomes unavailable. Show an error/warning and allow changing the destination or removing the registration. Custom harnesses use the same comparison and Install/Update/Uninstall workflow as built-ins.
- Open folder appears beside the active managed destination and requests that exact configured folder from Rust. Open it in Explorer when available; report missing/inaccessible paths or launch failures without creating folders or opening an unrelated ancestor.
- Preserve independent selections, keyboard tab navigation, themes, shared-destination warnings, and the OpenCode-specific discovery notice. Duplicate destination paths are allowed and retain the existing shared-path warning.
- Disable conflicting harness, picker, and Explorer controls during scans, settings writes, preparation, and execution, following current UI conventions. No pending confirmation survives a settings change.

## Approach and compatibility

Rust returns harness descriptors containing stable identity, label, built-in/custom classification, visibility, and destination availability. The UI renders the returned registry instead of a hardcoded list. Keep built-in serialized identifiers unchanged; extend settings additively with custom registration metadata and default it to empty when loading older files.

Use string identities across IPC and backend-held scan/operation plans so every harness follows the existing shared filesystem flow. Rust must reject unknown identities even when a client supplies a destination entry. Preserve all existing saved source, theme, and destination overrides; malformed settings remain protected against overwrite.

Reuse save_settings and its operation guard, safe replacement writes, and plan invalidation for registry changes. Add one narrow Explorer command taking a harness identity, resolving the path from settings, checking that it is an existing directory, and launching Explorer through a standard-library process argument, without a shell or JavaScript shell permission. No new runtime dependency is needed.

Configuration detection was chosen over skills-directory-only detection because a configured harness can have no skills yet. Executable detection was rejected because it needs harness-specific installation rules and does not identify the managed skills location. Keep one saved registry rather than introduce plugin infrastructure or a database.

## Failure handling and boundaries

Failed validation or persistence leaves the visible registry and saved settings unchanged. Missing saved custom destinations retain their registrations. Removing a custom registration removes only its metadata and destination mapping, clears its selections, and invalidates confirmations; it performs no filesystem deletion or recycling.

Allow duplicate display names: stable IDs distinguish registrations and paths remain visible. Treat names as ordinary text, never HTML. Missing or unreadable destination behavior remains governed by the existing scan/operation safeguards. Explorer opening provides no mutation authority to the client.

No executable installation, remote discovery, new skill formats, project-local discovery, automatic synchronization, packaging, signing, publication, commit, or push is included.

## Acceptance and validation

Use disposable directories to verify built-in detection with and without skills folders, existing overrides, absent configurations, no discovery side effects, and Refresh-driven disappearance/reappearance. Mock the backend to verify filtered keyboard navigation, fallback selection, and the empty state.

Verify old-settings compatibility, custom add/rename/remove persistence, cancelled pickers, invalid names/folders, failed writes, independent selections, shared destinations, and unavailable saved custom paths. Exercise custom scan and confirmed install/update/uninstall through the existing backend test seams; prove changed or removed registrations reject stale plans and removal preserves every directory and file.

Test Explorer path resolution and refusal using disposable paths without launching real Explorer in unit tests. Mocked browser checks cover request arguments, disabled controls, and errors. Native user testing covers the actual folder picker, Explorer destination, restart persistence, and disposable skill operations. Keep automated, mocked browser, and native evidence separate.

## Delivery

Phase 1 delivers built-in visibility and Open folder. Phase 2 delivers custom registration management and extends identity handling. Each phase ends with user testing and an explicit authorization gate. Do not add Phase 2 placeholders in Phase 1.
