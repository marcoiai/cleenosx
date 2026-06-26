# Cleenosx

Cleenosx is a macOS-only storage cleanup app for understanding where SSD space went, especially the confusing "System Data" bucket. 
It scans, explains, classifies, lets the user select files or whole directories, and, remove them only after strong (3) confirmations.

The project has three entry points that share the same Rust domain logic:

- A Tauri desktop app with a React + Tailwind UI.
- A guided terminal CLI.
- A generated macOS Recovery helper script for manual review workflows.

## Objective

Is simple: help the user find what is taking space, drill into the biggest blocks, warn what is safer or riskier to remove, select whole files and/or directories, and, remove them only after strong (3) confirmations.

Implemented scan areas include:

- APFS volumes and mounted filesystems.
- `/System/Volumes/Data` large-block usage.
- AssetsV2 and known MobileAsset classes.
- Developer tool storage such as Xcode, simulators, Android SDK, Homebrew, Rust, and container tools.
- Local Time Machine snapshot listing.
- Risk classification and visible scan logs.

## Requirements

- macOS. This app is not intended to support Windows or Linux.
- Rust stable toolchain.
- Node.js and `pnpm`.
- Tauri 2 prerequisites for macOS development.

## Install

```sh
pnpm install
```

## Run The Desktop App

```sh
pnpm tauri:dev
```

For frontend-only development:

```sh
pnpm dev
```

## Run The CLI

```sh
cargo run -p cleanerx-cli
```

The CLI starts a guided menu and uses the same read-only scanners as the desktop app.

## Build

```sh
pnpm build
pnpm tauri:build
```

For Mac App Store preparation, see [docs/APP_STORE.md](docs/APP_STORE.md).

## Test And Check

```sh
cargo test
cargo check
pnpm build
```

## Project Layout

```text
crates/cleanerx-core/   Shared scanner, classifier, model, and recovery logic
crates/cleanerx-cli/    Guided terminal interface
src-tauri/              Tauri shell and command bridge
src/                    React desktop UI
docs/                   Product and engineering documentation
```

## Safety Model

CleanerX treats macOS storage cleanup as a high-risk operation.

- Scans are safe to run.
- Removals require explicitly selected files/directories and confirmations.
- Rust `target` directories are valid cleanup candidates because Cargo can rebuild them.
- Whole volumes, whole `AssetsV2`, broad system paths, projects, and user documents are not cleanup targets.
- SIP or `restricted` paths are marked as read-only/system risks.
- macOS command failures become logs and partial results instead of crashes.

## Documentation

- [Context](docs/CONTEXT.md) explains the product problem, users, goals, and MVP boundaries.
- [Architecture](docs/ARCHITECTURE.md) explains the workspace, data flow, scanners, safety rules, and extension points.
- [Mac App Store](docs/APP_STORE.md) tracks Store-specific signing, sandbox, and upload work.
