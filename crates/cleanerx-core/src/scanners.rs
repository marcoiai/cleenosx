use crate::classify::{classify_path, finding_for_path};
use crate::command::{log_command_error, run};
use crate::models::{
    Finding, Overview, RiskLevel, ScanLog, ScanResult, StorageCategory, StorageSummary, UsageNode,
    VolumeInfo,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

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

pub fn scan_overview() -> ScanResult<Overview> {
    let mut logs = vec![ScanLog::info("Starting read-only overview scan.")];

    let volume_scan = scan_volumes();
    logs.extend(volume_scan.logs);

    let data_scan = scan_data_usage();
    logs.extend(data_scan.logs);

    let assets_scan = scan_assets_v2();
    logs.extend(assets_scan.logs);

    let snapshots_scan = list_snapshots();
    logs.extend(snapshots_scan.logs);

    let mut findings = assets_scan.data;
    findings.extend(snapshots_scan.data);
    findings.extend(summary_warnings(&volume_scan.data));

    let summary = summarize_volumes(&volume_scan.data);

    ScanResult {
        data: Overview {
            summary,
            volumes: volume_scan.data,
            usage_roots: data_scan.data,
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

pub fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    let mut logs = vec![ScanLog::info(format!(
        "Scanning large blocks at {DATA_VOLUME_PATH}."
    ))];
    let data = scan_usage_path(DATA_VOLUME_PATH, &mut logs);

    ScanResult { data, logs }
}

pub fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    let home = env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let mut logs = vec![ScanLog::info(format!("Scanning user blocks at {home}."))];
    let data = scan_usage_path(&home, &mut logs);

    ScanResult { data, logs }
}

pub fn scan_assets_v2() -> ScanResult<Vec<Finding>> {
    let mut logs = vec![ScanLog::info("Scanning AssetsV2 known targets.")];
    let mut findings = Vec::new();

    for root in ASSETS_V2_PATHS {
        if !Path::new(root).exists() {
            continue;
        }

        let flags = flags_for_path(root, &mut logs);
        let restricted = flags.iter().any(|flag| flag == "restricted");
        let root_size = disk_usage_bytes(root, &mut logs);
        findings.push(Finding {
            title: "AssetsV2 root detected".to_string(),
            path: Some((*root).to_string()),
            size_bytes: root_size,
            category: StorageCategory::AssetsV2,
            risk: if restricted {
                RiskLevel::ReadOnlySystem
            } else {
                RiskLevel::SafeToAnalyze
            },
            reason: if restricted {
                "AssetsV2 root has restricted flag; normal boot should not force changes."
                    .to_string()
            } else {
                "AssetsV2 can contain update and simulator assets; inspect subdirectories only."
                    .to_string()
            },
            recommended_action: "Review subdirectories. Never delete AssetsV2 as a whole."
                .to_string(),
            destructive: false,
        });

        for target in ASSETS_V2_TARGETS {
            let path = format!("{root}/{target}");
            if Path::new(&path).exists() {
                let size_bytes = disk_usage_bytes(&path, &mut logs);
                findings.push(Finding {
                    title: format!("Known MobileAsset: {target}"),
                    path: Some(path),
                    size_bytes,
                    category: StorageCategory::AssetsV2,
                    risk: RiskLevel::ReviewRequired,
                    reason: "Known large MobileAsset class. Cleanup requires explicit review."
                        .to_string(),
                    recommended_action: "MVP: review only. Later: select exact assets and confirm."
                        .to_string(),
                    destructive: false,
                });
            }
        }
    }

    if findings.is_empty() {
        logs.push(ScanLog::info(
            "No AssetsV2 roots were found at known paths.",
        ));
    }

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

    for path in paths {
        if Path::new(&path).exists() {
            findings.push(finding_for_path(
                "Rust storage detected",
                &path,
                disk_usage_bytes(&path, &mut logs),
            ));
        }
    }

    ScanResult {
        data: findings,
        logs,
    }
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
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 6 {
        return None;
    }

    let total_bytes = parse_kib(parts.get(1)?);
    let used_bytes = parse_kib(parts.get(2)?);
    let available_bytes = parse_kib(parts.get(3)?);
    let mount_point = if parts.len() >= 9 {
        parts[8..].join(" ")
    } else {
        parts.last()?.to_string()
    };
    let name = if mount_point == "/" {
        "System".to_string()
    } else {
        mount_point
            .split('/')
            .filter(|part| !part.is_empty())
            .last()
            .unwrap_or(parts[0])
            .to_string()
    };
    let flags = Vec::new();
    let (_, risk, _) = classify_path(&mount_point, &flags);

    Some(VolumeInfo {
        name,
        identifier: parts[0].to_string(),
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

fn scan_usage_path(path: &str, logs: &mut Vec<ScanLog>) -> Vec<UsageNode> {
    if !Path::new(path).exists() {
        logs.push(ScanLog::warning(format!(
            "{path} does not exist or is not mounted."
        )));
        return Vec::new();
    }

    let output = match run("du", &["-xkd", "1", path]) {
        Ok(output) => output.stdout,
        Err(error) => {
            logs.push(log_command_error(
                &format!("du scan failed for {path}"),
                &error,
            ));
            return Vec::new();
        }
    };

    let mut nodes = output
        .lines()
        .filter_map(|line| parse_du_line(line, logs))
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    nodes
}

fn parse_du_line(line: &str, logs: &mut Vec<ScanLog>) -> Option<UsageNode> {
    let mut parts = line.split_whitespace();
    let size_kib = parts.next()?;
    let path = parts.collect::<Vec<_>>().join(" ");
    let path = path.trim();
    let size_bytes = size_kib.parse::<u64>().ok()?.saturating_mul(1024);
    let flags = flags_for_path(path, logs);
    let (category, risk, _) = classify_path(path, &flags);

    Some(UsageNode {
        path: path.to_string(),
        size_bytes,
        category,
        risk,
        flags,
        children: Vec::new(),
    })
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
            });
        }
    }

    findings
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
}
