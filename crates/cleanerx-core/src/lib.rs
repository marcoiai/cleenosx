pub mod admin;
pub mod classify;
pub mod cleanup;
pub mod command;
pub mod models;
pub mod recovery;
pub mod scanners;

pub use models::*;

pub fn admin_session_status() -> AdminSessionStatus {
    admin::admin_session_status()
}

pub fn unlock_admin_session() -> Result<AdminSessionStatus, String> {
    admin::unlock_admin_session()
}

pub fn lock_admin_session() -> AdminSessionStatus {
    admin::lock_admin_session()
}

pub fn scan_overview() -> ScanResult<Overview> {
    scanners::scan_storage_overview()
}

pub fn get_storage_overview() -> ScanResult<Overview> {
    scanners::scan_storage_overview()
}

pub fn scan_volumes() -> ScanResult<Vec<VolumeInfo>> {
    scanners::scan_volumes()
}

pub fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    scanners::scan_data_usage()
}

pub fn start_deep_scan(path: String) -> ScanResult<DeepScanResult> {
    scanners::start_deep_scan(&path)
}

pub fn cancel_deep_scan() -> ScanResult<bool> {
    scanners::cancel_deep_scan()
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

pub fn generate_recovery_script_with_targets(targets: &[String]) -> String {
    recovery::generate_recovery_script_with_targets(targets)
}

pub fn prepare_cleanup_plan(selection: CleanupSelection) -> ScanResult<PreparedCleanupPlan> {
    cleanup::prepare_cleanup_plan(selection)
}

pub fn execute_cleanup_plan(execution: CleanupExecution) -> ScanResult<CleanupOutcome> {
    cleanup::execute_cleanup_plan(execution)
}

pub fn execute_root_cleanup_continuation(continuation_id: String) -> ScanResult<CleanupOutcome> {
    cleanup::execute_root_cleanup_continuation(continuation_id)
}

pub fn cleanup_settings() -> CleanupSettings {
    cleanup::cleanup_settings()
}

pub fn update_cleanup_settings(settings: CleanupSettings) -> CleanupSettings {
    cleanup::update_cleanup_settings(settings)
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
            removed_items: Vec::new(),
            failed_items: Vec::new(),
            needs_root: false,
            root_continuation_id: None,
        },
        logs,
    }
}
