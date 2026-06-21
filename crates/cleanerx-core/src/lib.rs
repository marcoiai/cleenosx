pub mod classify;
pub mod command;
pub mod models;
pub mod recovery;
pub mod scanners;

pub use models::*;

pub fn scan_overview() -> ScanResult<Overview> {
    scanners::scan_overview()
}

pub fn scan_volumes() -> ScanResult<Vec<VolumeInfo>> {
    scanners::scan_volumes()
}

pub fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    scanners::scan_data_usage()
}

pub fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    scanners::scan_user_usage()
}

pub fn scan_path_usage(path: String) -> ScanResult<Vec<UsageNode>> {
    scanners::scan_path_usage(&path)
}

pub fn scan_assets_v2() -> ScanResult<Vec<Finding>> {
    scanners::scan_assets_v2()
}

pub fn scan_developer_tools() -> ScanResult<Vec<Finding>> {
    scanners::scan_developer_tools()
}

pub fn scan_rust_artifacts() -> ScanResult<Vec<Finding>> {
    scanners::scan_rust_artifacts()
}

pub fn scan_containers() -> ScanResult<Vec<Finding>> {
    scanners::scan_containers()
}

pub fn list_snapshots() -> ScanResult<Vec<Finding>> {
    scanners::list_snapshots()
}

pub fn generate_recovery_script() -> String {
    recovery::generate_recovery_script()
}

pub fn cleanup_selected_items(plan: CleanupPlan) -> ScanResult<CleanupOutcome> {
    scanners::cleanup_selected_items(plan)
}

pub fn thin_snapshots() -> ScanResult<CleanupOutcome> {
    let logs = vec![ScanLog::info(
        "MVP safe mode: snapshot thinning is disabled. No tmutil command was executed.",
    )];

    ScanResult {
        data: CleanupOutcome {
            dry_run: true,
            deleted_bytes: 0,
            message: "Snapshot thinning is disabled in the MVP.".to_string(),
        },
        logs,
    }
}
