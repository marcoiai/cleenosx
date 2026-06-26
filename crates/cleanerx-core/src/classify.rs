use crate::models::{
    ActionProfile, ActionRecommendation, ActionScores, ActionStatus, ActionType, ActionUi,
    DeleteCapability, Finding, RiskLevel, StorageCategory,
};

pub fn classify_path(path: &str, flags: &[String]) -> (StorageCategory, RiskLevel, String) {
    let normalized = path.replace('\\', "/");
    let restricted = flags.iter().any(|flag| flag == "restricted");

    if is_assets_v2_area(&normalized) {
        return (
            StorageCategory::AssetsV2,
            RiskLevel::ReadOnlySystem,
            "AssetsV2 is protected in normal boot; inspect only or use Apple tooling/Recovery."
                .to_string(),
        );
    }

    if restricted {
        return (
            StorageCategory::MacOsApfs,
            RiskLevel::ReadOnlySystem,
            "Path has the restricted flag; normal boot must not force changes.".to_string(),
        );
    }

    if normalized.starts_with("/Library/Developer/CoreSimulator/Cryptex/")
        || normalized.starts_with("/Library/Developer/CoreSimulator/Volumes/")
        || normalized.starts_with("/System/Cryptexes/")
        || normalized.starts_with("/System/Volumes/Preboot/")
    {
        return (
            StorageCategory::MacOsApfs,
            RiskLevel::ReadOnlySystem,
            "Read-only macOS runtime area; remove runtimes with Apple tooling or from Recovery only."
                .to_string(),
        );
    }

    if normalized == "/System"
        || normalized == "/Library"
        || normalized == "/opt"
        || normalized == "/System/Volumes/Data"
    {
        return (
            StorageCategory::MacOsApfs,
            RiskLevel::SafeToAnalyze,
            "Top-level/system location is safe to inspect but not safe to delete as a whole."
                .to_string(),
        );
    }

    if normalized.contains("/AssetsV2/") {
        let category = StorageCategory::AssetsV2;
        let risk = if normalized.contains("MacSoftwareUpdate")
            || normalized.contains("iOSSimulatorRuntime")
            || normalized.contains("watchOSSimulatorRuntime")
            || normalized.contains("appleTVOSSimulatorRuntime")
            || normalized.contains("xrOSSimulatorRuntime")
        {
            RiskLevel::ReviewRequired
        } else {
            RiskLevel::Attention
        };

        return (
            category,
            risk,
            "Known MobileAsset area; inspect exact asset before any cleanup.".to_string(),
        );
    }

    if normalized.ends_with("/target") || normalized.contains("/target/") {
        return (
            StorageCategory::RustArtifacts,
            RiskLevel::Attention,
            "Rust target build artifacts can usually be recreated by Cargo after cleanup."
                .to_string(),
        );
    }

    if normalized.contains("/CoreSimulator")
        || normalized.contains("iOS.simruntime")
        || normalized.contains("watchOS.simruntime")
        || normalized.contains("tvOS.simruntime")
        || normalized.contains("xrOS.simruntime")
    {
        return (
            StorageCategory::Simulators,
            RiskLevel::ReviewRequired,
            "Simulator runtimes can be large; remove only selected unused runtimes.".to_string(),
        );
    }

    if normalized.contains("/.cargo") || normalized.contains("/.rustup") {
        return (
            StorageCategory::RustArtifacts,
            RiskLevel::Attention,
            "Rust toolchain/cache area; review toolchain usage before cleanup.".to_string(),
        );
    }

    if normalized.contains("/node_modules")
        || normalized.contains("/.npm")
        || normalized.contains("/.pnpm-store")
        || normalized.contains("/.yarn")
    {
        return (
            StorageCategory::NodeCaches,
            RiskLevel::Attention,
            "JavaScript dependency/cache area; can often be recreated by package managers."
                .to_string(),
        );
    }

    if normalized.contains("/Library/Caches") || normalized.contains("/.cache") {
        return (
            StorageCategory::Caches,
            RiskLevel::Attention,
            "Cache directory; cleanup should still be explicit and app-aware.".to_string(),
        );
    }

    if normalized.contains("/Containers")
        || normalized.contains("/Docker")
        || normalized.contains("/OrbStack")
        || normalized.contains("/podman")
    {
        return (
            StorageCategory::Containers,
            RiskLevel::ReviewRequired,
            "Container storage should be cleaned with the owning tool when possible.".to_string(),
        );
    }

    if normalized.contains("/Homebrew") || normalized.starts_with("/opt/homebrew") {
        return (
            StorageCategory::Homebrew,
            RiskLevel::Attention,
            "Homebrew area; prefer brew cleanup commands over manual deletion.".to_string(),
        );
    }

    if normalized.contains("/Projects") {
        return (
            StorageCategory::Projects,
            RiskLevel::Dangerous,
            "Project/source folders must never be deleted automatically.".to_string(),
        );
    }

    if normalized.contains("/Users/") {
        return (
            StorageCategory::UserData,
            RiskLevel::SafeToAnalyze,
            "User area; inspect only unless a known cache/build artifact is selected.".to_string(),
        );
    }

    (
        StorageCategory::Unknown,
        RiskLevel::SafeToAnalyze,
        "Unknown block; inspect before making any recommendation.".to_string(),
    )
}

fn is_assets_v2_area(path: &str) -> bool {
    const ROOTS: [&str; 2] = [
        "/System/Library/AssetsV2",
        "/System/Volumes/Data/System/Library/AssetsV2",
    ];

    ROOTS
        .iter()
        .any(|root| path == *root || path.starts_with(&format!("{root}/")))
}

pub fn finding_for_path(title: &str, path: &str, size_bytes: Option<u64>) -> Finding {
    let (category, risk, reason) = classify_path(path, &[]);
    let recommended_action = recommended_action_for(&category, &risk).to_string();
    let action_profile = action_profile_for_finding(Some(path), &category, &risk);
    Finding {
        title: title.to_string(),
        path: Some(path.to_string()),
        size_bytes,
        category,
        risk,
        reason,
        recommended_action,
        destructive: false,
        action_profile: Some(action_profile),
    }
}

fn recommended_action_for(category: &StorageCategory, risk: &RiskLevel) -> &'static str {
    match (category, risk) {
        (&StorageCategory::RustArtifacts, &RiskLevel::Attention) => {
            "Good cleanup candidate after user confirmation. Cargo can rebuild it later."
        }
        (_, &RiskLevel::Dangerous) | (_, &RiskLevel::ReadOnlySystem) => {
            "Do not remove from CleanerX."
        }
        (_, &RiskLevel::ReviewRequired) => "Review exact contents before selecting for cleanup.",
        _ => "Review path before selecting for cleanup.",
    }
}

pub fn action_profile_for_finding(
    path: Option<&str>,
    category: &StorageCategory,
    risk: &RiskLevel,
) -> ActionProfile {
    let badge = badge_for(category).to_string();
    let requires_sudo = path.is_some_and(requires_sudo_for_path);

    match *category {
        StorageCategory::AssetsV2 => ActionProfile {
            status: ActionStatus::Rejected,
            action_type: ActionType::MeasureOnly,
            deletes_files: false,
            command: path.map(|value| format!("du -sk {value}")),
            requires_sudo,
            scores: ActionScores {
                safety_percent: 5,
                reclaim_value_percent: 54,
                automation_percent: 0,
                confidence_percent: 96,
            },
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: "Do not delete".to_string(),
                user_facing_summary: "CleanerX should only measure and explain this space. macOS manages this content.".to_string(),
                technical_reason: "AssetsV2 is system-protected storage and should not be removed from a normal boot session.".to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: 82,
                primary_action: "View details".to_string(),
                secondary_action: Some("Review updates".to_string()),
                explain_like_user: "This space usually holds macOS-managed components such as runtimes and updates. It is worth understanding, not force-cleaning.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some("Review which updates or Xcode runtimes still need to stay installed.".to_string()),
            },
        },
        StorageCategory::Snapshots => ActionProfile {
            status: ActionStatus::Candidate,
            action_type: ActionType::ListOnly,
            deletes_files: false,
            command: Some("tmutil listlocalsnapshots /".to_string()),
            requires_sudo: false,
            scores: ActionScores {
                safety_percent: 28,
                reclaim_value_percent: 71,
                automation_percent: 0,
                confidence_percent: 87,
            },
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: "Investigate only".to_string(),
                user_facing_summary: "Local snapshots may be worth reviewing, but the app does not clean them yet.".to_string(),
                technical_reason: "Snapshot thinning remains disabled in the MVP flow.".to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: 64,
                primary_action: "List snapshots".to_string(),
                secondary_action: Some("View details".to_string()),
                explain_like_user: "Local snapshots can temporarily use a lot of space. Right now the app only shows that they exist.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some("Use the list to decide whether a dedicated snapshot workflow is worth adding.".to_string()),
            },
        },
        StorageCategory::RustArtifacts | StorageCategory::NodeCaches | StorageCategory::Caches => {
            ActionProfile {
                status: ActionStatus::Candidate,
                action_type: match *category {
                    StorageCategory::RustArtifacts => ActionType::OpenFolder,
                    _ => ActionType::PurgeCache,
                },
                deletes_files: false,
                command: path.map(|value| format!("du -sk {value}")),
                requires_sudo,
                scores: diagnostic_scores_for(category),
                delete_capability: DeleteCapability {
                    can_delete: false,
                    user_facing_level: "Investigate only".to_string(),
                    user_facing_summary: "This kind of item is often a good cleanup candidate, but the exact path should be reviewed first.".to_string(),
                    technical_reason: "Caches and rebuildable artifacts are safest once the exact target has already been reviewed.".to_string(),
                },
                ui: ActionUi {
                    badge,
                    severity_percent: 34,
                    primary_action: "Open in scanner".to_string(),
                    secondary_action: Some("Review before cleanup".to_string()),
                    explain_like_user: "This is usually where build leftovers, caches, or temporary dependencies live. It can free space without touching personal files, but the exact item still matters.".to_string(),
                },
                recommendation: ActionRecommendation {
                    include_in_app: true,
                    include_as_cleanup: true,
                    include_as_diagnostic: true,
                    next_action: Some("Drill down to the exact item before preparing cleanup.".to_string()),
                },
            }
        }
        StorageCategory::Containers
        | StorageCategory::Simulators
        | StorageCategory::Homebrew
        | StorageCategory::DeveloperTools => ActionProfile {
            status: ActionStatus::Candidate,
            action_type: ActionType::Advisory,
            deletes_files: false,
            command: path.map(|value| format!("du -sk {value}")),
            requires_sudo,
            scores: diagnostic_scores_for(category),
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: "Investigate only".to_string(),
                user_facing_summary: "It may take a lot of space, but it is usually safer to clean it from the owning tool.".to_string(),
                technical_reason: category_cleanup_reason(category).to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: 58,
                primary_action: "View details".to_string(),
                secondary_action: Some("Open in scanner".to_string()),
                explain_like_user: "This space usually belongs to Xcode, Docker, Homebrew, or similar tooling. Cleaning from the original app is often safer.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some("Use the path detail to decide whether cleanup should happen in the app that owns this data.".to_string()),
            },
        },
        StorageCategory::Projects | StorageCategory::UserData => ActionProfile {
            status: ActionStatus::Rejected,
            action_type: ActionType::Advisory,
            deletes_files: false,
            command: path.map(|value| format!("du -sk {value}")),
            requires_sudo: false,
            scores: diagnostic_scores_for(category),
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: "Do not delete".to_string(),
                user_facing_summary: "This path looks like a project or user-owned data. The app should help explain it, not remove it blindly.".to_string(),
                technical_reason: "Projects and user data may contain source code, documents, or state that is hard to recover.".to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: 76,
                primary_action: "View details".to_string(),
                secondary_action: Some("Open in scanner".to_string()),
                explain_like_user: "This kind of folder may hold important work. Before thinking about cleanup, confirm exactly what is inside.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some("Look for caches or build folders inside the project, not the whole project itself.".to_string()),
            },
        },
        _ => ActionProfile {
            status: if matches!(*risk, RiskLevel::ReadOnlySystem | RiskLevel::Dangerous) {
                ActionStatus::Rejected
            } else {
                ActionStatus::Candidate
            },
            action_type: if matches!(*risk, RiskLevel::ReadOnlySystem) {
                ActionType::MeasureOnly
            } else {
                ActionType::Advisory
            },
            deletes_files: false,
            command: path.map(|value| format!("du -sk {value}")),
            requires_sudo,
            scores: diagnostic_scores_for(category),
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: if matches!(*risk, RiskLevel::ReadOnlySystem | RiskLevel::Dangerous) {
                    "Do not delete".to_string()
                } else {
                    "Investigate only".to_string()
                },
                user_facing_summary: "Use this item more for diagnosis than for direct cleanup.".to_string(),
                technical_reason: "There is not a strong enough rule yet to treat this path as cleanup-safe by default.".to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: severity_percent_for_risk(risk),
                primary_action: "View details".to_string(),
                secondary_action: Some("Open in scanner".to_string()),
                explain_like_user: "This space deserves investigation before any decision. The goal here is to understand it, not start deleting.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some("Drill down to a more specific path before considering cleanup.".to_string()),
            },
        },
    }
}

pub fn action_profile_for_cleanup_target(
    path: &str,
    category: &StorageCategory,
    risk: &RiskLevel,
) -> ActionProfile {
    let broad_target = is_broad_target_path(path);
    let approved_category = matches!(
        category,
        StorageCategory::RustArtifacts | StorageCategory::NodeCaches | StorageCategory::Caches
    );
    let requires_sudo = requires_sudo_for_path(path);
    let badge = badge_for(category).to_string();

    if matches!(*risk, RiskLevel::ReadOnlySystem | RiskLevel::Dangerous) || broad_target {
        return ActionProfile {
            status: ActionStatus::Rejected,
            action_type: ActionType::Advisory,
            deletes_files: false,
            command: None,
            requires_sudo,
            scores: cleanup_scores_for(category, risk, false),
            delete_capability: DeleteCapability {
                can_delete: false,
                user_facing_level: if matches!(*risk, RiskLevel::ReadOnlySystem) {
                    "Do not delete".to_string()
                } else {
                    "Investigate only".to_string()
                },
                user_facing_summary: if broad_target {
                    "This target is too broad. Drill down to a more specific item first."
                        .to_string()
                } else {
                    "This item should not enter automated cleanup in the app's current state."
                        .to_string()
                },
                technical_reason: if broad_target {
                    "Broad targets remain blocked to avoid overly wide accidental removals."
                        .to_string()
                } else {
                    "The current risk and category combination is not eligible for automated cleanup.".to_string()
                },
            },
            ui: ActionUi {
                badge,
                severity_percent: severity_percent_for_risk(risk),
                primary_action: "View details".to_string(),
                secondary_action: Some("Open in scanner".to_string()),
                explain_like_user:
                    "The app found this path, but it is not a good direct cleanup target yet."
                        .to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: false,
                include_as_diagnostic: true,
                next_action: Some(
                    "Review a more specific child item or use the tool that owns this content."
                        .to_string(),
                ),
            },
        };
    }

    if approved_category {
        return ActionProfile {
            status: ActionStatus::Approved,
            action_type: match *category {
                StorageCategory::RustArtifacts => ActionType::DeleteFiles,
                _ => ActionType::PurgeCache,
            },
            deletes_files: true,
            command: None,
            requires_sudo,
            scores: cleanup_scores_for(category, risk, true),
            delete_capability: DeleteCapability {
                can_delete: true,
                user_facing_level: if matches!(*category, StorageCategory::RustArtifacts) {
                    "Safe to clean".to_string()
                } else {
                    "Can clean with confirmation".to_string()
                },
                user_facing_summary: match *category {
                    StorageCategory::RustArtifacts => "These are build artifacts or developer caches. Cargo can rebuild them later if needed.".to_string(),
                    StorageCategory::NodeCaches => "These are dependencies or caches that can usually be downloaded or regenerated again.".to_string(),
                    _ => "These are temporary caches and are often good cleanup candidates after one final review.".to_string(),
                },
                technical_reason: category_cleanup_reason(category).to_string(),
            },
            ui: ActionUi {
                badge,
                severity_percent: 26,
                primary_action: "Prepare cleanup".to_string(),
                secondary_action: Some("View details".to_string()),
                explain_like_user: "This item looks rebuildable. Even so, cleanup should only happen after your final confirmation.".to_string(),
            },
            recommendation: ActionRecommendation {
                include_in_app: true,
                include_as_cleanup: true,
                include_as_diagnostic: true,
                next_action: Some("Confirm the exact path and final size before deletion.".to_string()),
            },
        };
    }

    ActionProfile {
        status: ActionStatus::Candidate,
        action_type: ActionType::DeleteFiles,
        deletes_files: true,
        command: None,
        requires_sudo,
        scores: cleanup_scores_for(category, risk, false),
        delete_capability: DeleteCapability {
            can_delete: true,
            user_facing_level: "Can clean with confirmation".to_string(),
            user_facing_summary: "This may free space, but it needs extra care because the path does not look like an obviously rebuildable leftover.".to_string(),
            technical_reason: category_cleanup_reason(category).to_string(),
        },
        ui: ActionUi {
            badge,
            severity_percent: severity_percent_for_risk(risk),
            primary_action: "Prepare cleanup".to_string(),
            secondary_action: Some("View details".to_string()),
            explain_like_user: "This item can be cleaned, but it is not the kind of thing I would treat as an obvious discard. Review it carefully before confirming.".to_string(),
        },
        recommendation: ActionRecommendation {
            include_in_app: true,
            include_as_cleanup: true,
            include_as_diagnostic: true,
            next_action: Some("Review the app or tool that owns this path before final removal.".to_string()),
        },
    }
}

fn diagnostic_scores_for(category: &StorageCategory) -> ActionScores {
    match *category {
        StorageCategory::AssetsV2 => ActionScores {
            safety_percent: 5,
            reclaim_value_percent: 54,
            automation_percent: 0,
            confidence_percent: 96,
        },
        StorageCategory::Snapshots => ActionScores {
            safety_percent: 28,
            reclaim_value_percent: 71,
            automation_percent: 0,
            confidence_percent: 87,
        },
        StorageCategory::RustArtifacts => ActionScores {
            safety_percent: 88,
            reclaim_value_percent: 72,
            automation_percent: 58,
            confidence_percent: 92,
        },
        StorageCategory::NodeCaches => ActionScores {
            safety_percent: 83,
            reclaim_value_percent: 68,
            automation_percent: 52,
            confidence_percent: 88,
        },
        StorageCategory::Caches => ActionScores {
            safety_percent: 78,
            reclaim_value_percent: 60,
            automation_percent: 40,
            confidence_percent: 84,
        },
        StorageCategory::Containers => ActionScores {
            safety_percent: 35,
            reclaim_value_percent: 82,
            automation_percent: 12,
            confidence_percent: 81,
        },
        StorageCategory::Simulators => ActionScores {
            safety_percent: 38,
            reclaim_value_percent: 86,
            automation_percent: 10,
            confidence_percent: 84,
        },
        StorageCategory::Homebrew => ActionScores {
            safety_percent: 44,
            reclaim_value_percent: 58,
            automation_percent: 18,
            confidence_percent: 78,
        },
        StorageCategory::DeveloperTools => ActionScores {
            safety_percent: 32,
            reclaim_value_percent: 74,
            automation_percent: 8,
            confidence_percent: 79,
        },
        StorageCategory::Projects => ActionScores {
            safety_percent: 8,
            reclaim_value_percent: 30,
            automation_percent: 0,
            confidence_percent: 93,
        },
        StorageCategory::UserData => ActionScores {
            safety_percent: 18,
            reclaim_value_percent: 34,
            automation_percent: 0,
            confidence_percent: 76,
        },
        StorageCategory::MacOsApfs => ActionScores {
            safety_percent: 12,
            reclaim_value_percent: 48,
            automation_percent: 0,
            confidence_percent: 90,
        },
        StorageCategory::Updates => ActionScores {
            safety_percent: 14,
            reclaim_value_percent: 50,
            automation_percent: 0,
            confidence_percent: 90,
        },
        StorageCategory::VolumesExtra => ActionScores {
            safety_percent: 20,
            reclaim_value_percent: 56,
            automation_percent: 0,
            confidence_percent: 82,
        },
        StorageCategory::Unknown => ActionScores {
            safety_percent: 30,
            reclaim_value_percent: 40,
            automation_percent: 0,
            confidence_percent: 60,
        },
    }
}

fn cleanup_scores_for(
    category: &StorageCategory,
    risk: &RiskLevel,
    approved_category: bool,
) -> ActionScores {
    if approved_category {
        return match *category {
            StorageCategory::RustArtifacts => ActionScores {
                safety_percent: 90,
                reclaim_value_percent: 74,
                automation_percent: 48,
                confidence_percent: 92,
            },
            StorageCategory::NodeCaches => ActionScores {
                safety_percent: 84,
                reclaim_value_percent: 68,
                automation_percent: 42,
                confidence_percent: 88,
            },
            StorageCategory::Caches => ActionScores {
                safety_percent: 80,
                reclaim_value_percent: 60,
                automation_percent: 35,
                confidence_percent: 84,
            },
            _ => ActionScores {
                safety_percent: 72,
                reclaim_value_percent: 54,
                automation_percent: 24,
                confidence_percent: 75,
            },
        };
    }

    match *risk {
        RiskLevel::SafeToAnalyze => ActionScores {
            safety_percent: 46,
            reclaim_value_percent: 46,
            automation_percent: 12,
            confidence_percent: 64,
        },
        RiskLevel::Attention => ActionScores {
            safety_percent: 58,
            reclaim_value_percent: 55,
            automation_percent: 18,
            confidence_percent: 70,
        },
        RiskLevel::ReviewRequired => ActionScores {
            safety_percent: 34,
            reclaim_value_percent: 72,
            automation_percent: 8,
            confidence_percent: 74,
        },
        RiskLevel::Dangerous => ActionScores {
            safety_percent: 8,
            reclaim_value_percent: 38,
            automation_percent: 0,
            confidence_percent: 90,
        },
        RiskLevel::ReadOnlySystem => ActionScores {
            safety_percent: 4,
            reclaim_value_percent: 35,
            automation_percent: 0,
            confidence_percent: 95,
        },
    }
}

fn badge_for(category: &StorageCategory) -> &'static str {
    match *category {
        StorageCategory::MacOsApfs
        | StorageCategory::AssetsV2
        | StorageCategory::Updates
        | StorageCategory::Snapshots
        | StorageCategory::VolumesExtra => "System",
        StorageCategory::DeveloperTools | StorageCategory::Simulators => "Xcode",
        StorageCategory::RustArtifacts | StorageCategory::Homebrew => "Developer",
        StorageCategory::NodeCaches | StorageCategory::Caches => "Cache",
        StorageCategory::Containers => "Docker",
        StorageCategory::Projects | StorageCategory::UserData => "User",
        StorageCategory::Unknown => "Diagnostic",
    }
}

fn category_cleanup_reason(category: &StorageCategory) -> &'static str {
    match *category {
        StorageCategory::RustArtifacts => {
            "Rust build artifacts can be recreated later by the toolchain."
        }
        StorageCategory::NodeCaches => {
            "JavaScript caches and dependencies can usually be downloaded or rebuilt again."
        }
        StorageCategory::Caches => {
            "Temporary caches can normally be recreated by apps when needed."
        }
        StorageCategory::Containers => {
            "Container layers, volumes, and images may depend on the runtime that owns the data."
        }
        StorageCategory::Simulators => {
            "Simulator runtimes and devices are usually managed more safely by Apple's tooling."
        }
        StorageCategory::Homebrew => {
            "Homebrew formulae, casks, and caches are usually safer to clean through brew itself."
        }
        StorageCategory::DeveloperTools => {
            "SDKs and development components may still be tied to active installations."
        }
        StorageCategory::Projects | StorageCategory::UserData => {
            "This path may hold work, documents, or state that is hard to reconstruct."
        }
        _ => "This path needs manual review before it should be treated as cleanup-safe.",
    }
}

fn severity_percent_for_risk(risk: &RiskLevel) -> u8 {
    match *risk {
        RiskLevel::SafeToAnalyze => 18,
        RiskLevel::Attention => 40,
        RiskLevel::ReviewRequired => 64,
        RiskLevel::Dangerous => 90,
        RiskLevel::ReadOnlySystem => 82,
    }
}

fn requires_sudo_for_path(path: &str) -> bool {
    path == "/System"
        || path.starts_with("/System/")
        || path == "/Library"
        || path.starts_with("/Library/")
        || path == "/Applications"
        || path.starts_with("/Applications/")
        || path == "/opt"
        || path.starts_with("/opt/")
}

fn is_broad_target_path(path: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_rust_artifact_attention() {
        let (category, risk, _) = classify_path("/Users/marco/Projects/app/target", &[]);
        assert_eq!(category, StorageCategory::RustArtifacts);
        assert_eq!(risk, RiskLevel::Attention);
    }

    #[test]
    fn nested_target_is_rust_artifact_before_project_source_rule() {
        let (category, risk, reason) =
            classify_path("/Users/marco/Projects/app/crates/core/target/debug", &[]);
        assert_eq!(category, StorageCategory::RustArtifacts);
        assert_eq!(risk, RiskLevel::Attention);
        assert!(reason.contains("Cargo"));
    }

    #[test]
    fn projects_are_dangerous() {
        let (_, risk, _) = classify_path("/Users/marco/Projects", &[]);
        assert_eq!(risk, RiskLevel::Dangerous);
    }

    #[test]
    fn restricted_is_read_only_system() {
        let (_, risk, _) = classify_path("/System/Library/AssetsV2", &["restricted".to_string()]);
        assert_eq!(risk, RiskLevel::ReadOnlySystem);
    }

    #[test]
    fn rust_cleanup_target_is_approved() {
        let profile = action_profile_for_cleanup_target(
            "/Users/marco/Projects/app/target",
            &StorageCategory::RustArtifacts,
            &RiskLevel::Attention,
        );
        assert_eq!(profile.status, ActionStatus::Approved);
        assert!(profile.delete_capability.can_delete);
        assert_eq!(profile.scores.safety_percent, 90);
    }

    #[test]
    fn assetsv2_finding_is_rejected() {
        let profile = action_profile_for_finding(
            Some("/System/Library/AssetsV2"),
            &StorageCategory::AssetsV2,
            &RiskLevel::ReadOnlySystem,
        );
        assert_eq!(profile.status, ActionStatus::Rejected);
        assert!(!profile.delete_capability.can_delete);
        assert_eq!(profile.action_type, ActionType::MeasureOnly);
    }
}
