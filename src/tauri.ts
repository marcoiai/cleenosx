import { invoke } from "@tauri-apps/api/core";
import type {
  CleanupOutcome,
  CleanupExecution,
  CleanupSelection,
  CleanupSettings,
  DeepScanResult,
  Finding,
  Overview,
  PreparedCleanupPlan,
  ScanResult,
  UsageNode,
  VolumeInfo,
} from "./types";

export async function scanOverview() {
  return invoke<ScanResult<Overview>>("scan_overview");
}

export async function getStorageOverview() {
  return invoke<ScanResult<Overview>>("get_storage_overview");
}

export async function getDefaultScanPath() {
  return invoke<string>("get_default_scan_path");
}

export async function openFullDiskAccessSettings() {
  return invoke<void>("open_full_disk_access_settings");
}

export async function scanVolumes() {
  return invoke<ScanResult<VolumeInfo[]>>("scan_volumes");
}

export async function scanDataUsage() {
  return invoke<ScanResult<UsageNode[]>>("scan_data_usage");
}

export async function startDeepScan(path: string) {
  return invoke<ScanResult<DeepScanResult>>("start_deep_scan", { path });
}

export async function cancelDeepScan() {
  return invoke<ScanResult<boolean>>("cancel_deep_scan");
}

export async function scanPathUsage(path: string) {
  return invoke<ScanResult<UsageNode[]>>("scan_path_usage", { path });
}

export async function scanAssetsV2() {
  return invoke<ScanResult<Finding[]>>("scan_assets_v2");
}

export async function scanDeveloperTools() {
  return invoke<ScanResult<Finding[]>>("scan_developer_tools");
}

export async function scanRustArtifacts() {
  return invoke<ScanResult<Finding[]>>("scan_rust_artifacts");
}

export async function scanContainers() {
  return invoke<ScanResult<Finding[]>>("scan_containers");
}

export async function listSnapshots() {
  return invoke<ScanResult<Finding[]>>("list_snapshots");
}

export async function generateRecoveryScript() {
  return invoke<string>("generate_recovery_script");
}

export async function exportRecoveryScript() {
  return invoke<string>("export_recovery_script");
}

export async function exportRecoveryScriptForTargets(paths: string[]) {
  return invoke<string>("export_recovery_script_for_targets", { paths });
}

export async function prepareCleanupPlan(selection: CleanupSelection) {
  return invoke<ScanResult<PreparedCleanupPlan>>("prepare_cleanup_plan", { selection });
}

export async function executeCleanupPlan(execution: CleanupExecution) {
  return invoke<ScanResult<CleanupOutcome>>("execute_cleanup_plan", { execution });
}

export async function executeRootCleanupContinuation(continuationId: string) {
  return invoke<ScanResult<CleanupOutcome>>("execute_root_cleanup_continuation", { continuationId });
}

export async function getCleanupSettings() {
  return invoke<CleanupSettings>("get_cleanup_settings");
}

export async function updateCleanupSettings(settings: CleanupSettings) {
  return invoke<CleanupSettings>("update_cleanup_settings", { settings });
}
