use cleanerx_core::{
    CleanupOutcome, CleanupPlan, Finding, Overview, ScanResult, UsageNode, VolumeInfo,
};

#[tauri::command]
fn scan_overview() -> ScanResult<Overview> {
    cleanerx_core::scan_overview()
}

#[tauri::command]
fn scan_volumes() -> ScanResult<Vec<VolumeInfo>> {
    cleanerx_core::scan_volumes()
}

#[tauri::command]
fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    cleanerx_core::scan_data_usage()
}

#[tauri::command]
fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    cleanerx_core::scan_user_usage()
}

#[tauri::command]
fn scan_assets_v2() -> ScanResult<Vec<Finding>> {
    cleanerx_core::scan_assets_v2()
}

#[tauri::command]
fn scan_developer_tools() -> ScanResult<Vec<Finding>> {
    cleanerx_core::scan_developer_tools()
}

#[tauri::command]
fn scan_rust_artifacts() -> ScanResult<Vec<Finding>> {
    cleanerx_core::scan_rust_artifacts()
}

#[tauri::command]
fn scan_containers() -> ScanResult<Vec<Finding>> {
    cleanerx_core::scan_containers()
}

#[tauri::command]
fn list_snapshots() -> ScanResult<Vec<Finding>> {
    cleanerx_core::list_snapshots()
}

#[tauri::command]
fn thin_snapshots() -> ScanResult<CleanupOutcome> {
    cleanerx_core::thin_snapshots()
}

#[tauri::command]
fn generate_recovery_script() -> String {
    cleanerx_core::generate_recovery_script()
}

#[tauri::command]
fn cleanup_selected_items(plan: CleanupPlan) -> ScanResult<CleanupOutcome> {
    cleanerx_core::cleanup_selected_items(plan)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_overview,
            scan_volumes,
            scan_data_usage,
            scan_user_usage,
            scan_assets_v2,
            scan_developer_tools,
            scan_rust_artifacts,
            scan_containers,
            list_snapshots,
            thin_snapshots,
            generate_recovery_script,
            cleanup_selected_items
        ])
        .run(tauri::generate_context!())
        .expect("error while running CleanerX");
}
