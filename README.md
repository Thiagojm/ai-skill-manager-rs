# ⚡ AI Skill Manager

[![Platform: Windows x64](https://img.shields.io/badge/Platform-Windows%20x64-0078D6?logo=windows&logoColor=white)](https://github.com/Thiagojm/ai-skill-manager-rs)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.98+-black?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Latest Release](https://img.shields.io/github/v/release/Thiagojm/ai-skill-manager-rs?label=Release)](https://github.com/Thiagojm/ai-skill-manager-rs/releases)

A fast, reliable, and user-friendly Windows desktop application to discover, compare, install, update, and manage local AI skills across your coding assistants.

Managing skills across multiple AI tools usually means manual folder copying, guessing which version is installed where, and risking accidental overwrites. **AI Skill Manager** solves this with a centralized, visual dashboard designed for seamless multi-agent skill maintenance.

---

## ✨ Features

- 🎯 **Multi-Agent Central Hub**: Dedicated tabs for **OpenAI Codex**, **Anthropic Claude Code**, **Google Antigravity IDE**, and **OpenCode**, each with independent destination folder configuration.
- 🔍 **Deep Whole-Tree Comparison**: Compares full directory trees — including nested files, helper scripts, configuration assets, empty directories, and hidden resources.
- 🛡️ **Safe & Non-Destructive Operations**:
  - **Atomic Staging**: Prepares and validates replacements before any modification takes place.
  - **Recycle Bin Integration**: Uninstalled or replaced folders are safely moved to the Windows Recycle Bin via native Windows Shell operations (`IFileOperation`), never permanently deleted.
  - **Link Protection**: Safeguards directory junctions and symbolic links without mutating external targets.
- ⚡ **High Performance Scanning**: Multi-threaded parallel inventory workers and in-memory source snapshots make tab switching and folder scanning instantaneous.
- 🎨 **Modern & Accessible UI**: Clean dark and light modes, instant search by skill title or directory ID, status filtering, and keyboard navigation.

---

## 🤖 Supported AI Agents

AI Skill Manager automatically resolves standard skill installation paths for popular coding assistants:

| Agent / Harness | Default Managed Location | Custom Override |
| :--- | :--- | :---: |
| **OpenAI Codex** | `$CODEX_HOME/skills` *(or `~/.codex/skills`)* | ✅ Yes |
| **Anthropic Claude Code** | `~/.claude/skills` | ✅ Yes |
| **Google Antigravity IDE** | `~/.gemini/config/skills` | ✅ Yes |
| **OpenCode** | `~/.config/opencode/skills` *(respects `XDG_CONFIG_HOME`)* | ✅ Yes |

> [!NOTE]
> Custom destination paths and chosen themes are automatically saved and restored on subsequent launches.

---

## 🔄 Understanding Skill Statuses

When scanning your source folder against an agent's destination, each skill is classified with an actionable status:

| Status | Meaning | Available Actions |
| :--- | :--- | :--- |
| 🟡 **Missing** | Present in source, not yet installed in the agent destination. | **Install** |
| 🟢 **Identical** | Fully synchronized; complete trees and file contents match. | **Uninstall** |
| 🔵 **Different** | Exists in both locations, but contents or structures differ. | **Update** (Clean Replacement), **Uninstall** |
| ⚪ **Installed only** | Exists only in the agent destination (absent from source). | **Uninstall** |

---

## 🚀 Getting Started

### Option 1: Pre-built Windows Installer (Recommended)

1. Download the latest installer (`AI Skill Manager_x.x.x_x64-setup.exe`) from the [Releases](https://github.com/Thiagojm/ai-skill-manager-rs/releases) page.
2. Run the installer and follow the setup wizard.
3. Launch **AI Skill Manager**, choose your skills source folder, and start managing!

### Option 2: Running from Source

#### Prerequisites
- **Node.js**: `24.x` or newer (with `npm 12+`)
- **Rust**: `1.98.x` or newer (with Cargo)
- **Operating System**: Windows 10/11 x64

#### Quick Start
```powershell
# Clone the repository
git clone https://github.com/Thiagojm/ai-skill-manager-rs.git
cd ai-skill-manager-rs

# Install frontend dependencies
npm.cmd ci

# Start the desktop application in development mode
npm.cmd run tauri dev
```

---

## 🛠️ Development & Quality Assurance

### Code Checks & Testing
Run the complete automated quality suite before submitting changes:

```powershell
# Frontend linting and type-checking
npm.cmd run check
npm.cmd run build

# Rust formatting, lints, and automated unit/integration tests
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

### Manual Testing with Disposable Paths
For testing filesystem operations safely:
1. Create a temporary source directory and temporary destination folders for each harness.
2. Populate the source directory with sample skill folders containing a valid `SKILL.md`.
3. Verify folder selection, search, status filtering, and theme switching.
4. Test **Install** on a missing skill, **Update** on a modified skill (confirming obsolete files are cleaned up), and **Uninstall** (confirming items move safely to the Windows Recycle Bin).

---

## 📄 License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
