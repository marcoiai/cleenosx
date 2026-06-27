import { CheckSquare, ChevronDown, ChevronRight, File, FolderOpen } from "lucide-react";
import { useMemo, useState } from "react";
import { categoryLabel, formatBytes, riskSortRank } from "../format";
import type { UsageNode } from "../types";
import { PathText } from "./PathText";
import { RiskChip } from "./RiskChip";

const TEST_DELETE_PATHS = new Set([
  "/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/Fake",
  "/private/tmp/Fake/test.txt",
]);

interface UsageTreeProps {
  nodes: UsageNode[];
  onScanPath?: (path: string) => void;
  onSelectForCleanup?: (node: UsageNode) => void;
  allowProjectRootCleanup?: boolean;
  disabled?: boolean;
}

export function UsageTree({ nodes, onScanPath, onSelectForCleanup, allowProjectRootCleanup = false, disabled = false }: UsageTreeProps) {
  const sortedNodes = useMemo(() => sortUsageNodes(nodes), [nodes]);

  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Large Blocks</h2>
      </div>
      <div className="divide-y divide-slate-100">
        {sortedNodes.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-ink-muted">No usage data yet.</div>
        ) : (
          sortedNodes.map((node) => (
            <UsageRow
              key={node.path}
              node={node}
              depth={0}
              onScanPath={onScanPath}
              onSelectForCleanup={onSelectForCleanup}
              allowProjectRootCleanup={allowProjectRootCleanup}
              disabled={disabled}
            />
          ))
        )}
      </div>
    </section>
  );
}

function UsageRow({
  node,
  depth,
  onScanPath,
  onSelectForCleanup,
  allowProjectRootCleanup,
  disabled,
}: {
  node: UsageNode;
  depth: number;
  onScanPath?: (path: string) => void;
  onSelectForCleanup?: (node: UsageNode) => void;
  allowProjectRootCleanup: boolean;
  disabled: boolean;
}) {
  const [open, setOpen] = useState(false);
  const hasChildren = node.children.length > 0;
  const selectable =
    node.risk !== "readOnlySystem" &&
    (node.risk !== "dangerous" || (allowProjectRootCleanup && isProjectPath(node.path))) &&
    !isAssetsV2Area(node.path) &&
    !isBroadTarget(node.path);
  const KindIcon = node.kind === "file" ? File : FolderOpen;
  const kindLabel = node.kind === "file" ? "File" : "Directory";

  return (
    <div>
      <div className="grid min-h-14 grid-cols-[1fr_130px_120px_120px_164px] items-center gap-3 px-4 py-3 text-sm">
        <div className="flex min-w-0 items-center gap-2" style={{ paddingLeft: depth * 18 }}>
          <button
            className="flex h-8 w-8 items-center justify-center rounded-lg text-ink-muted hover:bg-slate-100 disabled:opacity-30"
            disabled={!hasChildren}
            onClick={() => setOpen((value) => !value)}
            title={open ? "Collapse" : "Expand"}
          >
            {hasChildren ? open ? <ChevronDown size={16} /> : <ChevronRight size={16} /> : <span className="h-4 w-4" />}
          </button>
          <div
            className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg ring-1 ${
              node.kind === "file"
                ? "bg-slate-50 text-ink-muted ring-slate-200"
                : "bg-blue-50 text-blue-700 ring-blue-200"
            }`}
            title={kindLabel}
          >
            <KindIcon size={16} />
          </div>
          <div className="min-w-0">
            <PathText path={node.path} className="text-ink-strong" />
            {node.flags.length > 0 && <div className="mt-1 text-xs text-amber-700">flags: {node.flags.join(", ")}</div>}
          </div>
        </div>
        <div className="text-right font-semibold text-ink-strong">{formatBytes(node.sizeBytes)}</div>
        <div className="text-xs text-ink-muted">{categoryLabel(node.category)}</div>
        <RiskChip risk={node.risk} />
        <div className="flex items-center justify-end gap-2">
          <button
            className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-blue-700 px-2 text-xs font-semibold text-white hover:bg-blue-800 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!onSelectForCleanup || !selectable || disabled}
            onClick={() => onSelectForCleanup?.(node)}
            title={selectable ? "Select for cleanup" : "This target is protected by risk rules"}
          >
            <CheckSquare size={14} />
            Select
          </button>
          <button
            className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-100 px-2 text-xs font-semibold text-ink-body hover:bg-slate-200 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!onScanPath || disabled}
            onClick={() => onScanPath?.(node.path)}
            title="Scan this path"
          >
            <KindIcon size={14} />
            Scan
          </button>
        </div>
      </div>
      {open && sortUsageNodes(node.children).map((child) => (
        <UsageRow
          key={child.path}
          node={child}
          depth={depth + 1}
          onScanPath={onScanPath}
          onSelectForCleanup={onSelectForCleanup}
          allowProjectRootCleanup={allowProjectRootCleanup}
          disabled={disabled}
        />
      ))}
    </div>
  );
}

function isBroadTarget(path: string) {
  return [
    "/",
    "/System",
    "/Library",
    "/Applications",
    "/Users",
    "/tmp",
    "/private/tmp",
    "/opt",
    "/opt/homebrew",
    "/System/Volumes",
    "/System/Volumes/Data",
    "/System/Library/AssetsV2",
    "/System/Volumes/Data/System/Library/AssetsV2",
  ].includes(path);
}

function isProjectPath(path: string) {
  return path.includes("/Projects");
}

function isAssetsV2Area(path: string) {
  return (
    path === "/System/Library/AssetsV2" ||
    path.startsWith("/System/Library/AssetsV2/") ||
    path === "/System/Volumes/Data/System/Library/AssetsV2" ||
    path.startsWith("/System/Volumes/Data/System/Library/AssetsV2/")
  );
}

function sortUsageNodes(nodes: UsageNode[]) {
  return [...nodes].sort(
    (left, right) =>
      riskSortRank(left.risk) - riskSortRank(right.risk) ||
      Number(TEST_DELETE_PATHS.has(right.path)) - Number(TEST_DELETE_PATHS.has(left.path)) ||
      right.sizeBytes - left.sizeBytes ||
      left.path.localeCompare(right.path),
  );
}
