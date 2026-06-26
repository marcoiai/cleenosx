use crate::classify::{action_profile_for_finding, classify_path, finding_for_path};
use crate::cleanup::{cleanup_candidate_id, register_usage_nodes};
use crate::command::{log_command_error, run, run_partial_with_timeout_and_cancel};
use crate::models::{
    DeepScanResult, DeepScanWarningsSummary, Finding, Overview, RiskLevel, ScanLog, ScanResult,
    StorageCategory, StorageSummary, UsageKind, UsageNode, VolumeInfo,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const DATA_VOLUME_PATH: &str = "/System/Volumes/Data";
const ASSETS_V2_PATHS: &[&str] = &[
    "/System/Volumes/Data/System/Library/AssetsV2",
    "/System/Library/AssetsV2",
];
const ASSETS_V2_TARGETS: &[&str] = &[
    "com_apple_MobileAsset_iOSSimulatorRuntime",
    "com_apple_MobileAsset_xrOSSimulatorRuntime",
    "com_apple_MobileAsset_watchOSSimulatorRuntime",
    "com_apple_MobileAsset_appleTVOSSimulatorRuntime",
    "com_apple_MobileAsset_MacSoftwareUpdate",
    "com_apple_MobileAsset_LinguisticData",
    "com_apple_MobileAsset_UAF_Siri_Understanding",
];
const RUST_TARGET_SCAN_ROOT_HINTS: &[&str] = &[
    "code",
    "Projects",
    "src",
    "project",
    "projects",
    "workspace",
    "work",
    "repos",
    "repo",
    "repository",
    "repositories",
    "Development",
    "Documents",
];
const RUST_TARGET_SCAN_MAX_DEPTH: usize = 8;
const DEEP_SCAN_TIMEOUT: Duration = Duration::from_secs(120);
static DEEP_SCAN_RUNNING: AtomicBool = AtomicBool::new(false);
static DEEP_SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

pub fn scan_storage_overview() -> ScanResult<Overview> {
    let mut logs = vec![ScanLog::info("Starting lightweight storage overview scan.")];

    let volume_scan = scan_volumes();
    logs.extend(volume_scan.logs);

    let summary = summarize_volumes(&volume_scan.data);
    let findings = summary_warnings(&volume_scan.data);

    ScanResult {
        data: Overview {
            summary,
            volumes: volume_scan.data,
            usage_roots: Vec::new(),
            findings,
        },
        logs,
    }
}

pub fn scan_volumes() -> ScanResult<Vec<VolumeInfo>> {
    let mut logs = vec![ScanLog::info(
        "Scanning APFS volumes and mounted filesystems.",
    )];
    let mut volumes = parse_df_volumes(&mut logs);
    merge_mount_metadata(&mut volumes, &mut logs);

    match run("diskutil", &["apfs", "list"]) {
        Ok(output) => merge_apfs_metadata(&mut volumes, &output.stdout),
        Err(error) => logs.push(log_command_error("diskutil apfs list failed", &error)),
    }

    match run("diskutil", &["list", "internal"]) {
        Ok(output) => merge_diskutil_list_metadata(&mut volumes, &output.stdout),
        Err(error) => logs.push(log_command_error("diskutil list internal failed", &error)),
    }

    if volumes.is_empty() {
        logs.push(ScanLog::warning(
            "No mounted volumes were parsed. The app can still run, but disk data is partial.",
        ));
    }

    ScanResult {
        data: volumes,
        logs,
    }
}

fn merge_mount_metadata(volumes: &mut Vec<VolumeInfo>, logs: &mut Vec<ScanLog>) {
    let output = match run("mount", &[]) {
        Ok(output) => output.stdout,
        Err(error) => {
            logs.push(log_command_error("mount failed", &error));
            return;
        }
    };

    for line in output.lines() {
        let Some((identifier, rest)) = line.split_once(" on ") else {
            continue;
        };
        let Some((mount_point, options)) = rest.rsplit_once(" (") else {
            continue;
        };
        let flags = options
            .trim_end_matches(')')
            .split(',')
            .map(str::trim)
            .filter(|flag| !flag.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        if let Some(volume) = volumes.iter_mut().find(|volume| {
            volume.identifier == identifier || volume.mount_point.as_deref() == Some(mount_point)
        }) {
            for flag in &flags {
                if !volume.flags.contains(flag) {
                    volume.flags.push(flag.clone());
                }
            }
            if flags
                .iter()
                .any(|flag| flag == "read-only" || flag == "sealed")
            {
                volume.risk = RiskLevel::ReadOnlySystem;
                let note =
                    "Mounted read-only; deleting the mountpoint will not free its backing storage.";
                if !volume.notes.iter().any(|existing| existing == note) {
                    volume.notes.push(note.to_string());
                }
            }
        } else {
            let (_, mut risk, _) = classify_path(mount_point, &flags);
            if flags
                .iter()
                .any(|flag| flag == "read-only" || flag == "sealed")
            {
                risk = RiskLevel::ReadOnlySystem;
            }
            volumes.push(VolumeInfo {
                name: mount_point
                    .split('/')
                    .rfind(|part| !part.is_empty())
                    .unwrap_or(identifier)
                    .to_string(),
                identifier: identifier.to_string(),
                role: role_from_mount_point(mount_point),
                mount_point: Some(mount_point.to_string()),
                mounted: true,
                encrypted: None,
                locked: None,
                flags: flags.clone(),
                capacity_bytes: None,
                used_bytes: None,
                available_bytes: None,
                risk,
                notes: if flags.iter().any(|flag| flag == "read-only" || flag == "sealed") {
                    vec![
                        "Mounted read-only; deleting the mountpoint will not free its backing storage."
                            .to_string(),
                    ]
                } else {
                    Vec::new()
                },
            });
        }
    }
}

pub fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    let mut logs = vec![ScanLog::info(format!(
        "Scanning large blocks at {DATA_VOLUME_PATH}."
    ))];
    let scan = scan_usage_path(DATA_VOLUME_PATH, &mut logs);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    let home = env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let mut logs = vec![ScanLog::info(format!("Scanning user blocks at {home}."))];
    let scan = scan_usage_path(&home, &mut logs);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn scan_path_usage(path: &str) -> ScanResult<Vec<UsageNode>> {
    let mut logs = vec![ScanLog::info(format!("Scanning blocks at {path}."))];
    let scan = scan_usage_path(path, &mut logs);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn start_deep_scan(path: &str) -> ScanResult<DeepScanResult> {
    let path = path.trim();
    if !Path::new(path).is_absolute() || path.contains('\0') || path.contains('\n') {
        return ScanResult {
            data: DeepScanResult {
                path: path.to_string(),
                entries: Vec::new(),
                partial: true,
                canceled: false,
                warnings_summary: DeepScanWarningsSummary {
                    unexpected_errors: vec![
                        "Scan path must be an absolute local filesystem path.".to_string()
                    ],
                    ..DeepScanWarningsSummary::default()
                },
                duration_ms: 0,
            },
            logs: vec![ScanLog::error(
                "Refused deep scan: path must be an absolute local filesystem path.",
            )],
        };
    }

    let _permit = match DeepScanPermit::acquire() {
        Some(permit) => permit,
        None => {
            return ScanResult {
                data: DeepScanResult {
                    path: path.to_string(),
                    entries: Vec::new(),
                    partial: false,
                    canceled: false,
                    warnings_summary: DeepScanWarningsSummary {
                        unexpected_errors: vec!["Another deep scan is already running.".to_string()],
                        ..DeepScanWarningsSummary::default()
                    },
                    duration_ms: 0,
                },
                logs: Vec::new(),
            };
        }
    };

    DEEP_SCAN_CANCEL.store(false, Ordering::Release);
    let mut logs = vec![ScanLog::info(format!("Starting deep scan at {path}."))];
    let started = Instant::now();
    let scan = scan_usage_path(path, &mut logs);
    let duration_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

    ScanResult {
        data: DeepScanResult {
            path: path.to_string(),
            entries: scan.entries,
            partial: scan.partial,
            canceled: scan.canceled,
            warnings_summary: scan.warnings_summary,
            duration_ms,
        },
        logs,
    }
}

pub fn cancel_deep_scan() -> ScanResult<bool> {
    let running = DEEP_SCAN_RUNNING.load(Ordering::Acquire);
    if running {
        DEEP_SCAN_CANCEL.store(true, Ordering::Release);
    }

    ScanResult {
        data: running,
        logs: vec![if running {
            ScanLog::warning("Cancel requested for the active deep scan.")
        } else {
            ScanLog::info("No active deep scan to cancel.")
        }],
    }
}

struct DeepScanPermit;

impl DeepScanPermit {
    fn acquire() -> Option<Self> {
        DEEP_SCAN_RUNNING
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self)
    }
}

impl Drop for DeepScanPermit {
    fn drop(&mut self) {
        DEEP_SCAN_RUNNING.store(false, Ordering::Release);
    }
}

pub fn scan_assets_v2() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Scanning AssetsV2 known targets.")];
    let mut findings = Vec::new();

    for root in ASSETS_V2_PATHS {
        if !Path::new(root).exists() {
            continue;
        }

        let _flags = flags_for_path(root, &mut logs);
        let root_size = disk_usage_bytes(root, &mut logs);
        findings.push(Finding {
            title: "AssetsV2 root detected".to_string(),
            path: Some((*root).to_string()),
            size_bytes: root_size,
            category: StorageCategory::AssetsV2,
            risk: RiskLevel::ReadOnlySystem,
            reason: "AssetsV2 is protected in normal boot; normal cleanup must not force changes."
                .to_string(),
            recommended_action: "Inspect only. Use Apple tooling or Recovery for protected assets."
                .to_string(),
            destructive: false,
            action_profile: Some(action_profile_for_finding(
                Some(root),
                &StorageCategory::AssetsV2,
                &RiskLevel::ReadOnlySystem,
            )),
        });

        for target in ASSETS_V2_TARGETS {
            let path = format!("{root}/{target}");
            if Path::new(&path).exists() {
                let size_bytes = disk_usage_bytes(&path, &mut logs);
                let action_profile = action_profile_for_finding(
                    Some(path.as_str()),
                    &StorageCategory::AssetsV2,
                    &RiskLevel::ReadOnlySystem,
                );
                findings.push(Finding {
                    title: format!("Known MobileAsset: {target}"),
                    path: Some(path),
                    size_bytes,
                    category: StorageCategory::AssetsV2,
                    risk: RiskLevel::ReadOnlySystem,
                    reason: "Known large MobileAsset class inside protected AssetsV2 storage."
                        .to_string(),
                    recommended_action: "Inspect only in CleanerX; do not remove in normal boot."
                        .to_string(),
                    destructive: false,
                    action_profile: Some(action_profile),
                });
            }
        }
    }

    if findings.is_empty() {
        logs.push(ScanLog::info(
            "No AssetsV2 roots were found at known paths.",
        ));
    }

    sort_findings_by_size(&mut findings);

    ScanResult {
        data: findings,
        logs,
    }
}

pub fn scan_developer_tools() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Scanning known developer tool locations.")];
    let mut findings = Vec::new();
    let home = env::var("HOME").unwrap_or_default();
    let paths = [
        "/Applications/Xcode.app".to_string(),
        "/Library/Developer/CommandLineTools".to_string(),
        format!("{home}/Library/Developer/CoreSimulator"),
        format!("{home}/Library/Android/sdk"),
        format!("{home}/.gradle"),
        "/opt/homebrew".to_string(),
    ];

    for path in paths {
        if Path::new(&path).exists() {
            findings.push(finding_for_path(
                "Developer tool storage detected",
                &path,
                disk_usage_bytes(&path, &mut logs),
            ));
        }
    }

    sort_findings_by_size(&mut findings);

    ScanResult {
        data: findings,
        logs,
    }
}

pub fn scan_rust_artifacts() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Scanning Rust toolchain/cache locations.")];
    let home = env::var("HOME").unwrap_or_default();
    let mut findings = Vec::new();
    let paths = [
        format!("{home}/.rustup"),
        format!("{home}/.cargo"),
    ];
    let mut target_roots = discover_rust_target_directories(&home);

    for path in paths {
        if Path::new(&path).exists() {
            findings.push(finding_for_path(
                "Rust storage detected",
                &path,
                disk_usage_bytes(&path, &mut logs),
            ));
        }
    }

    for path in target_roots.drain(..) {
        findings.push(finding_for_path(
            "Rust target directory",
            &path,
            disk_usage_bytes(&path, &mut logs),
        ));
    }

    sort_findings_by_size(&mut findings);

    ScanResult {
        data: findings,
        logs,
    }
}

fn discover_rust_target_directories(home: &str) -> Vec<String> {
    let mut findings = Vec::new();

    for hint in RUST_TARGET_SCAN_ROOT_HINTS {
        let base = format!("{home}/{hint}");
        if !Path::new(&base).exists() {
            continue;
        }
        gather_target_directories(Path::new(&base), 0, RUST_TARGET_SCAN_MAX_DEPTH, &mut findings);
    }

    findings.sort();
    findings.dedup();
    findings
}

fn gather_target_directories(
    root: &std::path::Path,
    depth: usize,
    max_depth: usize,
    findings: &mut Vec<String>,
) {
    if depth >= max_depth {
        return;
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" {
            if has_cargo_marker(&entry.path()) {
                findings.push(entry.path().to_string_lossy().to_string());
            }
            continue;
        }

        if is_noisy_directory(&name) {
            continue;
        }

        gather_target_directories(
            &entry.path(),
            depth + 1,
            max_depth,
            findings,
        );
    }
}

fn has_cargo_marker(target_dir: &std::path::Path) -> bool {
    let mut current = target_dir.parent();
    let mut depth = 0usize;
    while let Some(path) = current {
        if path.join("Cargo.toml").is_file() {
            return true;
        }
        if path.join("Cargo.lock").is_file() {
            return true;
        }
        depth += 1;
        if depth >= 3 {
            break;
        }
        current = path.parent();
    }
    false
}

fn is_noisy_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".cache"
            | "node_modules"
            | ".venv"
            | "target"
            | "vendor"
            | "dist"
            | "build"
            | "Pods"
            | ".gradle"
            | "Library"
            | ".idea"
    )
}

pub fn scan_containers() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Scanning known container storage locations.")];
    let home = env::var("HOME").unwrap_or_default();
    let mut findings = Vec::new();
    let paths = [
        format!("{home}/Library/Containers/com.docker.docker"),
        format!("{home}/Library/Group Containers/group.com.docker"),
        format!("{home}/.orbstack"),
        format!("{home}/.local/share/containers"),
    ];

    for path in paths {
        if Path::new(&path).exists() {
            findings.push(finding_for_path(
                "Container storage detected",
                &path,
                disk_usage_bytes(&path, &mut logs),
            ));
        }
    }

    sort_findings_by_size(&mut findings);

    ScanResult {
        data: findings,
        logs,
    }
}

pub fn list_snapshots() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Listing local Time Machine snapshots.")];
    let mut findings = Vec::new();

    match run("tmutil", &["listlocalsnapshots", "/"]) {
        Ok(output) => {
            for line in output
                .stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                findings.push(Finding {
                    title: "Local Time Machine snapshot".to_string(),
                    path: None,
                    size_bytes: None,
                    category: StorageCategory::Snapshots,
                    risk: RiskLevel::ReviewRequired,
                    reason: line.to_string(),
                    recommended_action:
                        "MVP: list only. Later: thin snapshots with explicit confirmation."
                            .to_string(),
                    destructive: false,
                    action_profile: Some(action_profile_for_finding(
                        None,
                        &StorageCategory::Snapshots,
                        &RiskLevel::ReviewRequired,
                    )),
                });
            }

            if findings.is_empty() {
                logs.push(ScanLog::info("No local snapshots reported by tmutil."));
            }
        }
        Err(error) => logs.push(log_command_error(
            "tmutil listlocalsnapshots failed",
            &error,
        )),
    }

    sort_findings_by_size(&mut findings);

    ScanResult {
        data: findings,
        logs,
    }
}

fn parse_df_volumes(logs: &mut Vec<ScanLog>) -> Vec<VolumeInfo> {
    let output = match run("df", &["-k"]) {
        Ok(output) => output.stdout,
        Err(error) => {
            logs.push(log_command_error("df -k failed", &error));
            return Vec::new();
        }
    };

    output
        .lines()
        .skip(1)
        .filter_map(parse_df_line)
        .collect::<Vec<_>>()
}

fn parse_df_line(line: &str) -> Option<VolumeInfo> {
    let mut fields = line.split_whitespace();
    let filesystem = fields.next()?;
    let total = fields.next()?;
    let used = fields.next()?;
    let available = fields.next()?;
    let _capacity = fields.next()?;
    let _iused = fields.next()?;
    let _ifree = fields.next()?;
    let _percent_iused = fields.next()?;
    let mount_point = fields.collect::<Vec<_>>().join(" ");
    if mount_point.is_empty() {
        return None;
    }

    let total_bytes = parse_kib(total);
    let used_bytes = parse_kib(used);
    let available_bytes = parse_kib(available);
    let name = if mount_point == "/" {
        "System".to_string()
    } else {
        mount_point
            .split('/')
            .rfind(|part| !part.is_empty())
            .unwrap_or(filesystem)
            .to_string()
    };
    let flags = Vec::new();
    let (_, risk, _) = classify_path(&mount_point, &flags);

    Some(VolumeInfo {
        name,
        identifier: filesystem.to_string(),
        role: role_from_mount_point(&mount_point),
        mount_point: Some(mount_point),
        mounted: true,
        encrypted: None,
        locked: None,
        flags,
        capacity_bytes: total_bytes,
        used_bytes,
        available_bytes,
        risk,
        notes: Vec::new(),
    })
}

fn merge_apfs_metadata(volumes: &mut Vec<VolumeInfo>, output: &str) {
    let mut current: Option<VolumeInfo> = None;
    for raw in output.lines() {
        let line = raw.trim();
        if line.contains("APFS Volume Disk (Role):") {
            if let Some(volume) = current.take() {
                upsert_volume(volumes, volume);
            }

            let right = line.split(':').nth(1).unwrap_or("").trim();
            let identifier = right.split_whitespace().next().unwrap_or("").to_string();
            let role = right
                .split('(')
                .nth(1)
                .and_then(|value| value.split(')').next())
                .map(str::to_string);
            current = Some(VolumeInfo {
                name: identifier.clone(),
                identifier,
                role,
                mount_point: None,
                mounted: false,
                encrypted: None,
                locked: None,
                flags: Vec::new(),
                capacity_bytes: None,
                used_bytes: None,
                available_bytes: None,
                risk: RiskLevel::ReadOnlySystem,
                notes: Vec::new(),
            });
            continue;
        }

        if let Some(volume) = current.as_mut() {
            if let Some(value) = line.strip_prefix("Name:") {
                volume.name = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Mount Point:") {
                let mount = value.trim();
                if mount != "Not Mounted" && !mount.is_empty() {
                    volume.mount_point = Some(mount.to_string());
                    volume.mounted = true;
                    let (_, risk, _) = classify_path(mount, &[]);
                    volume.risk = risk;
                }
            } else if let Some(value) = line.strip_prefix("FileVault:") {
                let value = value.trim();
                volume.encrypted = Some(value != "No");
                volume.locked = Some(value.contains("Locked"));
                if value.contains("Locked") {
                    volume.notes.push(
                        "Encrypted/locked volume; unlock and mount before operating.".to_string(),
                    );
                }
            } else if let Some(value) = line.strip_prefix("Capacity Consumed:") {
                volume.used_bytes = parse_sizeish_bytes(value);
            }
        }
    }

    if let Some(volume) = current.take() {
        upsert_volume(volumes, volume);
    }
}

fn merge_diskutil_list_metadata(volumes: &mut [VolumeInfo], output: &str) {
    for line in output.lines().map(str::trim) {
        if !(line.contains("Apple_APFS") || line.contains("APFS Volume")) {
            continue;
        }

        for volume in volumes.iter_mut() {
            if line.contains(&volume.identifier) && volume.role.is_none() {
                volume.role = Some("APFS".to_string());
            }
        }
    }
}

fn upsert_volume(volumes: &mut Vec<VolumeInfo>, incoming: VolumeInfo) {
    if let Some(existing) = volumes.iter_mut().find(|volume| {
        volume.identifier == incoming.identifier
            || volume
                .mount_point
                .as_ref()
                .zip(incoming.mount_point.as_ref())
                .is_some_and(|(left, right)| left == right)
    }) {
        if existing.role.is_none() {
            existing.role = incoming.role;
        }
        if existing.mount_point.is_none() {
            existing.mount_point = incoming.mount_point;
            existing.mounted = existing.mount_point.is_some();
        }
        if existing.encrypted.is_none() {
            existing.encrypted = incoming.encrypted;
        }
        if existing.locked.is_none() {
            existing.locked = incoming.locked;
        }
        if existing.used_bytes.is_none() {
            existing.used_bytes = incoming.used_bytes;
        }
        existing.notes.extend(incoming.notes);
    } else {
        volumes.push(incoming);
    }
}

struct UsageScan {
    entries: Vec<UsageNode>,
    partial: bool,
    canceled: bool,
    warnings_summary: DeepScanWarningsSummary,
}

fn scan_usage_path(path: &str, logs: &mut Vec<ScanLog>) -> UsageScan {
    if !Path::new(path).exists() {
        logs.push(ScanLog::warning(format!(
            "{path} does not exist or is not mounted."
        )));
        return UsageScan {
            entries: Vec::new(),
            partial: true,
            canceled: false,
            warnings_summary: DeepScanWarningsSummary {
                vanished_paths: 1,
                samples: vec![path.to_string()],
                ..DeepScanWarningsSummary::default()
            },
        };
    }

    let scan_path = scan_operand_path(path);
    let output = match run_partial_with_timeout_and_cancel(
        "du",
        &["-x", "-k", "-d", "1", &scan_path],
        DEEP_SCAN_TIMEOUT,
        Some(&DEEP_SCAN_CANCEL),
    ) {
        Ok(output) => output,
        Err(error) => {
            logs.push(ScanLog::error(format!(
                "du scan failed for {path}: {error}"
            )));
            return UsageScan {
                entries: Vec::new(),
                partial: false,
                canceled: false,
                warnings_summary: DeepScanWarningsSummary {
                    unexpected_errors: vec![error.to_string()],
                    ..DeepScanWarningsSummary::default()
                },
            };
        }
    };

    let mut nodes = output
        .stdout
        .lines()
        .filter_map(|line| parse_du_line(line, logs))
        .collect::<Vec<_>>();
    append_direct_file_nodes(path, &mut nodes, logs);
    nodes.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));

    let stderr_summary = summarize_du_stderr(&output.stderr);
    let warning_messages = stderr_summary.warning_messages();
    let has_expected_warnings = stderr_summary.has_expected_warnings();
    let mut warnings_summary = stderr_summary.warnings;
    if output.timed_out {
        let message = format!(
            "du timed out after {}s and the process was stopped.",
            DEEP_SCAN_TIMEOUT.as_secs()
        );
        logs.push(ScanLog::warning(message.clone()));
        push_sample(&mut warnings_summary.samples, &message);
    }
    if output.canceled {
        let message = "du scan was canceled and the process was stopped.".to_string();
        logs.push(ScanLog::warning(message.clone()));
        push_sample(&mut warnings_summary.samples, &message);
    }
    for warning in warning_messages {
        logs.push(ScanLog::warning(warning));
    }
    for error in &warnings_summary.unexpected_errors {
        logs.push(ScanLog::error(format!("du unexpected error: {error}")));
    }

    let has_unexpected_errors = !warnings_summary.unexpected_errors.is_empty();
    if nodes.is_empty() && has_unexpected_errors {
        logs.push(ScanLog::error(format!(
            "du scan for {path} did not return parseable entries."
        )));
    } else if nodes.is_empty() && !output.success && has_expected_warnings {
        logs.push(ScanLog::warning(format!(
            "du scan for {path} returned no visible entries after expected macOS permission skips."
        )));
    } else if nodes.is_empty() && output.timed_out {
        logs.push(ScanLog::warning(format!(
            "du scan for {path} was stopped by timeout before visible entries were returned."
        )));
    } else if nodes.is_empty() && output.canceled {
        logs.push(ScanLog::warning(format!(
            "du scan for {path} was canceled before visible entries were returned."
        )));
    }

    if !output.success && !nodes.is_empty() {
        if output.timed_out {
            logs.push(ScanLog::warning(format!(
                "du scan for {path} returned partial results before timeout."
            )));
        } else if output.canceled {
            logs.push(ScanLog::warning(format!(
                "du scan for {path} returned partial results before cancellation."
            )));
        } else {
            logs.push(ScanLog::warning(format!(
                "du scan for {path} returned partial results with exit status {}.",
                output
                    .status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )));
        }
    }

    UsageScan {
        entries: {
            register_usage_nodes(&nodes);
            nodes
        },
        partial: !output.success
            || output.timed_out
            || output.canceled
            || has_expected_warnings
            || has_unexpected_errors,
        canceled: output.canceled,
        warnings_summary,
    }
}

fn scan_operand_path(path: &str) -> String {
    if path == "/tmp" {
        "/tmp/".to_string()
    } else {
        path.to_string()
    }
}

fn append_direct_file_nodes(path: &str, nodes: &mut Vec<UsageNode>, logs: &mut Vec<ScanLog>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let path_string = entry_path.to_string_lossy().to_string();
        if nodes.iter().any(|node| node.path == path_string) {
            continue;
        }

        let Ok(metadata) = fs::symlink_metadata(&entry_path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        let flags = flags_for_path(&path_string, logs);
        let (category, risk, _) = classify_path(&path_string, &flags);
        nodes.push(UsageNode {
            id: cleanup_candidate_id(&path_string),
            path: path_string,
            kind: UsageKind::File,
            size_bytes: metadata.len(),
            category,
            risk,
            flags,
            children: Vec::new(),
        });
    }
}

fn parse_du_line(line: &str, logs: &mut Vec<ScanLog>) -> Option<UsageNode> {
    let (size_kib, path) = line.split_once(char::is_whitespace)?;
    let path = path.trim();
    let size_bytes = size_kib.parse::<u64>().ok()?.saturating_mul(1024);
    let flags = flags_for_path(path, logs);
    let (category, risk, _) = classify_path(path, &flags);
    let kind = fs::symlink_metadata(path)
        .map(|metadata| {
            if metadata.is_file() {
                UsageKind::File
            } else {
                UsageKind::Folder
            }
        })
        .unwrap_or(UsageKind::Folder);

    Some(UsageNode {
        id: cleanup_candidate_id(path),
        path: path.to_string(),
        kind,
        size_bytes,
        category,
        risk,
        flags,
        children: Vec::new(),
    })
}

struct DuStderrSummary {
    warnings: DeepScanWarningsSummary,
}

fn summarize_du_stderr(stderr: &str) -> DuStderrSummary {
    let mut warnings = DeepScanWarningsSummary::default();

    for line in stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if line.contains("Permission denied") {
            warnings.permission_denied += 1;
            push_sample(&mut warnings.samples, line);
        } else if line.contains("Operation not permitted") {
            warnings.operation_not_permitted += 1;
            push_sample(&mut warnings.samples, line);
        } else if line.contains("No such file or directory") {
            warnings.vanished_paths += 1;
            push_sample(&mut warnings.samples, line);
        } else {
            push_sample(&mut warnings.samples, line);
            push_sample(&mut warnings.unexpected_errors, line);
        }
    }

    DuStderrSummary { warnings }
}

impl DuStderrSummary {
    fn has_expected_warnings(&self) -> bool {
        self.warnings.permission_denied > 0
            || self.warnings.operation_not_permitted > 0
            || self.warnings.vanished_paths > 0
    }

    fn warning_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if self.warnings.permission_denied > 0 {
            messages.push(format!(
                "du skipped {} path(s) due to Permission denied.",
                self.warnings.permission_denied
            ));
        }
        if self.warnings.operation_not_permitted > 0 {
            messages.push(format!(
                "du skipped {} path(s) due to Operation not permitted.",
                self.warnings.operation_not_permitted
            ));
        }
        if self.warnings.vanished_paths > 0 {
            messages.push(format!(
                "du skipped {} path(s) that disappeared during the scan.",
                self.warnings.vanished_paths
            ));
        }
        messages
    }
}

fn push_sample(samples: &mut Vec<String>, line: &str) {
    if samples.len() < 3 {
        samples.push(line.to_string());
    }
}

fn flags_for_path(path: &str, logs: &mut Vec<ScanLog>) -> Vec<String> {
    match run("ls", &["-ldOe", path]) {
        Ok(output) => parse_flags_from_ls(&output.stdout),
        Err(error) => {
            if fs::metadata(path).is_ok() {
                logs.push(log_command_error(
                    &format!("flag scan failed for {path}"),
                    &error,
                ));
            }
            Vec::new()
        }
    }
}

fn parse_flags_from_ls(output: &str) -> Vec<String> {
    let known = ["restricted", "hidden", "uchg", "schg", "compressed"];
    let mut flags = Vec::new();
    for flag in known {
        if output.split_whitespace().any(|part| part.contains(flag)) {
            flags.push(flag.to_string());
        }
    }
    flags
}

fn disk_usage_bytes(path: &str, logs: &mut Vec<ScanLog>) -> Option<u64> {
    let output = run("du", &["-sk", path]).map_err(|error| {
        logs.push(log_command_error(
            &format!("du -sk failed for {path}"),
            &error,
        ));
    });
    let output = output.ok()?;
    output
        .stdout
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kib| kib.saturating_mul(1024))
}

fn summarize_volumes(volumes: &[VolumeInfo]) -> StorageSummary {
    let primary = volumes
        .iter()
        .find(|volume| volume.mount_point.as_deref() == Some(DATA_VOLUME_PATH))
        .or_else(|| {
            volumes
                .iter()
                .find(|volume| volume.mount_point.as_deref() == Some("/"))
        })
        .or_else(|| volumes.first());

    if let Some(volume) = primary {
        let total = volume.capacity_bytes;
        let used = volume.used_bytes;
        let available = volume.available_bytes;
        let percent_used = total
            .zip(used)
            .and_then(|(total, used)| (total > 0).then_some((used as f64 / total as f64) * 100.0));

        StorageSummary {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            percent_used,
        }
    } else {
        StorageSummary {
            total_bytes: None,
            used_bytes: None,
            available_bytes: None,
            percent_used: None,
        }
    }
}

fn summary_warnings(volumes: &[VolumeInfo]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let summary = summarize_volumes(volumes);

    if let Some(available) = summary.available_bytes {
        let gib = available as f64 / 1024.0 / 1024.0 / 1024.0;
        if gib < 10.0 {
            findings.push(Finding {
                title: "Critical free space".to_string(),
                path: None,
                size_bytes: Some(available),
                category: StorageCategory::MacOsApfs,
                risk: RiskLevel::Dangerous,
                reason: "Less than 10 GB free on the primary mounted volume.".to_string(),
                recommended_action: "Review large blocks and snapshots immediately.".to_string(),
                destructive: false,
                action_profile: Some(action_profile_for_finding(
                    None,
                    &StorageCategory::MacOsApfs,
                    &RiskLevel::Dangerous,
                )),
            });
        } else if gib < 15.0 {
            findings.push(Finding {
                title: "Low free space".to_string(),
                path: None,
                size_bytes: Some(available),
                category: StorageCategory::MacOsApfs,
                risk: RiskLevel::ReviewRequired,
                reason: "Less than 15 GB free on the primary mounted volume.".to_string(),
                recommended_action: "Review large blocks before macOS updates or builds."
                    .to_string(),
                destructive: false,
                action_profile: Some(action_profile_for_finding(
                    None,
                    &StorageCategory::MacOsApfs,
                    &RiskLevel::ReviewRequired,
                )),
            });
        }
    }

    findings
}

fn sort_findings_by_size(findings: &mut [Finding]) {
    findings.sort_by(|left, right| {
        right
            .size_bytes
            .unwrap_or(0)
            .cmp(&left.size_bytes.unwrap_or(0))
            .then_with(|| left.title.cmp(&right.title))
    });
}

fn role_from_mount_point(mount_point: &str) -> Option<String> {
    let roles = HashMap::from([
        ("/", "System"),
        ("/System/Volumes/Data", "Data"),
        ("/System/Volumes/VM", "VM"),
        ("/System/Volumes/Update", "Update"),
        ("/System/Volumes/Preboot", "Preboot"),
    ]);
    roles.get(mount_point).map(|value| value.to_string())
}

fn parse_kib(value: &str) -> Option<u64> {
    value
        .parse::<u64>()
        .ok()
        .map(|kib| kib.saturating_mul(1024))
}

fn parse_sizeish_bytes(value: &str) -> Option<u64> {
    let normalized = value.trim().replace(',', "");
    let mut parts = normalized.split_whitespace();
    let amount = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().unwrap_or("B");
    let multiplier = match unit {
        "B" | "Bytes" => 1.0,
        "KB" | "KiB" => 1024.0,
        "MB" | "MiB" => 1024.0 * 1024.0,
        "GB" | "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TB" | "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    Some((amount * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_df_line_with_mount_point() {
        let line = "/dev/disk3s5 488245288 400000000 88245288 82% 1 2 34% /System/Volumes/Data";
        let parsed = parse_df_line(line).unwrap();
        assert_eq!(parsed.mount_point.as_deref(), Some("/System/Volumes/Data"));
        assert_eq!(parsed.role.as_deref(), Some("Data"));
    }

    #[test]
    fn parses_restricted_flag() {
        let flags = parse_flags_from_ls("drwxr-xr-x@ 3 root wheel restricted 96 Jun 1 AssetsV2");
        assert_eq!(flags, vec!["restricted"]);
    }

    #[test]
    fn summarizes_expected_du_permission_errors() {
        let summary = summarize_du_stderr(
            "du: /System/Volumes/Data/.Spotlight-V100: Permission denied\n\
             du: /System/Volumes/Data/private/var/db: Operation not permitted\n\
             du: /tmp/gone: No such file or directory",
        );
        assert_eq!(summary.warnings.permission_denied, 1);
        assert_eq!(summary.warnings.operation_not_permitted, 1);
        assert_eq!(summary.warnings.vanished_paths, 1);
        assert!(summary.warnings.unexpected_errors.is_empty());
        assert_eq!(summary.warnings.samples.len(), 3);
    }

    #[test]
    fn summarizes_unexpected_du_errors() {
        let summary = summarize_du_stderr("du: strange failure");
        assert_eq!(summary.warnings.permission_denied, 0);
        assert_eq!(summary.warnings.unexpected_errors.len(), 1);
        assert_eq!(summary.warnings.samples.len(), 1);
    }

    #[test]
    fn parses_du_line_with_spaces_in_path() {
        let mut logs = Vec::new();
        let parsed = parse_du_line(
            "1024\t/Users/me/Library/Application Support/App Cache",
            &mut logs,
        )
        .unwrap();
        assert_eq!(
            parsed.path,
            "/Users/me/Library/Application Support/App Cache"
        );
        assert_eq!(parsed.size_bytes, 1024 * 1024);
    }
}
