# cleenosx Context

## Product Intent

cleenosx is a MealWare macOS app. It helps Mac users understand and recover SSD space without blindly deleting files. Clearing disk space on macOS can be hard to do alone because useful files, system-managed storage, caches, snapshots, app containers, and build artifacts are mixed together and often hidden behind vague labels like "System Data".

The app focuses on storage that is hard to reason about from Finder or System Settings, especially APFS volumes, macOS "System Data", `AssetsV2`, simulator runtimes, local snapshots, developer caches, containers, and large user folders.

The product should feel like a careful storage investigator, not a one-click cleaner. Its first job is to make hidden disk usage visible, identify the biggest bottlenecks, explain removal risk, and guide the user toward safe next steps.

## Target Users

- Developers with large Xcode, simulator, Rust, Node, Homebrew, Docker, OrbStack, or Android SDK footprints.
- Technical Mac users trying to understand why a disk is nearly full.
- Users comfortable reviewing paths and terminal output, but who still need guardrails.
- Future support scenarios where a guided macOS Recovery script helps inspect a mounted Data volume.

## Problem Space

macOS storage accounting is opaque. A Mac may show large "System Data" usage even when ordinary user folders look small. Common causes include:

- APFS volume accounting and snapshots.
- AssetsV2 MobileAsset downloads.
- Xcode and simulator runtimes.
- Package manager caches and build artifacts.
- Containers and VM disk images.
- Files on `/System/Volumes/Data` that are not obvious from the Finder view.
- Permission or SIP restrictions that make normal scans incomplete.

cleenosx exists to make these causes explicit, rank the biggest blocks, and label them with conservative risk so the user can decide what is safe to remove.

## Core User Jobs

- Find what is taking the most space.
- Drill into big directories until the real bottleneck is visible.
- Understand whether a file or directory is likely safe, risky, or dangerous to remove.
- Select whole files or whole directories for removal when the target is clear.
- Treat Rust `target` directories as good cleanup candidates because Cargo can rebuild them.
- Avoid accidental deletion of system files, source code, documents, and broad parent directories.
- See logs and warnings when macOS permissions or locked volumes make a scan incomplete.

## MVP Definition

The MVP is the core storage cleanup loop:

1. Show the user where disk space is going.
2. Highlight the biggest files and directories.
3. Let the user drill down into large blocks.
4. Warn whether each target looks safer or riskier to remove.
5. Let the user select whole files or whole directories.
6. Confirm removal strongly before deleting anything.
7. Show progress, results, and logs.

Everything else is secondary until this loop works well.

## MVP Goals

- Provide a Tauri desktop dashboard for storage overview, volumes, scanner results, findings, review, recovery, and logs.
- Provide a guided CLI using the same Rust core logic.
- Scan real macOS storage metadata through read-only system commands.
- Provide a dedicated large-block scanner screen so users can quickly find storage bottlenecks.
- Classify large paths by category and risk.
- Warn the user which files or directories are more likely safe, risky, or unsafe to remove.
- Remove explicitly selected whole files or whole directories after strong confirmation.
- Detect `AssetsV2` and known MobileAsset classes.
- List local Time Machine snapshots.
- Generate a safe-mode macOS Recovery helper script.
- Preserve visible logs for partial failures and permission issues.

## Non-Goals For The MVP

- No automatic deletion without explicit user selection and confirmation.
- No privileged helper or escalation flow.
- No background daemon.
- No signed/notarized distribution pipeline.
- No Windows or Linux support. cleenosx should use macOS/APFS-specific behavior directly.
- No persistent settings beyond placeholder UI.

## Safety Principles

cleenosx should bias toward preserving data when uncertain.

- Read-only scans can run without confirmation.
- Unknown paths are inspect-first.
- Whole files or whole directories may be removable only when selected explicitly.
- Rust project `target` directories can be removed after confirmation; source directories around them must still be protected.
- Project/source folders and user documents are dangerous cleanup targets.
- Whole APFS volumes, whole `AssetsV2`, whole `/Library`, whole `/opt`, and broad system paths must never be removed automatically.
- SIP or `restricted` paths should be explained rather than forced from normal boot.
- Future destructive workflows must show the exact target, estimated size, category, reason, risk, reversibility, and require strong confirmation.

## Product Vocabulary

- **Overview:** A combined scan summary with storage totals, volumes, usage roots, findings, and logs.
- **Volume:** A mounted filesystem or APFS volume with role, capacity, availability, flags, and notes.
- **Usage node:** A measured path with size, category, risk, flags, and optional children.
- **Finding:** A human-readable recommendation or warning about a path, snapshot, or storage area.
- **Bottleneck:** A file or directory that accounts for a large amount of disk usage and deserves user attention.
- **Clear target:** A whole file or whole directory selected by the user for possible removal.
- **Risk:** The safety label for investigation or cleanup decisions.
- **Recovery helper:** A generated Bash script for guided inspection from macOS Recovery.

## Current Product State

The codebase already implements the read-only scanner core, Tauri command bridge, React UI, CLI, and recovery script generation. Cleanup and snapshot-thinning functions exist as API placeholders but intentionally return dry-run messages.
