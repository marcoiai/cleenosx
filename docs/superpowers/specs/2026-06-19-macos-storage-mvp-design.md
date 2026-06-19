# macOS Storage Cleaner MVP Design

## Goal

Build a macOS product that helps technical and semi-technical users understand confusing storage usage, especially macOS "System Data", without performing destructive cleanup in the MVP.

The product has two interfaces sharing the same Rust classification logic:

- A Tauri desktop app with React, TailwindCSS, and Material Design-inspired interaction patterns.
- A guided CLI/recovery companion for normal macOS and macOS Recovery workflows.

## MVP Scope

The first MVP is read-only by default and includes:

- Tauri app opens correctly.
- React dashboard, volumes view, scanner view, review placeholder, logs, and `Settings`.
- Read-only APFS/disk/volume scan.
- Read-only usage scan for `/System/Volumes/Data` when available.
- Basic large-block classification.
- AssetsV2 detection.
- `restricted` flag detection through filesystem metadata.
- Recovery script generation.
- Guided CLI with menus, color, emoji labels, spinners, elapsed-time feedback, logs, and safe-mode defaults.
- No automatic destructive cleanup.

Cleanup actions are modeled but disabled or dry-run only in the MVP. Destructive actions will be implemented later with strong confirmation such as typing `DELETE`.

## Architecture

### Rust Workspace

Rust owns the domain logic and system interactions.

- `crates/cleanerx-core`: shared models, command execution, parsing, classifiers, recommendations, scan orchestration, and recovery script generation.
- `crates/cleanerx-cli`: guided terminal UI using the core crate.
- `src-tauri`: Tauri app shell and commands that call the core crate.

### React UI

React is presentation only. It calls typed Tauri commands and renders:

- Dashboard with storage summary and warnings.
- Volumes/partitions table.
- Directory tree for large blocks.
- Recommendation cards with risk chips.
- Review screen placeholder for future cleanup.
- Visible logs.
- Settings placeholder: `Settings will be added later.`

TailwindCSS implements layout and design tokens. Material Design concepts are applied through clear surfaces, chips, progress indicators, navigation, state feedback, and accessible controls.

### CLI / Recovery

The CLI uses the Rust core where possible and provides a guided flow:

- Detect normal macOS vs Recovery-like environment.
- List internal disks and APFS volumes.
- Show locked/encrypted/FileVault hints when detectable.
- Help identify and mount a Data volume.
- Measure large blocks and known AssetsV2 targets.
- Generate or print safe Recovery instructions.
- Never remove whole APFS volumes, whole `AssetsV2`, whole `/Library`, whole `/opt`, source code, or user documents.

## Domain Model

Core types:

- `VolumeInfo`: name, identifier, role, mount point, mounted state, encryption/locked hints, capacity, available bytes, flags, risk.
- `UsageNode`: path, size, category, risk, flags, children.
- `Finding`: title, path, size, category, risk, reason, recommended action, destructive flag.
- `ScanLog`: timestamp, level, message.
- `RiskLevel`: safe to analyze, attention, review required, dangerous, read-only/system.
- `CleanupPlan`: future structure for selected actions; MVP supports dry-run/review only.

## Scanner Strategy

The app uses macOS commands through Rust wrappers:

- `df -k`
- `diskutil list internal`
- `diskutil apfs list`
- `diskutil info`
- `tmutil listlocalsnapshots /`
- `du -xkd 1 <path>`
- `ls -ldOe <path>`

Command failures are reported as logs and partial scan results instead of crashing the UI.

## Classification Rules

Risk defaults are conservative:

- Read-only/system: APFS volumes, `/System`, whole `/Library`, whole `/opt`, whole `AssetsV2`, locked volumes, other OS partitions.
- Safe to analyze: `/System/Volumes/Data`, `/Library`, `/opt`, `~/Library`, `~/Projects`.
- Attention/review required: caches, developer build artifacts, simulator runtimes, container images/machines, Time Machine snapshots, update assets.
- Dangerous: broad deletes, source folders, documents, whole volumes, SIP/restricted paths under normal boot.

Known categories:

- macOS/APFS
- AssetsV2
- developer tools
- Rust artifacts
- Node package caches
- Homebrew
- containers
- simulators
- projects
- unknown/investigate

## Safety Rules

- Read-only scans do not require confirmation.
- MVP cleanup commands do not delete anything.
- Any future destructive action must show target path, estimated size, reason, risk, reversibility, and require explicit confirmation.
- If a target has `restricted`, normal boot must not force deletion. The app explains that Recovery is required.
- Recovery helper never deletes `AssetsV2` as a whole.

## Error Handling

- Missing commands, permission denial, SIP restrictions, unmounted volumes, and locked FileVault volumes become visible findings/logs.
- Scans return partial data plus warnings when a command fails.
- Long-running scans expose loading state and elapsed time.

## Validation

MVP validation:

- `cargo check`
- `cargo test` for core classifiers/parsers where practical
- `pnpm build`
- Tauri dev command starts the desktop app
- CLI can show menus and run read-only scans

## Out Of Scope For MVP

- Destructive cleanup.
- Privilege escalation helpers.
- Background daemon.
- Persistent settings beyond the placeholder.
- App signing/notarization.
- Cross-platform support.
