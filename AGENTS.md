# Agent instructions

## Reading order

1. `docs/PROJECT_CONTEXT.md` for the current application and evidence.
2. `docs/TODO.md` for the active phase and authorization boundary.
3. `docs/DECISIONS.md` for durable choices.
4. `docs/specs/2026-10-01-ai-skill-manager-design.md` and the corresponding plan in `docs/plans/` before implementation.

## Commands

Verified on Windows on October 1, 2026: Node 24.21.0, npm 12.2.0, Rust/Cargo 1.98.1.

```powershell
npm.cmd run check
npm.cmd run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

Use `npm.cmd ci` to restore locked dependencies. Use `npm.cmd run tauri dev` for desktop testing; native picker behavior still needs user validation. Vite uses port 1420 with strict port checking.

## Conventions and boundaries

- Keep the English interface and English project documents; conversational reports can follow the user's language.
- Prefer standard library functions and the smallest working change. Keep filesystem logic testable without Tauri UI.
- Automated filesystem tests use disposable paths and must never modify real harness installations.
- Distinguish automated, mocked browser, native startup, native interaction, and recycling evidence.
- Stop at each planned phase for user validation and explicit authorization. Phase 2 is not authorized.
- Commit/push, packaging, signing, publishing, and production changes require explicit authorization for their scope.
- Keep Cargo/npm lockfiles versioned. Keep generated output and local configuration out of Git.

## Context maintenance

Update current work in `docs/TODO.md`, product facts in `docs/PROJECT_CONTEXT.md`, and durable choices in `docs/DECISIONS.md`. Preserve binding design/plan contracts and approval evidence; avoid duplicating them.
