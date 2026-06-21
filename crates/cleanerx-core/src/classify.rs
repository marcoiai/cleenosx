use crate::models::{Finding, RiskLevel, StorageCategory};

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
    Finding {
        title: title.to_string(),
        path: Some(path.to_string()),
        size_bytes,
        category,
        risk,
        reason,
        recommended_action,
        destructive: false,
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
}
