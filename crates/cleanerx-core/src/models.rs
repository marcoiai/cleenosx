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
    pub path: String,
    pub size_bytes: u64,
    pub category: StorageCategory,
    pub risk: RiskLevel,
    pub flags: Vec<String>,
    pub children: Vec<UsageNode>,
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
pub struct CleanupPlan {
    pub items: Vec<CleanupItem>,
    pub confirmation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub path: String,
    pub risk: RiskLevel,
    pub estimated_bytes: Option<u64>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutcome {
    pub dry_run: bool,
    pub deleted_bytes: u64,
    pub message: String,
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
