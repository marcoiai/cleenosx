use crate::classify::action_profile_for_cleanup_target;
use crate::models::{
    CleanupExecution, CleanupItemOutcome, CleanupOutcome, CleanupSelection, CleanupSettings,
    PreparedCleanupItem, PreparedCleanupPlan, RiskLevel, ScanLog, ScanResult, UsageNode,
};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

static ALLOWLIST: OnceLock<Mutex<HashMap<String, PreparedCleanupItem>>> = OnceLock::new();
static PREPARED_PLANS: OnceLock<Mutex<HashMap<String, PreparedCleanupPlan>>> = OnceLock::new();
static ROOT_CONTINUATIONS: OnceLock<Mutex<HashMap<String, Vec<PreparedCleanupItem>>>> =
    OnceLock::new();
static CLEANUP_SETTINGS: OnceLock<Mutex<CleanupSettings>> = OnceLock::new();

pub fn cleanup_candidate_id(path: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("cx-{hash:016x}")
}

pub fn register_usage_nodes(nodes: &[UsageNode]) {
    let mut allowlist = ALLOWLIST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup allowlist poisoned");

    for node in nodes {
        register_node(&mut allowlist, node);
    }
}

pub fn cleanup_settings() -> CleanupSettings {
    CLEANUP_SETTINGS
        .get_or_init(|| Mutex::new(CleanupSettings::default()))
        .lock()
        .expect("cleanup settings poisoned")
        .clone()
}

pub fn update_cleanup_settings(settings: CleanupSettings) -> CleanupSettings {
    let mut current = CLEANUP_SETTINGS
        .get_or_init(|| Mutex::new(CleanupSettings::default()))
        .lock()
        .expect("cleanup settings poisoned");
    *current = settings;
    current.clone()
}

pub fn prepare_cleanup_plan(selection: CleanupSelection) -> ScanResult<PreparedCleanupPlan> {
    let mut logs = vec![ScanLog::info(
        "Preparing cleanup plan from selected item ids.",
    )];
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let allowlist = ALLOWLIST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup allowlist poisoned");

    let mut items = Vec::new();
    for id in selection.item_ids {
        if !seen.insert(id.clone()) {
            continue;
        }

        let Some(candidate) = allowlist.get(&id) else {
            warnings.push(format!(
                "Skipped unknown cleanup id {id}. Rescan before selecting it again."
            ));
            continue;
        };

        if let Some(reason) = validate_candidate(candidate) {
            warnings.push(format!("Skipped {}: {reason}", candidate.path));
            continue;
        }

        let mut item = candidate.clone();
        if let Ok(metadata) = fs::symlink_metadata(&item.path) {
            if metadata.is_file() {
                item.estimated_bytes = metadata.len();
            }
        } else {
            warnings.push(format!("Skipped {}: path no longer exists.", item.path));
            continue;
        }
        items.push(item);
    }
    drop(allowlist);

    if items.is_empty() {
        warnings.push("No selected item is currently eligible for cleanup.".to_string());
    }

    let estimated_recoverable_bytes = items
        .iter()
        .map(|item| item.estimated_bytes)
        .fold(0_u64, u64::saturating_add);
    let plan_id = plan_id(&items);
    let final_confirmation_phrase = format!("DELETE {}", items.len());
    let plan = PreparedCleanupPlan {
        plan_id: plan_id.clone(),
        items,
        estimated_recoverable_bytes,
        warnings,
        final_confirmation_phrase,
    };

    PREPARED_PLANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup prepared plans poisoned")
        .insert(plan_id, plan.clone());

    logs.push(ScanLog::info(format!(
        "Prepared cleanup plan with {} eligible item(s).",
        plan.items.len()
    )));
    for warning in &plan.warnings {
        logs.push(ScanLog::warning(warning.clone()));
    }

    ScanResult { data: plan, logs }
}

pub fn execute_cleanup_plan(execution: CleanupExecution) -> ScanResult<CleanupOutcome> {
    let mut logs = vec![ScanLog::info("Final cleanup confirmation received.")];
    let Some(plan) = PREPARED_PLANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup prepared plans poisoned")
        .get(&execution.plan_id)
        .cloned()
    else {
        return cleanup_refused("Cleanup plan was not found. Prepare the plan again.", logs);
    };

    if execution.final_confirmation != plan.final_confirmation_phrase {
        return cleanup_refused(
            "Final confirmation phrase did not match. Nothing was removed.",
            logs,
        );
    }

    if execution.elevated {
        let result = execute_cleanup_plan_elevated(&plan, logs);
        PREPARED_PLANS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("cleanup prepared plans poisoned")
            .remove(&execution.plan_id);
        return result;
    }

    let allowlist = ALLOWLIST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup allowlist poisoned");

    let mut deleted_bytes = 0_u64;
    let mut removed_items = Vec::new();
    let mut failed_items = Vec::new();
    let mut needs_root = false;

    for item in &plan.items {
        let Some(current) = allowlist.get(&item.id) else {
            let message = "no longer in the backend allowlist".to_string();
            logs.push(ScanLog::error(format!("Refused {}: {message}", item.path)));
            failed_items.push(CleanupItemOutcome {
                path: item.path.clone(),
                message,
                needs_root: false,
            });
            continue;
        };

        if current.path != item.path {
            let message = "allowlist target changed".to_string();
            logs.push(ScanLog::error(format!("Refused {}: {message}", item.path)));
            failed_items.push(CleanupItemOutcome {
                path: item.path.clone(),
                message,
                needs_root: false,
            });
            continue;
        }

        if let Some(reason) = validate_candidate(current) {
            logs.push(ScanLog::error(format!("Refused {}: {reason}", item.path)));
            failed_items.push(CleanupItemOutcome {
                path: item.path.clone(),
                message: reason,
                needs_root: false,
            });
            continue;
        }

        match remove_item(&item.path) {
            Ok(()) => {
                deleted_bytes = deleted_bytes.saturating_add(item.estimated_bytes);
                logs.push(ScanLog::info(format!("Removed {}", item.path)));
                removed_items.push(CleanupItemOutcome {
                    path: item.path.clone(),
                    message: "removed".to_string(),
                    needs_root: false,
                });
            }
            Err(error) => {
                let root_required = cleanup_error_needs_root(&error);
                needs_root |= root_required;
                let message = if root_required {
                    format!("{error}; root or Recovery cleanup required")
                } else {
                    error.to_string()
                };
                logs.push(ScanLog::error(format!(
                    "Failed to remove {}: {message}",
                    item.path
                )));
                failed_items.push(CleanupItemOutcome {
                    path: item.path.clone(),
                    message,
                    needs_root: root_required,
                });
            }
        }
    }
    drop(allowlist);

    PREPARED_PLANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup prepared plans poisoned")
        .remove(&execution.plan_id);

    let root_continuation_id = if needs_root {
        let root_items = plan
            .items
            .iter()
            .filter(|item| {
                failed_items
                    .iter()
                    .any(|failed| failed.path == item.path && failed.needs_root)
            })
            .cloned()
            .collect::<Vec<_>>();
        if root_items.is_empty() {
            None
        } else {
            let id = format!("root-{}", plan_id(&root_items));
            ROOT_CONTINUATIONS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("cleanup root continuations poisoned")
                .insert(id.clone(), root_items);
            Some(id)
        }
    } else {
        None
    };

    ScanResult {
        data: CleanupOutcome {
            dry_run: false,
            deleted_bytes,
            message: format!(
                "Cleanup finished. Removed {} item(s), failed {} item(s).",
                removed_items.len(),
                failed_items.len()
            ),
            removed_items,
            failed_items,
            needs_root,
            root_continuation_id,
        },
        logs,
    }
}

fn execute_cleanup_plan_elevated(
    plan: &PreparedCleanupPlan,
    mut logs: Vec<ScanLog>,
) -> ScanResult<CleanupOutcome> {
    logs.push(ScanLog::info(
        "Elevated cleanup requested. Building admin shell script.",
    ));

    let mut shell_script = String::from("set -e\n");
    let mut eligible_items = Vec::new();

    for item in &plan.items {
        if let Some(reason) = validate_candidate(item) {
            logs.push(ScanLog::error(format!("Refused {}: {reason}", item.path)));
            continue;
        }
        if is_broad_target(&item.path) || !is_normal_absolute_path(&item.path) {
            logs.push(ScanLog::error(format!(
                "Refused {}: target is not a normal exact cleanup path.",
                item.path
            )));
            continue;
        }
        shell_script.push_str("rm -rf -- ");
        shell_script.push_str(&shell_quote(&item.path));
        shell_script.push('\n');
        eligible_items.push(item.clone());
    }

    if eligible_items.is_empty() {
        return cleanup_refused("No elevated cleanup targets are currently eligible.", logs);
    }

    let status = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script {} with administrator privileges",
            applescript_quote(&shell_script)
        ))
        .status();

    match status {
        Ok(status) if status.success() => {
            let deleted_bytes = eligible_items
                .iter()
                .map(|item| item.estimated_bytes)
                .fold(0_u64, u64::saturating_add);
            let removed_items = eligible_items
                .iter()
                .map(|item| CleanupItemOutcome {
                    path: item.path.clone(),
                    message: "removed with admin privileges".to_string(),
                    needs_root: false,
                })
                .collect::<Vec<_>>();
            logs.push(ScanLog::info(format!(
                "Elevated cleanup removed {} item(s).",
                removed_items.len()
            )));
            ScanResult {
                data: CleanupOutcome {
                    dry_run: false,
                    deleted_bytes,
                    message: format!(
                        "Elevated cleanup finished. Removed {} item(s), failed 0 item(s).",
                        removed_items.len()
                    ),
                    removed_items,
                    failed_items: Vec::new(),
                    needs_root: false,
                    root_continuation_id: None,
                },
                logs,
            }
        }
        Ok(status) => cleanup_refused(
            &format!(
                "Elevated cleanup did not complete. osascript exited with status {status}."
            ),
            logs,
        ),
        Err(error) => cleanup_refused(
            &format!("Failed to request elevated cleanup through osascript: {error}"),
            logs,
        ),
    }
}

pub fn execute_root_cleanup_continuation(continuation_id: String) -> ScanResult<CleanupOutcome> {
    let mut logs = vec![ScanLog::info(
        "Admin cleanup continuation requested for permission-blocked targets.",
    )];
    let Some(items) = ROOT_CONTINUATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("cleanup root continuations poisoned")
        .get(&continuation_id)
        .cloned()
    else {
        return cleanup_refused(
            "Root cleanup continuation was not found. Run normal cleanup again.",
            logs,
        );
    };

    let mut shell_script = String::from("set -e\n");
    let mut eligible_items = Vec::new();
    for item in &items {
        if let Some(reason) = validate_candidate(item) {
            logs.push(ScanLog::error(format!("Refused {}: {reason}", item.path)));
            continue;
        }
        if is_broad_target(&item.path) || !is_normal_absolute_path(&item.path) {
            logs.push(ScanLog::error(format!(
                "Refused {}: target is not a normal exact cleanup path.",
                item.path
            )));
            continue;
        }
        shell_script.push_str("rm -rf -- ");
        shell_script.push_str(&shell_quote(&item.path));
        shell_script.push('\n');
        eligible_items.push(item.clone());
    }

    if eligible_items.is_empty() {
        return cleanup_refused("No root cleanup targets are currently eligible.", logs);
    }

    let status = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script {} with administrator privileges",
            applescript_quote(&shell_script)
        ))
        .status();

    match status {
        Ok(status) if status.success() => {
            ROOT_CONTINUATIONS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("cleanup root continuations poisoned")
                .remove(&continuation_id);
            let deleted_bytes = eligible_items
                .iter()
                .map(|item| item.estimated_bytes)
                .fold(0_u64, u64::saturating_add);
            let removed_items = eligible_items
                .iter()
                .map(|item| CleanupItemOutcome {
                    path: item.path.clone(),
                    message: "removed with admin privileges".to_string(),
                    needs_root: false,
                })
                .collect::<Vec<_>>();
            logs.push(ScanLog::info(format!(
                "Admin cleanup removed {} item(s).",
                removed_items.len()
            )));
            ScanResult {
                data: CleanupOutcome {
                    dry_run: false,
                    deleted_bytes,
                    message: format!(
                        "Admin cleanup finished. Removed {} item(s), failed 0 item(s).",
                        removed_items.len()
                    ),
                    removed_items,
                    failed_items: Vec::new(),
                    needs_root: false,
                    root_continuation_id: None,
                },
                logs,
            }
        }
        Ok(status) => cleanup_refused(
            &format!("Admin cleanup did not complete. osascript exited with status {status}."),
            logs,
        ),
        Err(error) => cleanup_refused(
            &format!("Failed to request admin cleanup through osascript: {error}"),
            logs,
        ),
    }
}

fn register_node(allowlist: &mut HashMap<String, PreparedCleanupItem>, node: &UsageNode) {
    allowlist.insert(
        node.id.clone(),
        PreparedCleanupItem {
            id: node.id.clone(),
            path: node.path.clone(),
            kind: node.kind.clone(),
            category: node.category.clone(),
            risk: node.risk.clone(),
            estimated_bytes: node.size_bytes,
            reason: format!("{:?} cleanup target", node.category),
            action: cleanup_action_for_risk(&node.risk).to_string(),
            action_profile: Some(action_profile_for_cleanup_target(
                &node.path,
                &node.category,
                &node.risk,
            )),
        },
    );
    for child in &node.children {
        register_node(allowlist, child);
    }
}

fn cleanup_action_for_risk(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::SafeToAnalyze => "removeAfterFinalConfirmation",
        RiskLevel::Attention => "removeAfterFinalConfirmation",
        RiskLevel::ReviewRequired => "removeAfterFinalConfirmation",
        RiskLevel::Dangerous | RiskLevel::ReadOnlySystem => "refuse",
    }
}

fn validate_candidate(item: &PreparedCleanupItem) -> Option<String> {
    if item.risk == RiskLevel::ReadOnlySystem {
        return Some("risk level is not eligible for automated cleanup".to_string());
    }
    if item.risk == RiskLevel::Dangerous
        && !(is_project_path(&item.path) && cleanup_settings().allow_project_root_cleanup)
    {
        return Some("risk level is not eligible for automated cleanup".to_string());
    }
    if is_read_only_system_area(&item.path) {
        return Some("path is in a read-only system/runtime area".to_string());
    }
    if is_broad_target(&item.path) {
        return Some("broad/system targets must be drilled into first".to_string());
    }
    if !is_normal_absolute_path(&item.path) {
        return Some("path is not a normal absolute path".to_string());
    }
    None
}

fn is_project_path(path: &str) -> bool {
    path.contains("/Projects")
}

fn is_read_only_system_area(path: &str) -> bool {
    path.starts_with("/Library/Developer/CoreSimulator/Cryptex/")
        || path.starts_with("/Library/Developer/CoreSimulator/Volumes/")
        || path == "/System/Library/AssetsV2"
        || path.starts_with("/System/Library/AssetsV2/")
        || path == "/System/Volumes/Data/System/Library/AssetsV2"
        || path.starts_with("/System/Volumes/Data/System/Library/AssetsV2/")
        || path.starts_with("/System/Cryptexes/")
        || path.starts_with("/System/Volumes/Preboot/")
}

fn is_normal_absolute_path(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir))
}

fn is_broad_target(path: &str) -> bool {
    matches!(
        path,
        "/" | "/System"
            | "/Library"
            | "/Applications"
            | "/Users"
            | "/tmp"
            | "/private/tmp"
            | "/opt"
            | "/opt/homebrew"
            | "/System/Volumes"
            | "/System/Volumes/Data"
            | "/System/Library/AssetsV2"
            | "/System/Volumes/Data/System/Library/AssetsV2"
    )
}

fn remove_item(path: &str) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn cleanup_refused(message: &str, mut logs: Vec<ScanLog>) -> ScanResult<CleanupOutcome> {
    logs.push(ScanLog::error(message.to_string()));
    ScanResult {
        data: CleanupOutcome {
            dry_run: true,
            deleted_bytes: 0,
            message: message.to_string(),
            removed_items: Vec::new(),
            failed_items: Vec::new(),
            needs_root: false,
            root_continuation_id: None,
        },
        logs,
    }
}

fn cleanup_error_needs_root(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
        || matches!(error.raw_os_error(), Some(1 | 13 | 66))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn plan_id(items: &[PreparedCleanupItem]) -> String {
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
        .to_string();
    for item in items {
        seed.push_str(&item.id);
    }
    format!("plan-{}", cleanup_candidate_id(&seed))
}
