use crate::classify::{action_profile_for_finding, classify_path, finding_for_path};
use crate::cleanup::{cleanup_candidate_id, register_usage_nodes};
use crate::command::{
    log_command_error, run, run_partial_with_cancel, CommandError, CommandOutput,
};
use crate::models::{
    DeepScanProgress, DeepScanResult, DeepScanWarningsSummary, Finding, Overview, RiskLevel,
    ScanLog, ScanResult, StorageCategory, StorageSummary, UsageKind, UsageNode, VolumeInfo,
    VolumeOperationResult,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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

pub fn mount_volume(identifier: String, elevated: bool) -> ScanResult<VolumeOperationResult> {
    run_volume_operation(identifier, VolumeOperation::Mount, elevated)
}

pub fn unmount_volume(identifier: String, elevated: bool) -> ScanResult<VolumeOperationResult> {
    run_volume_operation(identifier, VolumeOperation::Unmount, elevated)
}

#[derive(Clone, Copy)]
enum VolumeOperation {
    Mount,
    Unmount,
}

impl VolumeOperation {
    fn diskutil_action(self) -> &'static str {
        match self {
            Self::Mount => "mount",
            Self::Unmount => "unmount",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Mount => "mount",
            Self::Unmount => "unmount",
        }
    }
}

fn run_volume_operation(
    identifier: String,
    operation: VolumeOperation,
    elevated: bool,
) -> ScanResult<VolumeOperationResult> {
    let identifier = identifier.trim().to_string();
    let mut logs = vec![ScanLog::info(format!(
        "Preparing to {} volume {identifier}.",
        operation.label()
    ))];

    let before = scan_volumes();
    logs.extend(before.logs);

    if identifier.is_empty() || identifier.contains('\0') || identifier.contains('\n') {
        logs.push(ScanLog::error(
            "Volume operation refused: invalid volume identifier.",
        ));
        return ScanResult {
            data: volume_operation_result(before.data, None),
            logs,
        };
    }

    let volume = find_volume_for_identifier(&before.data, &identifier).cloned();
    if let Some(volume) = &volume {
        match operation {
            VolumeOperation::Mount => {
                if volume.mounted {
                    logs.push(ScanLog::info(format!(
                        "{} is already mounted.",
                        volume.identifier
                    )));
                    return ScanResult {
                        data: volume_operation_result(before.data, volume.mount_point.clone()),
                        logs,
                    };
                }
                if volume.locked.unwrap_or(false) {
                    logs.push(ScanLog::warning(format!(
                        "{} is locked. Unlock it before mounting.",
                        volume.identifier
                    )));
                    return ScanResult {
                        data: volume_operation_result(before.data, None),
                        logs,
                    };
                }
                if is_protected_support_volume(volume) {
                    logs.push(ScanLog::warning(format!(
                        "{} is a protected APFS support volume; cleenosx will not mount it for cleanup.",
                        volume.identifier
                    )));
                    return ScanResult {
                        data: volume_operation_result(before.data, None),
                        logs,
                    };
                }
            }
            VolumeOperation::Unmount => {
                if !volume.mounted {
                    logs.push(ScanLog::info(format!(
                        "{} is already unmounted.",
                        volume.identifier
                    )));
                    return ScanResult {
                        data: volume_operation_result(before.data, None),
                        logs,
                    };
                }
                if is_protected_live_mount(volume) {
                    logs.push(ScanLog::warning(format!(
                        "{} is a live system/support mount; cleenosx will not unmount it.",
                        volume.identifier
                    )));
                    return ScanResult {
                        data: volume_operation_result(before.data, volume.mount_point.clone()),
                        logs,
                    };
                }
            }
        }
    }

    if elevated {
        logs.push(ScanLog::info(format!(
            "Requesting administrator permission to {} volume {identifier}.",
            operation.label()
        )));
    }

    let command_target = volume_operation_target(volume.as_ref(), &identifier, operation);
    let completed =
        match run_diskutil_volume_operation(command_target.as_str(), operation, elevated) {
            Ok(output) => {
                let output = output.trim();
                logs.push(ScanLog::info(if output.is_empty() {
                    format!("diskutil {} completed.", operation.label())
                } else {
                    output.to_string()
                }));
                true
            }
            Err(error) if matches!(operation, VolumeOperation::Unmount) => {
                logs.push(log_command_error(
                    &format!("diskutil {} {command_target} failed", operation.label()),
                    &error,
                ));
                logs.push(ScanLog::info(format!(
                    "Trying umount fallback for {command_target}."
                )));
                match run_umount_volume(command_target.as_str(), elevated) {
                    Ok(output) => {
                        let output = output.trim();
                        logs.push(ScanLog::info(if output.is_empty() {
                            "umount completed.".to_string()
                        } else {
                            output.to_string()
                        }));
                        true
                    }
                    Err(fallback_error) => {
                        logs.push(log_command_error(
                            &format!("umount {command_target} failed"),
                            &fallback_error,
                        ));
                        false
                    }
                }
            }
            Err(error) => {
                logs.push(log_command_error(
                    &format!("diskutil {} {command_target} failed", operation.label()),
                    &error,
                ));
                false
            }
        };

    let after = scan_volumes();
    logs.extend(after.logs);
    let mount_point = match operation {
        VolumeOperation::Mount => find_volume_mount_point(&after.data, &identifier)
            .or_else(|| diskutil_mount_point(&command_target, &mut logs)),
        VolumeOperation::Unmount => completed
            .then(|| {
                volume
                    .and_then(|volume| volume.mount_point)
                    .or_else(|| identifier.starts_with('/').then(|| identifier.clone()))
            })
            .flatten(),
    };

    ScanResult {
        data: volume_operation_result(after.data, mount_point),
        logs,
    }
}

fn volume_operation_target(
    volume: Option<&VolumeInfo>,
    requested: &str,
    operation: VolumeOperation,
) -> String {
    match operation {
        VolumeOperation::Mount => volume
            .map(|volume| volume.identifier.clone())
            .unwrap_or_else(|| requested.to_string()),
        VolumeOperation::Unmount => volume
            .and_then(|volume| volume.mount_point.clone())
            .unwrap_or_else(|| requested.to_string()),
    }
}

fn volume_operation_result(
    volumes: Vec<VolumeInfo>,
    mount_point: Option<String>,
) -> VolumeOperationResult {
    VolumeOperationResult {
        volumes,
        mount_point,
    }
}

fn find_volume_mount_point(volumes: &[VolumeInfo], identifier: &str) -> Option<String> {
    find_volume_for_identifier(volumes, identifier).and_then(|volume| volume.mount_point.clone())
}

fn find_volume_for_identifier<'a>(
    volumes: &'a [VolumeInfo],
    identifier: &str,
) -> Option<&'a VolumeInfo> {
    let normalized = identifier.trim_start_matches("/dev/");
    volumes
        .iter()
        .find(|volume| {
            volume.identifier == identifier
                || volume.identifier.trim_start_matches("/dev/") == normalized
                || volume.mount_point.as_deref() == Some(identifier)
        })
        .or_else(|| nearest_mount_for_path(volumes, identifier))
}

fn nearest_mount_for_path<'a>(volumes: &'a [VolumeInfo], path: &str) -> Option<&'a VolumeInfo> {
    if !path.starts_with('/') {
        return None;
    }

    volumes
        .iter()
        .filter_map(|volume| {
            let mount_point = volume.mount_point.as_deref()?;
            path_contains_mount(path, mount_point).then_some((volume, mount_point.len()))
        })
        .max_by_key(|(_, mount_len)| *mount_len)
        .map(|(volume, _)| volume)
}

fn path_contains_mount(path: &str, mount_point: &str) -> bool {
    let path = trim_trailing_slash(path);
    let mount_point = trim_trailing_slash(mount_point);
    path == mount_point || path.starts_with(&format!("{mount_point}/"))
}

fn trim_trailing_slash(path: &str) -> &str {
    if path == "/" {
        return path;
    }
    path.trim_end_matches('/')
}

fn diskutil_mount_point(identifier: &str, logs: &mut Vec<ScanLog>) -> Option<String> {
    let output = match run("diskutil", &["info", identifier]) {
        Ok(output) => output.stdout,
        Err(error) => {
            logs.push(log_command_error(
                &format!("diskutil info {identifier} failed"),
                &error,
            ));
            return None;
        }
    };
    parse_diskutil_info_mount_point(&output)
}

fn parse_diskutil_info_mount_point(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let value = line.trim().strip_prefix("Mount Point:")?.trim();
        (!value.is_empty() && value != "Not Mounted").then(|| value.to_string())
    })
}

fn run_diskutil_volume_operation(
    identifier: &str,
    operation: VolumeOperation,
    elevated: bool,
) -> Result<String, CommandError> {
    if !elevated {
        return run("diskutil", &[operation.diskutil_action(), identifier])
            .map(|output| output.stdout);
    }

    let shell_script = format!(
        "diskutil {} {}",
        operation.diskutil_action(),
        shell_quote(identifier)
    );
    let apple_script = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&shell_script)
    );
    run("osascript", &["-e", apple_script.as_str()]).map(|output| output.stdout)
}

fn run_umount_volume(target: &str, elevated: bool) -> Result<String, CommandError> {
    if !elevated {
        return run("/sbin/umount", &[target]).map(|output| output.stdout);
    }

    let shell_script = format!("/sbin/umount {}", shell_quote(target));
    let apple_script = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&shell_script)
    );
    run("osascript", &["-e", apple_script.as_str()]).map(|output| output.stdout)
}

fn is_protected_support_volume(volume: &VolumeInfo) -> bool {
    matches!(
        volume.role.as_deref(),
        Some(
            "System" | "Preboot" | "VM" | "Update" | "Recovery" | "xART" | "Hardware" | "Baseband"
        )
    )
}

fn is_protected_live_mount(volume: &VolumeInfo) -> bool {
    matches!(
        volume.mount_point.as_deref(),
        Some(
            "/" | DATA_VOLUME_PATH
                | "/System/Volumes/Preboot"
                | "/System/Volumes/VM"
                | "/System/Volumes/Update"
        )
    ) || is_protected_support_volume(volume)
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
    let scan = scan_usage_path(DATA_VOLUME_PATH, &mut logs, false);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    let home = env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let mut logs = vec![ScanLog::info(format!("Scanning user blocks at {home}."))];
    let scan = scan_usage_path(&home, &mut logs, false);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn scan_path_usage(path: &str) -> ScanResult<Vec<UsageNode>> {
    let mut logs = vec![ScanLog::info(format!("Scanning blocks at {path}."))];
    let scan = scan_usage_path(path, &mut logs, false);

    ScanResult {
        data: scan.entries,
        logs,
    }
}

pub fn start_deep_scan(path: &str, elevated: bool) -> ScanResult<DeepScanResult> {
    start_deep_scan_with_progress(path, elevated, |_| {})
}

pub fn start_deep_scan_with_progress<F>(
    path: &str,
    elevated: bool,
    on_progress: F,
) -> ScanResult<DeepScanResult>
where
    F: Fn(DeepScanProgress) + Send + Sync,
{
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
    let scan = scan_usage_path_with_progress(path, &mut logs, elevated, Some(&on_progress));
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
                    recommended_action: "Inspect only in cleenosx; do not remove in normal boot."
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
    let paths = [format!("{home}/.rustup"), format!("{home}/.cargo")];
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
        gather_target_directories(
            Path::new(&base),
            0,
            RUST_TARGET_SCAN_MAX_DEPTH,
            &mut findings,
        );
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

        gather_target_directories(&entry.path(), depth + 1, max_depth, findings);
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

fn scan_usage_path(path: &str, logs: &mut Vec<ScanLog>, elevated: bool) -> UsageScan {
    scan_usage_path_with_progress(path, logs, elevated, None)
}

fn scan_usage_path_with_progress(
    path: &str,
    logs: &mut Vec<ScanLog>,
    elevated: bool,
    on_progress: Option<&(dyn Fn(DeepScanProgress) + Send + Sync)>,
) -> UsageScan {
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
    let targets = direct_scan_targets(&scan_path, logs);
    let total_items = targets.len();
    let mut processed_items = 0usize;
    let mut nodes = Vec::new();
    let mut warnings_summary = DeepScanWarningsSummary::default();
    let mut partial = false;
    let mut canceled = false;

    if elevated {
        logs.push(ScanLog::info(
            "Requesting administrator permission for this scan.".to_string(),
        ));
    }

    emit_deep_scan_progress(
        path,
        None,
        processed_items,
        total_items,
        false,
        total_items == 0,
        on_progress,
    );

    for target in targets {
        if DEEP_SCAN_CANCEL.load(Ordering::Acquire) {
            canceled = true;
            partial = true;
            logs.push(ScanLog::warning(
                "du scan was canceled before the next item started.".to_string(),
            ));
            break;
        }

        emit_deep_scan_progress(
            path,
            Some(&target),
            processed_items,
            total_items,
            false,
            false,
            on_progress,
        );

        let output = match run_du_target_scan(&target, elevated) {
            Ok(output) => output,
            Err(error) => {
                partial = true;
                logs.push(ScanLog::error(format!(
                    "du scan failed for {target}: {error}"
                )));
                push_sample(&mut warnings_summary.unexpected_errors, &error.to_string());
                push_sample(&mut warnings_summary.samples, &error.to_string());
                processed_items += 1;
                emit_deep_scan_progress(
                    path,
                    Some(&target),
                    processed_items,
                    total_items,
                    false,
                    processed_items == total_items,
                    on_progress,
                );
                continue;
            }
        };

        let mut target_nodes = output
            .stdout
            .lines()
            .filter_map(|line| parse_du_line(line, logs))
            .collect::<Vec<_>>();
        let target_had_nodes = !target_nodes.is_empty();
        nodes.append(&mut target_nodes);

        let stderr_summary = summarize_du_stderr(&output.stderr);
        let warning_messages = stderr_summary.warning_messages();
        let has_expected_warnings = stderr_summary.has_expected_warnings();
        let has_unexpected_errors = !stderr_summary.warnings.unexpected_errors.is_empty();
        let unexpected_errors = stderr_summary.warnings.unexpected_errors.clone();
        merge_warnings_summary(&mut warnings_summary, stderr_summary.warnings);
        for warning in warning_messages {
            logs.push(ScanLog::warning(warning));
        }
        for error in unexpected_errors {
            logs.push(ScanLog::error(format!("du unexpected error: {error}")));
        }

        if output.canceled {
            canceled = true;
            partial = true;
            let message = "du scan was canceled and the process was stopped.".to_string();
            logs.push(ScanLog::warning(message.clone()));
            push_sample(&mut warnings_summary.samples, &message);
        } else if !output.success || has_expected_warnings || has_unexpected_errors {
            partial = true;
        }

        if !target_had_nodes && has_unexpected_errors {
            logs.push(ScanLog::error(format!(
                "du scan for {target} did not return parseable entries."
            )));
        } else if output.canceled && !target_had_nodes {
            logs.push(ScanLog::warning(format!(
                "du scan for {target} was canceled before visible entries were returned."
            )));
        } else if !target_had_nodes && !output.success && has_expected_warnings {
            logs.push(ScanLog::warning(format!(
                "du scan for {target} returned no visible entries after expected macOS permission skips."
            )));
        } else if !output.success {
            logs.push(ScanLog::warning(format!(
                "du scan for {target} returned partial results with exit status {}.",
                output
                    .status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )));
        }

        if output.canceled {
            emit_deep_scan_progress(
                path,
                Some(&target),
                processed_items,
                total_items,
                true,
                false,
                on_progress,
            );
            break;
        }

        processed_items += 1;
        emit_deep_scan_progress(
            path,
            Some(&target),
            processed_items,
            total_items,
            false,
            processed_items == total_items,
            on_progress,
        );
    }

    append_direct_file_nodes(path, &mut nodes, logs);
    nodes.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));

    if canceled && !nodes.is_empty() {
        logs.push(ScanLog::warning(format!(
            "du scan for {path} returned partial results before cancellation."
        )));
    }

    UsageScan {
        entries: {
            register_usage_nodes(&nodes);
            nodes
        },
        partial,
        canceled,
        warnings_summary,
    }
}

fn direct_scan_targets(path: &str, logs: &mut Vec<ScanLog>) -> Vec<String> {
    let path_ref = Path::new(path);
    let metadata = match fs::symlink_metadata(path_ref) {
        Ok(metadata) => metadata,
        Err(error) => {
            logs.push(ScanLog::warning(format!(
                "Could not inspect {path}: {error}. Falling back to direct du scan."
            )));
            return vec![path.to_string()];
        }
    };

    if !metadata.is_dir() {
        return vec![path.to_string()];
    }

    let entries = match fs::read_dir(path_ref) {
        Ok(entries) => entries,
        Err(error) => {
            logs.push(ScanLog::warning(format!(
                "Could not list {path}: {error}. Falling back to direct du scan."
            )));
            return vec![path.to_string()];
        }
    };

    let mut targets = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

fn emit_deep_scan_progress(
    path: &str,
    current_path: Option<&str>,
    processed_items: usize,
    total_items: usize,
    canceled: bool,
    finished: bool,
    on_progress: Option<&(dyn Fn(DeepScanProgress) + Send + Sync)>,
) {
    let Some(on_progress) = on_progress else {
        return;
    };
    let percent = if total_items == 0 {
        if finished {
            100
        } else {
            0
        }
    } else {
        ((processed_items as f64 / total_items as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8
    };

    on_progress(DeepScanProgress {
        path: path.to_string(),
        current_path: current_path.map(ToString::to_string),
        processed_items,
        total_items,
        percent,
        canceled,
        finished,
    });
}

fn merge_warnings_summary(target: &mut DeepScanWarningsSummary, incoming: DeepScanWarningsSummary) {
    target.permission_denied += incoming.permission_denied;
    target.operation_not_permitted += incoming.operation_not_permitted;
    target.vanished_paths += incoming.vanished_paths;
    for sample in incoming.samples {
        push_sample(&mut target.samples, &sample);
    }
    for error in incoming.unexpected_errors {
        push_sample(&mut target.unexpected_errors, &error);
    }
}

fn run_du_target_scan(scan_path: &str, elevated: bool) -> Result<CommandOutput, CommandError> {
    if !elevated {
        return run_partial_with_cancel(
            "du",
            &["-x", "-k", "-s", scan_path],
            Some(&DEEP_SCAN_CANCEL),
        );
    }

    let shell_script = format!("du -x -k -s {}", shell_quote(scan_path));
    let apple_script = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&shell_script)
    );
    run_partial_with_cancel(
        "osascript",
        &["-e", apple_script.as_str()],
        Some(&DEEP_SCAN_CANCEL),
    )
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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

    fn test_volume(identifier: &str, mount_point: Option<&str>) -> VolumeInfo {
        VolumeInfo {
            name: identifier.to_string(),
            identifier: identifier.to_string(),
            role: None,
            mount_point: mount_point.map(ToString::to_string),
            mounted: mount_point.is_some(),
            encrypted: None,
            locked: None,
            flags: Vec::new(),
            capacity_bytes: None,
            used_bytes: None,
            available_bytes: None,
            risk: RiskLevel::SafeToAnalyze,
            notes: Vec::new(),
        }
    }

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
    fn parses_diskutil_info_mount_point() {
        let output = "Device Identifier:         disk4s2\n\
             Volume Name:               External\n\
             Mount Point:               /Volumes/External";
        assert_eq!(
            parse_diskutil_info_mount_point(output).as_deref(),
            Some("/Volumes/External")
        );
        assert_eq!(
            parse_diskutil_info_mount_point("Mount Point:               Not Mounted"),
            None
        );
    }

    #[test]
    fn finds_nearest_mount_for_nested_runtime_path() {
        let volumes = vec![
            test_volume("/dev/disk3s5", Some("/System/Volumes/Data")),
            test_volume(
                "/dev/disk9s1",
                Some("/Library/Developer/CoreSimulator/Cryptex/Images/bundle/Runtime"),
            ),
        ];
        let nested_path = "/Library/Developer/CoreSimulator/Cryptex/Images/bundle/Runtime/System";
        let volume = find_volume_for_identifier(&volumes, nested_path).unwrap();

        assert_eq!(volume.identifier, "/dev/disk9s1");
        assert_eq!(
            volume_operation_target(Some(volume), nested_path, VolumeOperation::Unmount),
            "/Library/Developer/CoreSimulator/Cryptex/Images/bundle/Runtime"
        );
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
