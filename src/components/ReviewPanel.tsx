import { AlertTriangle, CheckSquare, ChevronRight, FolderOpen, RefreshCw, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { categoryLabel, formatBytes } from "../format";
import { cleanupSelectedItems, scanPathUsage } from "../tauri";
import type { CleanupItem, ScanLog, UsageNode } from "../types";
import { LoadingButton } from "./LoadingButton";
import { RiskChip } from "./RiskChip";

interface ReviewPanelProps {
  initialNodes: UsageNode[];
  onLogs: (logs: ScanLog[]) => void;
}

const rootChoices = [
  "/System/Volumes/Data",
  "/Users",
  "/tmp",
  "/Applications",
  "/opt/homebrew",
].filter((path, index, paths) => path && paths.indexOf(path) === index);

export function ReviewPanel({ initialNodes, onLogs }: ReviewPanelProps) {
  const [path, setPath] = useState("/System/Volumes/Data");
  const [nodes, setNodes] = useState<UsageNode[]>(initialNodes);
  const [selected, setSelected] = useState<Record<string, UsageNode>>({});
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmText, setConfirmText] = useState("");
  const [challengeIndex, setChallengeIndex] = useState(0);

  const selectedNodes = Object.values(selected);
  const selectedBytes = selectedNodes.reduce((total, node) => total + node.sizeBytes, 0);
  const challenge = useMemo(
    () => makeChallenge(selectedNodes, challengeIndex),
    [challengeIndex, selectedNodes],
  );

  async function scan(nextPath = path) {
    setLoading(true);
    setMessage("");
    try {
      const result = await scanPathUsage(nextPath);
      setPath(nextPath);
      setNodes(result.data);
      onLogs(result.logs);
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  function toggle(node: UsageNode) {
    setSelected((current) => {
      const next = { ...current };
      if (next[node.path]) {
        delete next[node.path];
      } else {
        next[node.path] = node;
      }
      return next;
    });
  }

  function prepareRemoval() {
    setConfirmText("");
    setChallengeIndex((value) => value + 1);
    setConfirmOpen(true);
  }

  async function removeSelected() {
    setLoading(true);
    setMessage("");
    try {
      const items: CleanupItem[] = selectedNodes.map((node) => ({
        path: node.path,
        risk: node.risk,
        estimatedBytes: node.sizeBytes,
        reason: `${categoryLabel(node.category)} cleanup target`,
      }));
      const result = await cleanupSelectedItems({
        items,
        confirmation: "I_UNDERSTAND_DELETE",
      });
      onLogs(result.logs);
      setMessage(result.data.message);
      setSelected({});
      setConfirmOpen(false);
      await scan(path);
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="grid gap-6">
      <section className="rounded-lg border border-slate-200 bg-white shadow-material">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-4 py-3">
          <div>
            <h2 className="text-sm font-semibold text-ink-strong">Clear</h2>
            <div className="mt-1 max-w-3xl truncate font-mono text-xs text-ink-muted">{path}</div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {rootChoices.map((choice) => (
              <button
                key={choice}
                className="min-h-9 rounded-lg bg-slate-100 px-3 text-xs font-semibold text-ink-body hover:bg-slate-200"
                onClick={() => void scan(choice)}
              >
                {shortPath(choice)}
              </button>
            ))}
            <LoadingButton loading={loading} onClick={() => void scan()}>
              <RefreshCw size={16} />
              Scan
            </LoadingButton>
          </div>
        </div>

        <div className="divide-y divide-slate-100">
          {nodes.length === 0 ? (
            <div className="px-4 py-10 text-center text-sm text-ink-muted">Scan a location to find removable bottlenecks.</div>
          ) : (
            nodes.map((node) => (
              <ClearRow
                key={node.path}
                node={node}
                selected={Boolean(selected[node.path])}
                onSelect={() => toggle(node)}
                onDrill={() => void scan(node.path)}
              />
            ))
          )}
        </div>
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-50 text-blue-700">
              <CheckSquare size={21} />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-ink-strong">Selected Targets</h2>
              <div className="text-sm text-ink-muted">
                {selectedNodes.length} item(s), {formatBytes(selectedBytes)}
              </div>
            </div>
          </div>
          <LoadingButton
            loading={loading}
            disabled={selectedNodes.length === 0}
            className="bg-red-700 hover:bg-red-800"
            onClick={prepareRemoval}
          >
            <Trash2 size={16} />
            Remove
          </LoadingButton>
        </div>

        {selectedNodes.length > 0 && (
          <div className="mt-4 grid gap-2">
            {selectedNodes.map((node) => (
              <div key={node.path} className="flex items-center justify-between gap-3 rounded-lg bg-slate-50 px-3 py-2 text-sm">
                <span className="truncate font-mono text-xs text-ink-strong">{node.path}</span>
                <span className="shrink-0 font-semibold">{formatBytes(node.sizeBytes)}</span>
              </div>
            ))}
          </div>
        )}

        {message && <div className="mt-4 rounded-lg bg-slate-50 p-3 text-sm text-ink-body">{message}</div>}
      </section>

      {confirmOpen && (
        <section className={`rounded-lg border p-4 shadow-material ${challenge.panelClass}`}>
          <div className={`grid gap-4 ${challenge.layoutClass}`}>
            <div>
              <div className="flex items-center gap-2 text-sm font-semibold">
                <AlertTriangle size={18} />
                Confirm Removal
              </div>
              <p className="mt-2 text-sm">
                This will remove {selectedNodes.length} selected item(s), approximately {formatBytes(selectedBytes)}.
              </p>
              <p className="mt-2 text-sm font-semibold">Type `{challenge.phrase}` to unlock the remove button.</p>
              <input
                className="mt-3 min-h-10 w-full rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm text-ink-strong outline-none focus:border-blue-500"
                value={confirmText}
                onChange={(event) => setConfirmText(event.target.value)}
              />
            </div>
            <div className="flex flex-wrap items-end justify-end gap-2">
              <button
                className="min-h-10 rounded-lg bg-white px-4 text-sm font-semibold text-ink-body ring-1 ring-slate-300 hover:bg-slate-50"
                onClick={() => setConfirmOpen(false)}
              >
                Cancel
              </button>
              <LoadingButton
                loading={loading}
                disabled={confirmText !== challenge.phrase}
                className={challenge.buttonClass}
                onClick={() => void removeSelected()}
              >
                <Trash2 size={16} />
                Remove Selected
              </LoadingButton>
            </div>
          </div>
        </section>
      )}
    </section>
  );
}

function ClearRow({
  node,
  selected,
  onSelect,
  onDrill,
}: {
  node: UsageNode;
  selected: boolean;
  onSelect: () => void;
  onDrill: () => void;
}) {
  const selectable = node.risk !== "dangerous" && node.risk !== "readOnlySystem" && !isBroadTarget(node.path);

  return (
    <div className="grid min-h-16 grid-cols-[36px_1fr_132px_124px_96px] items-center gap-3 px-4 py-3 text-sm">
      <input
        type="checkbox"
        className="h-4 w-4"
        checked={selected}
        disabled={!selectable}
        onChange={onSelect}
        title={selectable ? "Select for removal" : "This target is protected by risk rules"}
      />
      <div className="flex min-w-0 items-center gap-2">
        <button
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-blue-700 hover:bg-blue-50"
          onClick={onDrill}
          title="Drill down"
        >
          <FolderOpen size={16} />
        </button>
        <div className="min-w-0">
          <div className="truncate font-mono text-xs text-ink-strong">{node.path}</div>
          <div className="mt-1 text-xs text-ink-muted">{categoryLabel(node.category)}</div>
        </div>
      </div>
      <div className="text-right font-semibold text-ink-strong">{formatBytes(node.sizeBytes)}</div>
      <RiskChip risk={node.risk} />
      <button className="inline-flex items-center justify-end gap-1 text-xs font-semibold text-blue-700" onClick={onDrill}>
        Open
        <ChevronRight size={14} />
      </button>
    </div>
  );
}

function makeChallenge(nodes: UsageNode[], index: number) {
  const total = nodes.reduce((sum, node) => sum + node.sizeBytes, 0);
  const last = nodes[0] ? basename(nodes[0].path) : "TARGET";
  const variants = [
    {
      phrase: `CLEAR ${nodes.length} ITEMS`,
      panelClass: "border-red-200 bg-red-50",
      buttonClass: "bg-red-700 hover:bg-red-800",
      layoutClass: "md:grid-cols-[1fr_280px]",
    },
    {
      phrase: `REMOVE ${formatBytes(total)}`,
      panelClass: "border-amber-200 bg-amber-50",
      buttonClass: "bg-amber-700 hover:bg-amber-800",
      layoutClass: "md:grid-cols-[280px_1fr]",
    },
    {
      phrase: `DELETE ${last}`,
      panelClass: "border-slate-300 bg-slate-100",
      buttonClass: "bg-slate-900 hover:bg-black",
      layoutClass: "md:grid-cols-[1fr_240px]",
    },
  ];
  return variants[index % variants.length];
}

function basename(path: string) {
  return path.split("/").filter(Boolean).pop() ?? path;
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

function shortPath(path: string) {
  if (path === "/System/Volumes/Data") return "Data";
  return basename(path);
}
