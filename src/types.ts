export type RiskLevel =
  | "safeToAnalyze"
  | "attention"
  | "reviewRequired"
  | "dangerous"
  | "readOnlySystem";

export type StorageCategory =
  | "macOsApfs"
  | "assetsV2"
  | "developerTools"
  | "rustArtifacts"
  | "nodeCaches"
  | "homebrew"
  | "containers"
  | "simulators"
  | "projects"
  | "userData"
  | "caches"
  | "updates"
  | "snapshots"
  | "volumesExtra"
  | "unknown";

export type LogLevel = "info" | "warning" | "error";

export interface ScanLog {
  timestamp: number;
  level: LogLevel;
  message: string;
}

export interface ScanResult<T> {
  data: T;
  logs: ScanLog[];
}

export interface CleanupItem {
  path: string;
  risk: RiskLevel;
  estimatedBytes?: number | null;
  reason: string;
}

export interface CleanupPlan {
  items: CleanupItem[];
  confirmation?: string | null;
}

export interface CleanupOutcome {
  dryRun: boolean;
  deletedBytes: number;
  message: string;
}

export interface StorageSummary {
  totalBytes?: number | null;
  usedBytes?: number | null;
  availableBytes?: number | null;
  percentUsed?: number | null;
}

export interface VolumeInfo {
  name: string;
  identifier: string;
  role?: string | null;
  mountPoint?: string | null;
  mounted: boolean;
  encrypted?: boolean | null;
  locked?: boolean | null;
  flags: string[];
  capacityBytes?: number | null;
  usedBytes?: number | null;
  availableBytes?: number | null;
  risk: RiskLevel;
  notes: string[];
}

export interface UsageNode {
  path: string;
  sizeBytes: number;
  category: StorageCategory;
  risk: RiskLevel;
  flags: string[];
  children: UsageNode[];
}

export interface Finding {
  title: string;
  path?: string | null;
  sizeBytes?: number | null;
  category: StorageCategory;
  risk: RiskLevel;
  reason: string;
  recommendedAction: string;
  destructive: boolean;
}

export interface Overview {
  summary: StorageSummary;
  volumes: VolumeInfo[];
  usageRoots: UsageNode[];
  findings: Finding[];
}
