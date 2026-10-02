# Ubuntu Support — Phased Implementation Plan

Status: Approved in chat on October 2, 2026; Phase 1 explicitly authorized. Source of truth: [design](../specs/2026-10-02-ubuntu-support-design.md). Stop after every phase for user validation and explicit authorization of the next. Commit/push and release uploads are separate approvals.

## Phase 1 — Compatibility and automatic CI

1. Recheck checkout/instructions/status. Reuse destination validation in `src-tauri/src/lib.rs`; preserve Windows Explorer and add Linux `xdg-open` with one absolute argument, no shell, and spawn/exit error reporting. Test with a disposable executable.
2. In settings, choose the OS-specific home environment variable and retain overrides. In the shared manager fingerprint flow, include regular-file Linux `0o777` permissions in comparison, stage verification, and mutation baselines, including unreadable-file fallback. Preserve stdlib copies and literal Unix relative filenames. No schema change or application dependency.
3. Change UI/backend recovery wording to system trash without altering recycling safeguards. Add disposable Linux regressions for file permissions, file symlinks, literal filenames, case-sensitive identities, permission-only staging differences, and stale scan/confirmation rejection. Reuse existing cycle/overlap/external-target/failure tests.
4. Add `.github/workflows/ci.yml` for main pushes, PRs, and dispatch: Windows 2022/Ubuntu 22.04, Node 24/Rust stable, caches, read-only permissions, cancellation. Install Ubuntu development prerequisites and xdg-utils. Run npm ci/check/build, cargo fmt/clippy/tests with locked dependencies, and optimized Tauri without bundling.
5. Update current status, product facts, and durable choices. Review the diff and run all six prescribed local checks plus optimized no-bundle build. Validate workflow structure. Record actual Actions results only after an authorized push; do not push just to validate CI.

Gate: Report local Windows evidence, unexecuted Linux tests/CI where applicable, and any blockers. User checks the Windows app using disposable folders and accepts the phase. Phase 2 requires explicit authorization.

## Phase 2 — Manual packages

Add a dispatch-only packaging workflow. Build the selected event commit on the same OS baselines after Phase 1 checks. Fail version disagreement, use explicit nsis/deb targets, retain version/identifier, add xdg-utils to deb runtime dependencies, and inspect deb metadata/contents. Upload separate Windows/Ubuntu artifacts with package, SHA256SUMS.txt, and commit identity; retain 14 days. Never create tags/releases or replace release assets.

Gate: Both packages build in actual Actions runs and downloaded checksums verify. Stop for acceptance and explicit Phase 3 authorization. Requires an authorized push before remote execution.

## Phase 3 — Native Ubuntu validation and documentation

Install the generated deb in disposable Ubuntu 22.04 and 24.04 desktop environments. Verify launcher/icon, picker, folder opening, persistence, built-in/custom harnesses, keyboard/themes, full Install/Update/Uninstall, executable scripts, symlinks/external targets, and manual trash restoration. Remove the package and confirm settings/skills remain. Update README/context/status/decisions in English and correct historical draft-release wording with current evidence. Report only validated configurations; no invented native evidence.

Gate: CI and both package builds pass, downloaded checksums verify, and Ubuntu desktop/package/trash acceptance is recorded. Missing native environments are reported as pending, not replaced with mocked checks. Commit/push and publication remain separately gated.
