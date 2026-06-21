import type { RiskLevel, StorageCategory } from "./types";

export function formatBytes(bytes?: number | null) {
  if (bytes == null) return "Unknown";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function riskMeta(risk: RiskLevel) {
  switch (risk) {
    case "safeToAnalyze":
      return { label: "Safe", className: "bg-emerald-50 text-signal-safe ring-emerald-200" };
    case "attention":
      return { label: "Attention", className: "bg-amber-50 text-signal-attention ring-amber-200" };
    case "reviewRequired":
      return { label: "Review", className: "bg-orange-50 text-signal-review ring-orange-200" };
    case "dangerous":
      return { label: "Blocked", className: "bg-red-50 text-signal-danger ring-red-200" };
    case "readOnlySystem":
      return { label: "Read-only", className: "bg-slate-100 text-signal-system ring-slate-300" };
  }
}

export function categoryLabel(category: StorageCategory) {
  const labels: Record<StorageCategory, string> = {
    macOsApfs: "macOS/APFS",
    assetsV2: "AssetsV2",
    developerTools: "Developer",
    rustArtifacts: "Rust",
    nodeCaches: "Node",
    homebrew: "Homebrew",
    containers: "Containers",
    simulators: "Simulators",
    projects: "Projects",
    userData: "User",
    caches: "Caches",
    updates: "Updates",
    snapshots: "Snapshots",
    volumesExtra: "Extra volume",
    unknown: "Unknown",
  };
  return labels[category];
}
