use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    SafeToAnalyze,
    Attention,
    ReviewRequired,
    Dangerous,
    ReadOnlySystem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StorageCategory {
    MacOsApfs,
    AssetsV2,
    DeveloperTools,
    RustArtifacts,
    NodeCaches,
    Homebrew,
    Containers,
    Simulators,
    Projects,
    UserData,
    Caches,
    Updates,
    Snapshots,
    VolumesExtra,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UsageKind {
    File,
    Folder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionStatus {
    Candidate,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    MeasureOnly,
    ListOnly,
    DeleteFiles,
    PurgeCache,
    OpenFolder,
    Advisory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionScores {
    pub safety_percent: u8,
    pub reclaim_value_percent: u8,
    pub automation_percent: u8,
    pub confidence_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCapability {
    pub can_delete: bool,
    pub user_facing_level: String,
    pub user_facing_summary: String,
    pub technical_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionUi {
    pub badge: String,
    pub severity_percent: u8,
    pub primary_action: String,
    pub secondary_action: Option<String>,
    pub explain_like_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRecommendation {
    pub include_in_app: bool,
    pub include_as_cleanup: bool,
    pub include_as_diagnostic: bool,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionProfile {
    pub status: ActionStatus,
    pub action_type: ActionType,
    pub deletes_files: bool,
    pub command: Option<String>,
    pub requires_sudo: bool,
    pub scores: ActionScores,
    pub delete_capability: DeleteCapability,
    pub ui: ActionUi,
    pub recommendation: ActionRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub name: String,
    pub identifier: String,
    pub role: Option<String>,
    pub mount_point: Option<String>,
    pub mounted: bool,
    pub encrypted: Option<bool>,
    pub locked: Option<bool>,
    pub flags: Vec<String>,
    pub capacity_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub risk: RiskLevel,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageNode {
    pub id: String,
    pub path: String,
    pub kind: UsageKind,
    pub size_bytes: u64,
    pub category: StorageCategory,
    pub risk: RiskLevel,
    pub flags: Vec<String>,
    pub children: Vec<UsageNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepScanResult {
    pub path: String,
    pub entries: Vec<UsageNode>,
    pub partial: bool,
    pub canceled: bool,
    pub warnings_summary: DeepScanWarningsSummary,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeepScanWarningsSummary {
    pub permission_denied: usize,
    pub operation_not_permitted: usize,
    pub vanished_paths: usize,
    pub unexpected_errors: Vec<String>,
    pub samples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub title: String,
    pub path: Option<String>,
    pub size_bytes: Option<u64>,
    pub category: StorageCategory,
    pub risk: RiskLevel,
    pub reason: String,
    pub recommended_action: String,
    pub destructive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_profile: Option<ActionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub percent_used: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub summary: StorageSummary,
    pub volumes: Vec<VolumeInfo>,
    pub usage_roots: Vec<UsageNode>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutcome {
    pub dry_run: bool,
    pub deleted_bytes: u64,
    pub message: String,
    pub removed_items: Vec<CleanupItemOutcome>,
    pub failed_items: Vec<CleanupItemOutcome>,
    pub needs_root: bool,
    pub root_continuation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemOutcome {
    pub path: String,
    pub message: String,
    pub needs_root: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSelection {
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedCleanupItem {
    pub id: String,
    pub path: String,
    pub kind: UsageKind,
    pub category: StorageCategory,
    pub risk: RiskLevel,
    pub estimated_bytes: u64,
    pub reason: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_profile: Option<ActionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedCleanupPlan {
    pub plan_id: String,
    pub items: Vec<PreparedCleanupItem>,
    pub estimated_recoverable_bytes: u64,
    pub warnings: Vec<String>,
    pub final_confirmation_phrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecution {
    pub plan_id: String,
    pub final_confirmation: String,
    #[serde(default)]
    pub elevated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSettings {
    pub allow_project_root_cleanup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLog {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult<T> {
    pub data: T,
    pub logs: Vec<ScanLog>,
}

impl ScanLog {
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }

    fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();

        Self {
            timestamp,
            level,
            message: message.into(),
        }
    }
}
