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

export type UsageKind = "file" | "folder";

export type LogLevel = "info" | "warning" | "error";

export type ActionStatus = "candidate" | "approved" | "rejected";

export type ActionType =
  | "measureOnly"
  | "listOnly"
  | "deleteFiles"
  | "purgeCache"
  | "openFolder"
  | "advisory";

export interface ActionScores {
  safetyPercent: number;
  reclaimValuePercent: number;
  automationPercent: number;
  confidencePercent: number;
}

export interface DeleteCapability {
  canDelete: boolean;
  userFacingLevel: string;
  userFacingSummary: string;
  technicalReason: string;
}

export interface ActionUi {
  badge: string;
  severityPercent: number;
  primaryAction: string;
  secondaryAction?: string | null;
  explainLikeUser: string;
}

export interface ActionRecommendation {
  includeInApp: boolean;
  includeAsCleanup: boolean;
  includeAsDiagnostic: boolean;
  nextAction?: string | null;
}

export interface ActionProfile {
  status: ActionStatus;
  actionType: ActionType;
  deletesFiles: boolean;
  command?: string | null;
  requiresSudo: boolean;
  scores: ActionScores;
  deleteCapability: DeleteCapability;
  ui: ActionUi;
  recommendation: ActionRecommendation;
}

export interface ScanLog {
  timestamp: number;
  level: LogLevel;
  message: string;
}

export interface ScanResult<T> {
  data: T;
  logs: ScanLog[];
}

export interface CleanupOutcome {
  dryRun: boolean;
  deletedBytes: number;
  message: string;
  removedItems: CleanupItemOutcome[];
  failedItems: CleanupItemOutcome[];
  needsRoot: boolean;
  rootContinuationId?: string | null;
}

export interface CleanupItemOutcome {
  path: string;
  message: string;
  needsRoot: boolean;
}

export interface CleanupSelection {
  itemIds: string[];
}

export interface PreparedCleanupItem {
  id: string;
  path: string;
  kind: UsageKind;
  category: StorageCategory;
  risk: RiskLevel;
  estimatedBytes: number;
  reason: string;
  action: string;
  actionProfile?: ActionProfile | null;
}

export interface PreparedCleanupPlan {
  planId: string;
  items: PreparedCleanupItem[];
  estimatedRecoverableBytes: number;
  warnings: string[];
  finalConfirmationPhrase: string;
}

export interface CleanupExecution {
  planId: string;
  finalConfirmation: string;
  elevated?: boolean;
}

export interface CleanupSettings {
  allowProjectRootCleanup: boolean;
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
  id: string;
  path: string;
  kind: UsageKind;
  sizeBytes: number;
  category: StorageCategory;
  risk: RiskLevel;
  flags: string[];
  children: UsageNode[];
}

export interface DeepScanResult {
  path: string;
  entries: UsageNode[];
  partial: boolean;
  canceled: boolean;
  warningsSummary: DeepScanWarningsSummary;
  durationMs: number;
}

export interface DeepScanWarningsSummary {
  permissionDenied: number;
  operationNotPermitted: number;
  vanishedPaths: number;
  unexpectedErrors: string[];
  samples: string[];
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
  actionProfile?: ActionProfile | null;
}

export interface Overview {
  summary: StorageSummary;
  volumes: VolumeInfo[];
  usageRoots: UsageNode[];
  findings: Finding[];
}
