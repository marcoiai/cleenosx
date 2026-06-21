import { invoke } from "@tauri-apps/api/core";
import type {
  CleanupOutcome,
  CleanupPlan,
  Finding,
  Overview,
  ScanResult,
  UsageNode,
  VolumeInfo,
} from "./types";

export async function scanOverview() {
  return invoke<ScanResult<Overview>>("scan_overview");
}

export async function scanVolumes() {
  return invoke<ScanResult<VolumeInfo[]>>("scan_volumes");
}

export async function scanDataUsage() {
  return invoke<ScanResult<UsageNode[]>>("scan_data_usage");
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

export async function cleanupSelectedItems(plan: CleanupPlan) {
  return invoke<ScanResult<CleanupOutcome>>("cleanup_selected_items", { plan });
}
