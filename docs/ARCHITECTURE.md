# cleenosx Architecture

## Overview

cleenosx is a MealWare macOS-only app. It is built around a small Rust library that wraps macOS storage tools such as `du`, `df`, `diskutil`, `tmutil`, and `ls`. The library turns raw command output into structured storage facts, risk labels, and cleanup candidates that the desktop app and CLI can present safely.

The desktop app is a Tauri shell with a React frontend. Rust owns domain logic and macOS system interaction. React owns presentation and calls Tauri commands through a small typed bridge.

```text
React UI -> src/tauri.ts -> Tauri commands -> cleanerx-core -> macOS commands/filesystem
CLI --------------------------------------^
```

The important architectural choice is that scanning, parsing, classification, cleanup planning, and recovery script generation live in `cleanerx-core`, so the desktop app and CLI use the same macOS-specific behavior.

## Workspace

```text
Cargo.toml
crates/
  cleanerx-core/
  cleanerx-cli/
src-tauri/
src/
```

### `crates/cleanerx-core`

Shared Rust library for:

- Data models in `models.rs`.
- macOS command execution in `command.rs`.
- Scanners in `scanners.rs`.
- Path classification in `classify.rs`.
- Cleanup plan validation and future removal execution.
- Recovery helper generation in `recovery.rs`.
- Public API re-exports and orchestration in `lib.rs`.

This crate has no UI responsibilities.

### `crates/cleanerx-cli`

Guided terminal UI for safe-mode storage investigation. It calls `cleanerx-core` directly and renders menus, logs, findings, volumes, and recovery script output.

### `src-tauri`

Tauri app shell. `src-tauri/src/lib.rs` exposes command handlers such as:

- `scan_overview`
- `scan_volumes`
- `scan_data_usage`
- `scan_assets_v2`
- `scan_developer_tools`
- `scan_rust_artifacts`
- `scan_containers`
- `list_snapshots`
- `thin_snapshots`
- `generate_recovery_script`
- `cleanup_selected_items`

Each command delegates to `cleanerx-core`.

### `src`

React + TypeScript frontend. Key files:

- `src/App.tsx`: navigation, scan orchestration, state, logs, and view composition.
- `src/tauri.ts`: typed wrappers around Tauri `invoke`.
- `src/types.ts`: TypeScript mirror of Rust models.
- `src/components/`: dashboard panels, findings, logs, usage tree, volumes, review, settings, and recovery panels.

## Domain Model

Core data moves through `ScanResult<T>`, which contains:

- `data`: the scan result payload.
- `logs`: timestamped `info`, `warning`, or `error` scan logs.

Primary payloads:

- `Overview`: storage summary, volumes, usage roots, and findings.
- `VolumeInfo`: APFS/filesystem metadata, capacity, mount state, risk, flags, and notes.
- `UsageNode`: measured path, bytes, category, risk, flags, and children.
- `Finding`: title, path, optional size, category, risk, reason, recommended action, and destructive flag.
- `CleanupPlan` and `CleanupOutcome`: cleanup API shape. Current implementation is dry-run only while removal is being implemented.

## Scanner Flow

`scan_overview()` composes multiple narrower scans:

1. `scan_volumes()`
2. `scan_data_usage()`
3. `scan_assets_v2()`
4. `list_snapshots()`
5. Volume summary warnings

The UI can also call individual scans for more focused views.

## macOS Tool Layer

The core library is intentionally small, macOS-specific, and command-driven. It should prefer stable macOS tools before adding heavier dependencies. There is no cross-platform abstraction layer. Commands go through `command::run()` so failures can become logs instead of panics.

- `df -k` for mounted filesystem capacity.
- `diskutil apfs list` for APFS metadata.
- `diskutil list internal` for internal disk hints.
- `tmutil listlocalsnapshots /` for local snapshots.
- `du` for measured path sizes.
- `ls -ldOe` for flags such as `restricted`.

Command failures are converted into warning logs. The app should prefer partial results over crashing or hiding the problem.

## App Sections

### Volumes

The Volumes section lists mounted filesystems and APFS volumes with:

- Name, identifier, role, mount point, and mounted state.
- Capacity, used bytes, available bytes, and percent used.
- Encryption, locked state, APFS role hints, and filesystem flags.
- Risk and notes explaining whether the volume is safe to inspect.

This section is investigative only. It helps the user understand APFS layout and find where space is being counted before they move into cleanup.

### Clear

The Clear section is the MVP cleanup workspace. It should be a drilldown UI, not a flat delete list.

Expected flow:

1. Start from categories such as AssetsV2, simulators, Rust artifacts, Node caches, Homebrew, containers, snapshots, and large user cache folders.
2. Drill down from category to path groups, then to individual removable items.
3. Show size, path, last modified time when available, category, risk, reason, and recommended action.
4. Let the user select individual items or safe groups.
5. Build a `CleanupPlan` from selected items.
6. Validate the plan in Rust before any destructive operation.
7. Ask for confirmation through a deliberately varied confirmation challenge.
8. Execute only exact validated targets and stream logs/progress back to the UI.

The UI should make it easy to remove specific caches/build artifacts while making broad deletion inconvenient or impossible.

Rust project `target` directories are first-class cleanup candidates. They are build artifacts that Cargo can recreate, so cleenosx may mark them as `attention` rather than `dangerous`, even when they live under a project folder. The surrounding project/source directory remains protected and should not be removable as a broad target.

## Classification

`classify_path()` maps paths and flags to:

- `StorageCategory`
- `RiskLevel`
- Explanation text

Current categories include APFS/system storage, AssetsV2, developer tools, Rust artifacts, Node caches, Homebrew, containers, simulators, projects, user data, caches, updates, snapshots, extra volumes, and unknown.

Risk levels are:

- `safeToAnalyze`
- `attention`
- `reviewRequired`
- `dangerous`
- `readOnlySystem`

Classification is conservative. Known cache/build areas may be marked as attention, but source folders, broad paths, and restricted system areas are not treated as cleanup-safe.

## Safety Boundaries

The current code is still read-only, but the MVP product target includes explicit file/directory removal from the Clear section:

- `cleanup_selected_items()` currently returns a dry-run outcome and should become the validated removal path.
- `thin_snapshots()` currently returns a dry-run outcome and can stay separate from normal file/directory cleanup.
- Generated recovery script includes review flows and should remain conservative.

Before real cleanup is implemented, the architecture should require:

- Exact path targeting.
- Category and risk display.
- Estimated bytes.
- Clear reason and recommended action.
- Reversibility notes.
- Strong confirmation that changes from time to time.
- Tests for refusal of broad or dangerous targets.

## Confirmation UX

Cleanup confirmation should not become muscle memory. The app should vary the challenge so users stay attentive before destructive actions.

Possible confirmation patterns:

- Type a changing phrase such as `DELETE 4 ITEMS`, `CLEAR 2.4 GB`, or the final path segment.
- Move the destructive button to a different screen area for each operation.
- Change button color and layout between confirmation sessions while preserving accessibility contrast.
- Ask the user to select the matching target path from a small list.
- Require holding a button for a short countdown on high-risk actions.
- Show a final diff-style plan summary and require checking a statement that names the consequence.

The confirmation system should be creative, but never deceptive. It must remain accessible, keyboard usable, screen-reader legible, and clear about what will be removed.

Rust remains the final authority. The frontend can make confirmation harder to do accidentally, but `cleanerx-core` must still reject dangerous plans regardless of UI state.

## Frontend Data Flow

`App.tsx` owns:

- Active view.
- Latest overview, volumes, usage nodes, findings, logs.
- Loading and error state.
- Category filtering for usage nodes.

Scan buttons call typed functions from `src/tauri.ts`. Results update React state and append logs. Errors are rendered as UI messages and log entries.

The Clear section should keep selection state in React, but treat it as provisional. The selected items become a `CleanupPlan`, and the Rust core validates that plan before execution.

## Adding A New Scanner Or Cleaner

1. Add or extend models in `crates/cleanerx-core/src/models.rs`.
2. Implement the scanner in `crates/cleanerx-core/src/scanners.rs`.
3. Add classification rules in `classify.rs` if new paths/categories are involved.
4. If cleanup is involved, add plan validation and exact-target refusal tests before adding execution.
5. Expose the function in `crates/cleanerx-core/src/lib.rs`.
6. Add a Tauri command in `src-tauri/src/lib.rs`.
7. Add a typed wrapper in `src/tauri.ts`.
8. Add or update UI components in `src/`.
9. Add tests for parsers/classification/cleanup refusal and run `cargo test`, `cargo check`, and `pnpm build`.

## Development Commands

```sh
pnpm install
pnpm tauri:dev
cargo run -p cleanerx-cli
cargo test
cargo check
pnpm build
```

## Existing Design Note

The original MVP design note is in `docs/superpowers/specs/2026-06-19-macos-storage-mvp-design.md`. This architecture document reflects the current repository structure and should be kept in sync as implementation evolves.
