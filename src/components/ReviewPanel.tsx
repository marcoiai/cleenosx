import { AlertTriangle, ArrowUp, CheckSquare, ChevronRight, CircleStop, ClipboardCheck, File, FolderOpen, RefreshCw, ShieldCheck, Trash2, Unplug, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { categoryLabel, formatBytes, riskSortRank } from "../format";
import { useI18n } from "../i18n";
import { formatScanWarningSummary, hasScanWarnings } from "../scanWarnings";
import { cancelDeepScan, executeCleanupPlan, executeRootCleanupContinuation, listenDeepScanProgress, prepareCleanupPlan, startDeepScan } from "../tauri";
import type { CleanupOutcome, DeepScanProgress, PreparedCleanupItem, PreparedCleanupPlan, RiskLevel, ScanLog, UsageNode } from "../types";
import { ActionScoreSummary } from "./ActionScoreSummary";
import { LoadingButton } from "./LoadingButton";
import { PathText } from "./PathText";
import { RiskChip } from "./RiskChip";

const TEST_DELETE_PATHS = new Set([
  "/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/Fake",
  "/private/tmp/Fake/test.txt",
]);

interface ReviewPanelProps {
  initialNodes: UsageNode[];
  initialSelectedNodes?: UsageNode[];
  defaultPath: string;
  initialPath?: string;
  allowProjectRootCleanup?: boolean;
  appStoreMode?: boolean;
  adminSessionUnlocked?: boolean;
  onLogs: (logs: ScanLog[]) => void;
  onCleanupRecovered?: (deletedBytes: number) => void;
  onRescanPath?: (path: string) => void;
  onUnmountAndRevealPath?: (path: string, elevated?: boolean) => void;
}

export function ReviewPanel({
  initialNodes,
  initialSelectedNodes = [],
  defaultPath,
  initialPath = defaultPath,
  allowProjectRootCleanup = false,
  appStoreMode = false,
  adminSessionUnlocked = false,
  onLogs,
  onCleanupRecovered,
  onRescanPath,
  onUnmountAndRevealPath,
}: ReviewPanelProps) {
  const { t } = useI18n();
  const [path, setPath] = useState(initialPath);
  const [manualPath, setManualPath] = useState(initialPath);
  const [nodes, setNodes] = useState<UsageNode[]>(initialNodes);
  const [selected, setSelected] = useState<Record<string, UsageNode>>({});
  const [loading, setLoading] = useState(false);
  const [scanLoading, setScanLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [preparedPlan, setPreparedPlan] = useState<PreparedCleanupPlan | null>(null);
  const [finalConfirmation, setFinalConfirmation] = useState("");
  const [cleanupStatus, setCleanupStatus] = useState("");
  const [rootContinuationId, setRootContinuationId] = useState("");
  const [useElevated, setUseElevated] = useState(false);
  const [scanProgress, setScanProgress] = useState<DeepScanProgress | null>(null);
  const scanInFlight = useRef(false);
  const currentScanPathRef = useRef("");
  const confirmationRef = useRef<HTMLElement | null>(null);
  const finalPlanRef = useRef<HTMLDivElement | null>(null);

  const sortedNodes = useMemo(() => sortUsageNodes(nodes), [nodes]);
  const safeDumpNodes = useMemo(() => sortedNodes.filter(isSafeDumpCandidate).slice(0, 12), [sortedNodes]);
  const selectedNodes = useMemo(() => sortUsageNodes(Object.values(selected)), [selected]);
  const allSafeDumpNodesSelected = safeDumpNodes.length > 0 && safeDumpNodes.every((node) => selected[node.id]);
  const selectedBytes = selectedNodes.reduce((total, node) => total + node.sizeBytes, 0);
  const riskCounts = useMemo(() => countRisks(selectedNodes), [selectedNodes]);
  const parentScanPath = parentPath(path);
  const canScanParent = normalizePath(path) !== "/";
  const rootChoices = useMemo(
    () =>
      [
        defaultPath,
        "/Users",
        "/private/tmp",
        "/Applications",
        "/opt/homebrew",
        "/System/Volumes/Data",
      ].filter((choice, index, paths) => choice && paths.indexOf(choice) === index),
    [defaultPath],
  );

  useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes]);

  useEffect(() => {
    if (initialSelectedNodes.length === 0) return;
    setSelected((current) => {
      const next = { ...current };
      for (const node of initialSelectedNodes) {
        next[node.id] = node;
      }
      return next;
    });
  }, [initialSelectedNodes]);

  useEffect(() => {
    setPath((current) => (current === "/Users" ? defaultPath : current));
    setManualPath((current) => (current === "/Users" ? defaultPath : current));
  }, [defaultPath]);

  useEffect(() => {
    setPath(initialPath);
    setManualPath(initialPath);
  }, [initialPath]);

  useEffect(() => {
    if (!confirmOpen) return;
    window.requestAnimationFrame(() => {
      confirmationRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }, [confirmOpen]);

  useEffect(() => {
    if (!preparedPlan) return;
    window.requestAnimationFrame(() => {
      finalPlanRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    });
  }, [preparedPlan]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listenDeepScanProgress((progress) => {
      if (progress.path === currentScanPathRef.current) {
        setScanProgress(progress);
      }
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch(() => {
        // Event listening is only available inside the Tauri runtime.
      });

    return () => {
      unlisten?.();
    };
  }, []);

  async function scan(nextPath = path) {
    if (scanInFlight.current) return;
    scanInFlight.current = true;
    currentScanPathRef.current = nextPath;
    setScanProgress(initialDeepScanProgress(nextPath));
    setScanLoading(true);
    setMessage(`Scanning ${nextPath}...`);
    setNodes([]);
    try {
      const result = await startDeepScan(nextPath);
      setPath(nextPath);
      setManualPath(nextPath);
      setNodes(result.data.entries);
      onLogs(result.logs);
      if (result.data.canceled) {
        setMessage("Scan canceled.");
      } else if (result.data.partial && hasScanWarnings(result.data.warningsSummary)) {
        setMessage(formatScanWarningSummary(result.data.warningsSummary));
      } else {
        setMessage("");
      }
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      scanInFlight.current = false;
      setScanLoading(false);
    }
  }

  async function cancelScan() {
    const result = await cancelDeepScan();
    onLogs(result.logs);
    if (result.data) {
      setMessage("Cancel requested. Stopping du...");
    }
  }

  function toggle(node: UsageNode) {
    setSelected((current) => {
      const next = { ...current };
      if (next[node.id]) {
        delete next[node.id];
      } else {
        next[node.id] = node;
      }
      return next;
    });
  }

  function openPrepareConfirmation() {
    setPreparedPlan(null);
    setFinalConfirmation("");
    setCleanupStatus("");
    setUseElevated(adminSessionUnlocked);
    setConfirmOpen(true);
  }

  async function preparePlan() {
    setLoading(true);
    setMessage("");
    setCleanupStatus("Validating selected ids against backend allowlist...");
    try {
      const result = await prepareCleanupPlan({
        itemIds: selectedNodes.map((node) => node.id),
      });
      onLogs(result.logs);
      setPreparedPlan(result.data);
      setCleanupStatus(
        result.data.items.length > 0
          ? "Safety checks finished. Review the final cleanup plan."
          : "No eligible cleanup items were prepared.",
      );
    } catch (reason) {
      setCleanupStatus(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  async function executePreparedPlan() {
    if (!preparedPlan) return;
    setLoading(true);
    setCleanupStatus("Executing final cleanup plan...");
    try {
      const rescanPath = nextPathAfterCleanup(path, preparedPlan.items.map((item) => item.path));
      const result = await executeCleanupPlan({
        planId: preparedPlan.planId,
        finalConfirmation,
        elevated: useElevated || undefined,
      });
      onLogs(result.logs);
      setMessage(formatCleanupOutcome(result.data));
      setRootContinuationId(result.data.rootContinuationId ?? "");
      setConfirmOpen(false);
      setPreparedPlan(null);
      setFinalConfirmation("");
      setSelected({});
      if (result.data.deletedBytes > 0) {
        onCleanupRecovered?.(result.data.deletedBytes);
        await scan(rescanPath);
        onRescanPath?.(rescanPath);
      }
    } catch (reason) {
      setCleanupStatus(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  async function continueAsAdmin() {
    if (!rootContinuationId) return;
    setLoading(true);
    setMessage("Requesting administrator permission...");
    try {
      const result = await executeRootCleanupContinuation(rootContinuationId);
      onLogs(result.logs);
      setMessage(formatCleanupOutcome(result.data));
      setRootContinuationId(result.data.rootContinuationId ?? "");
      if (result.data.deletedBytes > 0) {
        onCleanupRecovered?.(result.data.deletedBytes);
        await scan(path);
        onRescanPath?.(path);
      }
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
            <PathText path={path} className="mt-1 max-w-3xl text-ink-muted" />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button
              className="inline-flex min-h-9 items-center gap-1 rounded-lg bg-white px-3 text-xs font-semibold text-blue-700 ring-1 ring-blue-200 hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={loading || scanLoading || !canScanParent}
              onClick={() => void scan(parentScanPath)}
              title="Go up one level"
            >
              <ArrowUp size={14} />
              Up
            </button>
            {rootChoices.map((choice) => (
              <button
                key={choice}
                className={`min-h-9 rounded-lg px-3 text-xs font-semibold disabled:cursor-not-allowed disabled:opacity-50 ${
                  choice === "/System/Volumes/Data"
                    ? "bg-amber-100 text-amber-950 hover:bg-amber-200"
                    : "bg-slate-100 text-ink-body hover:bg-slate-200"
                }`}
                disabled={loading || scanLoading}
                onClick={() => void scan(choice)}
              >
                {shortPath(choice)}
              </button>
            ))}
            <LoadingButton loading={scanLoading} disabled={loading} onClick={() => void scan()}>
              <RefreshCw size={16} />
              Scan
            </LoadingButton>
            {scanLoading && (
              <button
                className="inline-flex min-h-10 items-center gap-1 rounded-lg bg-white px-3 text-xs font-semibold text-blue-800 ring-1 ring-blue-200 hover:bg-blue-50"
                onClick={() => void cancelScan()}
              >
                <CircleStop size={14} />
                Cancel
              </button>
            )}
          </div>
        </div>

        <form
          className="flex items-center gap-2 border-b border-slate-200 px-4 py-3"
          onSubmit={(event) => {
            event.preventDefault();
            const nextPath = normalizeManualPath(manualPath);
            if (nextPath) void scan(nextPath);
          }}
        >
          <input
            className="min-h-10 flex-1 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm text-ink-strong outline-none focus:border-blue-500 disabled:cursor-not-allowed disabled:opacity-60"
            value={manualPath}
            disabled={loading}
            onChange={(event) => setManualPath(event.target.value)}
            placeholder="/path/to/scan"
          />
          <LoadingButton loading={scanLoading} disabled={loading || !normalizeManualPath(manualPath)} type="submit">
            <RefreshCw size={16} />
            Scan Path
          </LoadingButton>
        </form>

        <div className="divide-y divide-slate-100">
          {sortedNodes.length === 0 ? (
            <div className="px-4 py-10 text-center text-sm text-ink-muted">Scan a location to find removable bottlenecks.</div>
          ) : (
            sortedNodes.map((node) => (
              <ClearRow
                key={node.path}
                node={node}
                selected={Boolean(selected[node.id])}
                onSelect={() => toggle(node)}
                onDrill={() => void scan(node.path)}
                onUnmountAndReveal={
                  onUnmountAndRevealPath
                    ? (elevated = false) => onUnmountAndRevealPath(node.path, elevated)
                    : undefined
                }
                allowProjectRootCleanup={allowProjectRootCleanup}
                appStoreMode={appStoreMode}
                disabled={loading}
                scanDisabled={loading || scanLoading}
              />
            ))
          )}
        </div>
      </section>

      {safeDumpNodes.length > 0 && (
        <section className="rounded-lg border border-emerald-200 bg-white p-4 shadow-material">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-sm font-semibold text-ink-strong">Safe To Dump</h2>
              <div className="text-sm text-ink-muted">Rebuildable caches, temp files, and generated artifacts from the current scan.</div>
            </div>
            <button
              className="min-h-9 rounded-lg bg-emerald-700 px-3 text-xs font-semibold text-white hover:bg-emerald-800 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={loading}
              onClick={() => {
                setSelected((current) => {
                  const next = { ...current };
                  for (const node of safeDumpNodes) {
                    if (allSafeDumpNodesSelected) {
                      delete next[node.id];
                    } else {
                      next[node.id] = node;
                    }
                  }
                  return next;
                });
              }}
            >
              {allSafeDumpNodesSelected ? "Unselect all" : "Select all"}
            </button>
          </div>
          <div className="mt-3 grid gap-2">
            {safeDumpNodes.map((node) => (
              <div key={node.id} className="flex items-center justify-between gap-3 rounded-lg bg-emerald-50 px-3 py-2 text-sm">
                <div className="min-w-0">
                  <PathText path={node.path} className="font-semibold text-ink-strong" />
                  <div className="mt-1 text-xs text-emerald-800">{categoryLabel(node.category)}</div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span className="font-semibold text-ink-strong">{formatBytes(node.sizeBytes)}</span>
                  <button
                    className={`rounded-lg bg-white px-2 py-1 text-xs font-semibold ring-1 hover:bg-emerald-100 disabled:cursor-not-allowed disabled:opacity-50 ${
                      selected[node.id]
                        ? "text-ink-body ring-slate-200"
                        : "text-emerald-800 ring-emerald-200"
                    }`}
                    disabled={loading}
                    onClick={() => toggle(node)}
                  >
                    {selected[node.id] ? "Unselect" : "Select"}
                  </button>
                  <button
                    className="rounded-lg bg-white px-2 py-1 text-xs font-semibold text-blue-700 ring-1 ring-slate-200 hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-50"
                    disabled={loading || scanLoading}
                    onClick={() => void scan(node.path)}
                  >
                    Drill
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

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
            className="bg-blue-700 hover:bg-blue-800"
            onClick={openPrepareConfirmation}
          >
            <ClipboardCheck size={16} />
            Prepare Cleanup
          </LoadingButton>
        </div>

        {selectedNodes.length > 0 && (
          <div className="mt-4 flex flex-wrap gap-2 text-xs font-semibold">
            {riskSummary(riskCounts).map((item) => (
              <span key={item} className="rounded-full bg-slate-100 px-3 py-1 text-ink-body">
                {item}
              </span>
            ))}
            <button
              className="ml-auto inline-flex min-h-8 items-center gap-1 rounded-lg bg-white px-3 text-xs font-semibold text-ink-body ring-1 ring-slate-200 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
              disabled={loading}
              onClick={() => setSelected({})}
            >
              <X size={14} />
              Clear selection
            </button>
          </div>
        )}

        {selectedNodes.length > 0 && (
          <div className="mt-4 grid gap-2">
            {selectedNodes.map((node) => (
              <div key={node.path} className="flex items-center justify-between gap-3 rounded-lg bg-slate-50 px-3 py-2 text-sm">
                <PathText path={node.path} className="flex-1 text-ink-strong" />
                <div className="flex shrink-0 items-center gap-2">
                  <span className="font-semibold">{formatBytes(node.sizeBytes)}</span>
                  <button
                    className="rounded-lg bg-white px-2 py-1 text-xs font-semibold text-blue-700 ring-1 ring-slate-200 hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-50"
                    disabled={loading || scanLoading}
                    onClick={() => void scan(node.path)}
                  >
                    Drill
                  </button>
                  <button
                    className="rounded-lg bg-white px-2 py-1 text-xs font-semibold text-ink-body ring-1 ring-slate-200 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50"
                    disabled={loading}
                    onClick={() => toggle(node)}
                  >
                    Unselect
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {message && (
          <div className="mt-4 rounded-lg bg-slate-50 p-3 text-sm text-ink-body">
            {scanLoading && (
              <div className="mb-2 flex items-center gap-3">
                <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-200">
                  <div className="h-full rounded-full bg-blue-700 transition-[width] duration-500" style={{ width: `${scanProgress?.percent ?? 0}%` }} />
                </div>
                <div className="w-10 text-right text-xs font-semibold text-blue-800 tabular-nums">{scanProgress?.percent ?? 0}%</div>
              </div>
            )}
            {message}
            {scanLoading && scanProgress && (
              <div className="mt-1 text-xs text-ink-muted">
                {scanProgress.totalItems > 0
                  ? t("scanStatus.progressItems", {
                      processed: scanProgress.processedItems,
                      total: scanProgress.totalItems,
                    })
                  : t("scanStatus.preparingScan")}
                {scanProgress.currentPath ? ` · ${t("scanStatus.currentPath", { path: scanProgress.currentPath })}` : ""}
              </div>
            )}
            {rootContinuationId && !appStoreMode && (
              <div className="mt-3 flex flex-wrap items-center gap-2">
                <button
                  className="min-h-9 rounded-lg bg-slate-900 px-3 text-xs font-semibold text-white hover:bg-black"
                  disabled={loading}
                  onClick={() => void continueAsAdmin()}
                >
                  Continue as Admin
                </button>
                <span className="text-xs text-ink-muted">macOS will ask for an administrator password.</span>
              </div>
            )}
          </div>
        )}
      </section>

      {confirmOpen && (
        <section ref={confirmationRef} className="rounded-lg border border-amber-200 bg-amber-50 p-4 shadow-material">
          <div className="grid gap-4 md:grid-cols-[1fr_320px]">
            <div>
              <div className="flex items-center gap-2 text-sm font-semibold text-amber-950">
                <AlertTriangle size={18} />
                Confirm Preparation
              </div>
              <p className="mt-2 text-sm text-amber-950">
                This first confirmation prepares a cleanup plan for {selectedNodes.length} selected item(s), approximately {formatBytes(selectedBytes)}.
              </p>
              <p className="mt-2 text-sm text-amber-950">
                Nothing is deleted during preparation. The backend validates selected IDs, checks actions and paths, and calculates the final recoverable size.
              </p>

              {cleanupStatus && (
                <div className="mt-3 rounded-lg bg-white/70 p-3 text-sm text-amber-950">
                  {loading && (
                    <div className="mb-2 h-2 overflow-hidden rounded-full bg-amber-100">
                      <div className="indeterminate-progress h-full rounded-full bg-amber-700" />
                    </div>
                  )}
                  {cleanupStatus}
                </div>
              )}

              {preparedPlan && (
                <div ref={finalPlanRef} className="mt-4 rounded-lg bg-white p-3 text-sm text-ink-body">
                  <div className="font-semibold text-ink-strong">Final Plan</div>
                  <div className="mt-1">
                    {preparedPlan.items.length} eligible item(s), {formatBytes(preparedPlan.estimatedRecoverableBytes)}
                  </div>
                  {preparedPlan.warnings.length > 0 && (
                    <div className="mt-2 grid gap-1 text-xs text-amber-900">
                      {preparedPlan.warnings.map((warning) => (
                        <div key={warning}>{warning}</div>
                      ))}
                    </div>
                  )}
                  {preparedPlan.items.length > 0 && (
                    <div className="mt-3 grid gap-2">
                      {preparedPlan.items.map((item) => (
                        <PreparedPlanRow key={item.id} item={item} />
                      ))}
                    </div>
                  )}
                  <div className="mt-3 text-xs font-semibold text-ink-strong">
                    Type `{preparedPlan.finalConfirmationPhrase}` for final deletion.
                  </div>
                  <input
                    className="mt-2 min-h-10 w-full rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm text-ink-strong outline-none focus:border-blue-500"
                    value={finalConfirmation}
                    onChange={(event) => setFinalConfirmation(event.target.value)}
                  />
                  {!appStoreMode && (
                    <label className="mt-3 flex cursor-pointer items-center gap-2 text-xs font-semibold text-ink-body select-none">
                      <input
                        type="checkbox"
                        className="h-4 w-4 accent-slate-900"
                        checked={useElevated}
                        onChange={(event) => setUseElevated(event.target.checked)}
                      />
                      {adminSessionUnlocked ? "Use Admin Mode for this cleanup" : "Request administrator privileges"}
                      <span className="font-normal text-ink-muted">
                        {adminSessionUnlocked
                          ? "(cleenosx will prefer admin cleanup; macOS may still re-prompt later)"
                          : "(macOS will ask for your password)"}
                      </span>
                    </label>
                  )}
                </div>
              )}
            </div>

            <div className="flex flex-wrap items-end justify-end gap-2">
              <button
                className="min-h-10 rounded-lg bg-white px-4 text-sm font-semibold text-ink-body ring-1 ring-slate-300 hover:bg-slate-50"
                disabled={loading}
                onClick={() => setConfirmOpen(false)}
              >
                Cancel
              </button>
              {!preparedPlan ? (
                <LoadingButton loading={loading} className="bg-amber-700 hover:bg-amber-800" onClick={() => void preparePlan()}>
                  <ClipboardCheck size={16} />
                  Prepare Plan
                </LoadingButton>
              ) : (
                <LoadingButton
                  loading={loading}
                  disabled={preparedPlan.items.length === 0 || finalConfirmation !== preparedPlan.finalConfirmationPhrase}
                  className="bg-red-700 hover:bg-red-800"
                  onClick={() => void executePreparedPlan()}
                >
                  <Trash2 size={16} />
                  Delete Prepared Items
                </LoadingButton>
              )}
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
  onUnmountAndReveal,
  allowProjectRootCleanup,
  appStoreMode,
  disabled,
  scanDisabled,
}: {
  node: UsageNode;
  selected: boolean;
  onSelect: () => void;
  onDrill: () => void;
  onUnmountAndReveal?: (elevated?: boolean) => void;
  allowProjectRootCleanup: boolean;
  appStoreMode: boolean;
  disabled: boolean;
  scanDisabled: boolean;
}) {
  const selectable =
    node.risk !== "readOnlySystem" &&
    (node.risk !== "dangerous" || (allowProjectRootCleanup && isProjectPath(node.path))) &&
    !isAssetsV2Area(node.path) &&
    !isBroadTarget(node.path);
  const showUnmountReveal = !appStoreMode && node.risk === "readOnlySystem" && onUnmountAndReveal;
  const KindIcon = node.kind === "file" ? File : FolderOpen;
  const kindLabel = node.kind === "file" ? "File" : "Directory";

  return (
    <div className="grid min-h-16 grid-cols-[36px_1fr_132px_124px_240px] items-center gap-3 px-4 py-3 text-sm">
      <input
        type="checkbox"
        className="h-4 w-4"
        checked={selected}
        disabled={!selectable || disabled}
        onChange={onSelect}
        title={selectable ? "Select for removal" : "This target is protected by risk rules"}
      />
      <div className="flex min-w-0 items-center gap-2">
        <button
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-blue-700 hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-40"
          disabled={scanDisabled}
          onClick={onDrill}
          title={node.kind === "file" ? "Scan file" : "Drill down"}
        >
          <KindIcon size={16} />
        </button>
        <div className="min-w-0">
          <PathText path={node.path} className="text-ink-strong" />
          <div className="mt-1 text-xs text-ink-muted">
            {kindLabel} · {categoryLabel(node.category)}
          </div>
        </div>
      </div>
      <div className="text-right font-semibold text-ink-strong">{formatBytes(node.sizeBytes)}</div>
      <RiskChip risk={node.risk} />
      <div className="flex justify-end gap-2">
        {showUnmountReveal && (
          <button
            className="inline-flex min-h-8 items-center gap-1 rounded-lg bg-amber-100 px-2.5 text-xs font-semibold text-amber-950 hover:bg-amber-200 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={disabled}
            onClick={() => onUnmountAndReveal(false)}
            title="Unmount this read-only mount and scan the revealed folder"
          >
            <Unplug size={13} />
            Unmount
          </button>
        )}
        {showUnmountReveal && (
          <button
            className="inline-flex min-h-8 items-center gap-1 rounded-lg bg-slate-900 px-2.5 text-xs font-semibold text-white hover:bg-black disabled:cursor-not-allowed disabled:opacity-40"
            disabled={disabled}
            onClick={() => onUnmountAndReveal(true)}
            title="Unmount with administrator permission and scan the revealed folder"
          >
            <ShieldCheck size={13} />
            Admin
          </button>
        )}
        <button
          className="inline-flex min-h-8 items-center justify-end gap-1 rounded-lg bg-white px-2.5 text-xs font-semibold text-blue-700 ring-1 ring-slate-200 hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-40"
          disabled={scanDisabled}
          onClick={onDrill}
        >
          Open
          <ChevronRight size={14} />
        </button>
      </div>
    </div>
  );
}

function basename(path: string) {
  return path.split("/").filter(Boolean).pop() ?? path;
}

function nextPathAfterCleanup(currentPath: string, deletedPaths: string[]) {
  const current = normalizePath(currentPath);
  const deleted = deletedPaths.map(normalizePath);

  for (const deletedPath of deleted) {
    if (current === deletedPath || current.startsWith(`${deletedPath}/`)) {
      return parentPath(deletedPath);
    }
  }

  const common = commonParentPath(deleted);
  return common === "/" ? parentPath(deleted[0] ?? current) : common;
}

function commonParentPath(paths: string[]) {
  if (paths.length === 0) return "/";
  const parents = paths.map(parentPath).map((path) => path.split("/").filter(Boolean));
  const common: string[] = [];
  for (let index = 0; index < parents[0].length; index += 1) {
    const part = parents[0][index];
    if (parents.every((parent) => parent[index] === part)) {
      common.push(part);
    } else {
      break;
    }
  }
  return common.length === 0 ? "/" : `/${common.join("/")}`;
}

function parentPath(path: string) {
  const normalized = normalizePath(path);
  if (normalized === "/") return "/";
  const parts = normalized.split("/").filter(Boolean);
  parts.pop();
  return parts.length === 0 ? "/" : `/${parts.join("/")}`;
}

function normalizePath(path: string) {
  const trimmed = path.trim();
  if (trimmed === "") return "/";
  return trimmed.length > 1 ? trimmed.replace(/\/+$/, "") : trimmed;
}

function normalizeManualPath(path: string) {
  const trimmed = path.trim();
  if (!trimmed.startsWith("/") || trimmed.includes("\n") || trimmed.includes("\0")) {
    return "";
  }
  return trimmed;
}

function countRisks(nodes: UsageNode[]) {
  return nodes.reduce<Record<RiskLevel, number>>(
    (counts, node) => {
      counts[node.risk] += 1;
      return counts;
    },
    {
      safeToAnalyze: 0,
      attention: 0,
      reviewRequired: 0,
      dangerous: 0,
      readOnlySystem: 0,
    },
  );
}

function riskSummary(counts: Record<RiskLevel, number>) {
  return [
    counts.safeToAnalyze > 0 ? `${counts.safeToAnalyze} safe` : "",
    counts.attention > 0 ? `${counts.attention} attention` : "",
    counts.reviewRequired > 0 ? `${counts.reviewRequired} review` : "",
    counts.dangerous > 0 ? `${counts.dangerous} danger` : "",
    counts.readOnlySystem > 0 ? `${counts.readOnlySystem} read-only` : "",
  ].filter(Boolean);
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

function isSafeDumpCandidate(node: UsageNode) {
  if (node.risk === "dangerous" || node.risk === "readOnlySystem" || isBroadTarget(node.path) || isAssetsV2Area(node.path)) {
    return false;
  }
  if (TEST_DELETE_PATHS.has(node.path)) {
    return true;
  }
  if (node.category === "rustArtifacts" || node.category === "nodeCaches" || node.category === "caches") {
    return true;
  }
  return (
    node.path.startsWith("/private/tmp/") ||
    node.path.endsWith("/target") ||
    node.path.includes("/target/") ||
    node.path.endsWith("/node_modules") ||
    node.path.includes("/node_modules/")
  );
}

function shortPath(path: string) {
  if (path === "/System/Volumes/Data") return "System Data";
  if (path === "/private/tmp") return "tmp";
  return basename(path);
}

function PreparedPlanRow({ item }: { item: PreparedCleanupItem }) {
  const actionProfile = item.actionProfile;
  const KindIcon = item.kind === "file" ? File : FolderOpen;
  const kindLabel = item.kind === "file" ? "File" : "Directory";

  return (
    <div className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="flex min-w-0 items-start gap-2">
          <div
            className={`mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg ring-1 ${
              item.kind === "file"
                ? "bg-white text-ink-muted ring-slate-200"
                : "bg-blue-50 text-blue-700 ring-blue-200"
            }`}
            title={kindLabel}
          >
            <KindIcon size={14} />
          </div>
          <div className="min-w-0">
            <PathText path={item.path} className="font-semibold text-ink-strong" />
            <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-ink-muted">
              <span>{kindLabel}</span>
              <span>{categoryLabel(item.category)}</span>
              <span>{formatBytes(item.estimatedBytes)}</span>
              {actionProfile && <span>{actionProfile.deleteCapability.userFacingLevel}</span>}
            </div>
          </div>
        </div>
        <RiskChip risk={item.risk} />
      </div>
      <div className="mt-2 text-xs text-ink-body">{actionProfile?.deleteCapability.userFacingSummary ?? item.reason}</div>
      {actionProfile && (
        <ActionScoreSummary actionProfile={actionProfile} includeNextAction compact className="mt-2" />
      )}
    </div>
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

function formatCleanupOutcome(outcome: CleanupOutcome) {
  const rootFailures = outcome.failedItems.filter((item) => item.needsRoot);
  if (rootFailures.length > 0) {
    return `${outcome.message} ${rootFailures.length} item(s) need root or Recovery cleanup. First: ${rootFailures[0].path}: ${rootFailures[0].message}`;
  }

  const firstFailure = outcome.failedItems[0];
  if (!firstFailure) {
    return outcome.message;
  }
  return `${outcome.message} First failure: ${firstFailure.path}: ${firstFailure.message}`;
}

function initialDeepScanProgress(path: string): DeepScanProgress {
  return {
    path,
    currentPath: null,
    processedItems: 0,
    totalItems: 0,
    percent: 0,
    canceled: false,
    finished: false,
  };
}
