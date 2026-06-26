use cleanerx_core::{
    AdminSessionStatus, CleanupExecution, CleanupOutcome, CleanupSelection, CleanupSettings,
    DeepScanResult, Finding, Overview, PreparedCleanupPlan, ScanLog, ScanResult, UsageNode,
    VolumeInfo,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[tauri::command]
async fn get_storage_overview() -> ScanResult<Overview> {
    blocking(cleanerx_core::get_storage_overview).await
}

#[tauri::command]
fn get_default_scan_path() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string())
}

#[tauri::command]
fn open_full_disk_access_settings() -> Result<(), String> {
    if app_store_build() {
        return Err(
            "Full Disk Access shortcuts are unavailable in the App Store build.".to_string(),
        );
    }

    let status = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .status()
        .map_err(|error| format!("failed to open macOS Privacy settings: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "macOS open command exited with status {}",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

#[tauri::command]
fn get_admin_session_status() -> AdminSessionStatus {
    if app_store_build() {
        return AdminSessionStatus {
            unlocked: false,
            available: false,
            last_unlocked_at_ms: None,
            message: "Admin Mode is unavailable in the App Store build.".to_string(),
        };
    }

    cleanerx_core::admin_session_status()
}

#[tauri::command]
fn unlock_admin_session() -> Result<AdminSessionStatus, String> {
    if app_store_build() {
        return Err("Admin Mode is unavailable in the App Store build.".to_string());
    }

    cleanerx_core::unlock_admin_session()
}

#[tauri::command]
fn lock_admin_session() -> AdminSessionStatus {
    if app_store_build() {
        return AdminSessionStatus {
            unlocked: false,
            available: false,
            last_unlocked_at_ms: None,
            message: "Admin Mode is unavailable in the App Store build.".to_string(),
        };
    }

    cleanerx_core::lock_admin_session()
}

#[tauri::command]
async fn scan_overview() -> ScanResult<Overview> {
    blocking(cleanerx_core::scan_overview).await
}

#[tauri::command]
async fn scan_volumes() -> ScanResult<Vec<VolumeInfo>> {
    blocking(cleanerx_core::scan_volumes).await
}

#[tauri::command]
async fn scan_data_usage() -> ScanResult<Vec<UsageNode>> {
    blocking(cleanerx_core::scan_data_usage).await
}

#[tauri::command]
async fn start_deep_scan(path: String) -> ScanResult<DeepScanResult> {
    blocking(move || cleanerx_core::start_deep_scan(path)).await
}

#[tauri::command]
async fn cancel_deep_scan() -> ScanResult<bool> {
    blocking(cleanerx_core::cancel_deep_scan).await
}

#[tauri::command]
async fn scan_user_usage() -> ScanResult<Vec<UsageNode>> {
    blocking(cleanerx_core::scan_user_usage).await
}

#[tauri::command]
async fn scan_path_usage(path: String) -> ScanResult<Vec<UsageNode>> {
    blocking(move || cleanerx_core::scan_path_usage(path)).await
}

#[tauri::command]
async fn scan_assets_v2() -> ScanResult<Vec<Finding>> {
    blocking(cleanerx_core::scan_assets_v2).await
}

#[tauri::command]
async fn scan_developer_tools() -> ScanResult<Vec<Finding>> {
    blocking(cleanerx_core::scan_developer_tools).await
}

#[tauri::command]
async fn scan_rust_artifacts() -> ScanResult<Vec<Finding>> {
    blocking(cleanerx_core::scan_rust_artifacts).await
}

#[tauri::command]
async fn scan_containers() -> ScanResult<Vec<Finding>> {
    blocking(cleanerx_core::scan_containers).await
}

#[tauri::command]
async fn list_snapshots() -> ScanResult<Vec<Finding>> {
    blocking(cleanerx_core::list_snapshots).await
}

#[tauri::command]
async fn thin_snapshots() -> ScanResult<CleanupOutcome> {
    blocking(cleanerx_core::thin_snapshots).await
}

#[tauri::command]
fn generate_recovery_script() -> String {
    if app_store_build() {
        return "Recovery helper export is unavailable in the App Store build.".to_string();
    }

    cleanerx_core::generate_recovery_script()
}

#[tauri::command]
fn export_recovery_script() -> Result<String, String> {
    if app_store_build() {
        return Err("Recovery helper export is unavailable in the App Store build.".to_string());
    }

    write_recovery_script(cleanerx_core::generate_recovery_script())
}

#[tauri::command]
fn export_recovery_script_for_targets(paths: Vec<String>) -> Result<String, String> {
    if app_store_build() {
        return Err("Recovery helper export is unavailable in the App Store build.".to_string());
    }

    write_recovery_script(cleanerx_core::generate_recovery_script_with_targets(&paths))
}

fn write_recovery_script(script: String) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|error| format!("HOME is not available: {error}"))?;
    let shared = std::path::Path::new("/Users/Shared");
    let primary_directory = if shared.is_dir() {
        shared.to_path_buf()
    } else {
        std::path::PathBuf::from(&home)
    };
    let primary_path = primary_directory.join("cx.sh");
    write_executable_script(&primary_path, &script)?;

    let desktop = std::path::Path::new(&home).join("Desktop");
    if desktop.is_dir() {
        let _ = write_executable_script(&desktop.join("cx.sh"), &script);
    }

    Ok(primary_path.display().to_string())
}

fn write_executable_script(path: &std::path::Path, script: &str) -> Result<(), String> {
    std::fs::write(path, script)
        .map_err(|error| format!("failed to write recovery script: {error}"))?;

    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)
            .map_err(|error| format!("failed to read recovery script permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)
            .map_err(|error| format!("failed to make recovery script executable: {error}"))?;
    }

    Ok(())
}

#[tauri::command]
async fn prepare_cleanup_plan(selection: CleanupSelection) -> ScanResult<PreparedCleanupPlan> {
    blocking(move || cleanerx_core::prepare_cleanup_plan(selection)).await
}

#[tauri::command]
async fn execute_cleanup_plan(execution: CleanupExecution) -> ScanResult<CleanupOutcome> {
    blocking(move || cleanerx_core::execute_cleanup_plan(execution)).await
}

#[tauri::command]
async fn execute_root_cleanup_continuation(continuation_id: String) -> ScanResult<CleanupOutcome> {
    if app_store_build() {
        return app_store_cleanup_refused(
            "Administrator cleanup is unavailable in the App Store build.",
        );
    }

    blocking(move || cleanerx_core::execute_root_cleanup_continuation(continuation_id)).await
}

#[tauri::command]
fn get_cleanup_settings() -> CleanupSettings {
    cleanerx_core::cleanup_settings()
}

#[tauri::command]
fn update_cleanup_settings(settings: CleanupSettings) -> CleanupSettings {
    cleanerx_core::update_cleanup_settings(settings)
}

async fn blocking<T, F>(work: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .expect("blocking task panicked")
}

fn app_store_build() -> bool {
    option_env!("CLEANERX_DISTRIBUTION") == Some("app-store")
}

fn app_store_cleanup_refused(message: &str) -> ScanResult<CleanupOutcome> {
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
        logs: vec![ScanLog::warning(message)],
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_storage_overview,
            get_default_scan_path,
            open_full_disk_access_settings,
            get_admin_session_status,
            unlock_admin_session,
            lock_admin_session,
            scan_overview,
            scan_volumes,
            scan_data_usage,
            start_deep_scan,
            cancel_deep_scan,
            scan_user_usage,
            scan_path_usage,
            scan_assets_v2,
            scan_developer_tools,
            scan_rust_artifacts,
            scan_containers,
            list_snapshots,
            thin_snapshots,
            generate_recovery_script,
            export_recovery_script,
            export_recovery_script_for_targets,
            prepare_cleanup_plan,
            execute_cleanup_plan,
            execute_root_cleanup_continuation,
            get_cleanup_settings,
            update_cleanup_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running CleanerX");
}
