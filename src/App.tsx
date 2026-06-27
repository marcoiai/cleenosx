import {
  Activity,
  CircleStop,
  Database,
  FolderTree,
  HardDrive,
  RotateCw,
  Settings,
  ShieldCheck,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { FindingsList } from "./components/FindingsList";
import { LanguageSelector } from "./components/LanguageSelector";
import { LoadingButton } from "./components/LoadingButton";
import { LogsPanel } from "./components/LogsPanel";
import { ProgressBar } from "./components/ProgressBar";
import { RecoveryPanel } from "./components/RecoveryPanel";
import { ReviewPanel } from "./components/ReviewPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { StatPanel } from "./components/StatPanel";
import { VolumeTable } from "./components/VolumeTable";
import { categoryLabel, formatBytes, riskSortRank } from "./format";
import { useI18n } from "./i18n";
import { formatScanWarningSummary, hasScanWarnings, shouldOfferFullDiskAccess } from "./scanWarnings";
import {
  cancelDeepScan,
  getAdminSessionStatus,
  getCleanupSettings,
  getDefaultScanPath,
  getStorageOverview,
  listenDeepScanProgress,
  lockAdminSession,
  listSnapshots,
  mountVolume,
  scanAssetsV2,
  scanContainers,
  scanDeveloperTools,
  scanRustArtifacts,
  scanVolumes,
  openFullDiskAccessSettings,
  startDeepScan,
  unmountVolume,
  unlockAdminSession,
  updateCleanupSettings,
} from "./tauri";
import type { AdminSessionStatus, CleanupSettings, DeepScanProgress, DeepScanWarningsSummary, Finding, Overview, ScanLog, UsageNode, VolumeInfo } from "./types";

type View = "dashboard" | "volumes" | "scanner" | "findings" | "recovery" | "settings";
type ScanState = "idle" | "loadingOverview" | "ready" | "deepScanRunning" | "deepScanPartial" | "deepScanCanceled" | "deepScanFailed";
type ThemeMode = "light" | "black";
const THEME_STORAGE_KEY = "cleanerx.themeMode.v2";
const TOTAL_RECOVERED_STORAGE_KEY = "cleenosx.totalRecoveredBytes.v1";
const IS_APP_STORE_BUILD = import.meta.env.VITE_CLEANERX_DISTRIBUTION === "app-store";

const emptyOverview: Overview = {
  summary: {},
  volumes: [],
  usageRoots: [],
  findings: [],
};

const defaultAdminSession: AdminSessionStatus = {
  unlocked: false,
  available: !IS_APP_STORE_BUILD,
  lastUnlockedAtMs: null,
  message: IS_APP_STORE_BUILD
    ? "Admin Mode is unavailable in the App Store build."
    : "Admin Mode is off. Unlock once to reuse administrator cleanup through this app session.",
};

const navItems: Array<{ id: View; labelKey: string; icon: typeof Activity }> = [
  { id: "dashboard", labelKey: "nav.dashboard", icon: Activity },
  { id: "volumes", labelKey: "nav.volumes", icon: HardDrive },
  { id: "scanner", labelKey: "nav.scanner", icon: FolderTree },
  { id: "findings", labelKey: "nav.findings", icon: Database },
  ...(IS_APP_STORE_BUILD ? [] : [{ id: "recovery" as const, labelKey: "nav.recovery", icon: TerminalSquare }]),
  { id: "settings", labelKey: "nav.settings", icon: Settings },
];
const TEST_DELETE_PATHS = new Set([
  "/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/cleanerx-delete-me-test.bin",
  "/private/tmp/Fake",
  "/private/tmp/Fake/test.txt",
]);

function App() {
  const { t } = useI18n();
  const [activeView, setActiveView] = useState<View>("dashboard");
  const [overview, setOverview] = useState<Overview>(emptyOverview);
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [usage, setUsage] = useState<UsageNode[]>([]);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [logs, setLogs] = useState<ScanLog[]>([]);
  const [cleanupSettings, setCleanupSettings] = useState<CleanupSettings>({ allowProjectRootCleanup: false });
  const [adminSession, setAdminSession] = useState<AdminSessionStatus>(defaultAdminSession);
  const [adminLoading, setAdminLoading] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [scanState, setScanState] = useState<ScanState>("idle");
  const [deepScanProgress, setDeepScanProgress] = useState<DeepScanProgress | null>(null);
  const [deepScanWarnings, setDeepScanWarnings] = useState<DeepScanWarningsSummary | null>(null);
  const [defaultScanPath, setDefaultScanPath] = useState("/Users");
  const [deepScanPath, setDeepScanPath] = useState("/Users");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => readThemeMode());
  const [totalRecoveredBytes, setTotalRecoveredBytes] = useState(() => readStoredBytes(TOTAL_RECOVERED_STORAGE_KEY));
  const scanInFlight = useRef(false);
  const overviewInFlight = useRef(false);
  const scanButtonLoading = loading || scanState === "loadingOverview" || scanState === "deepScanRunning";

  const allFindings = useMemo(() => {
    const byKey = new Map<string, Finding>();
    [...overview.findings, ...findings].forEach((finding) => {
      byKey.set(`${finding.title}-${finding.path ?? finding.reason}`, finding);
    });
    return Array.from(byKey.values()).sort(
      (left, right) =>
        riskSortRank(left.risk) - riskSortRank(right.risk) ||
        (right.sizeBytes ?? -1) - (left.sizeBytes ?? -1) ||
        left.title.localeCompare(right.title),
    );
  }, [findings, overview.findings]);

  const visibleUsage = useMemo(() => {
    const source = usage.length ? usage : overview.usageRoots;
    return [...source].sort(
      (left, right) =>
        riskSortRank(left.risk) - riskSortRank(right.risk) ||
        Number(TEST_DELETE_PATHS.has(right.path)) - Number(TEST_DELETE_PATHS.has(left.path)) ||
        right.sizeBytes - left.sizeBytes ||
        left.path.localeCompare(right.path),
    );
  }, [overview.usageRoots, usage]);

  useEffect(() => {
    void getDefaultScanPath().then((path) => {
      setDefaultScanPath(path);
      setDeepScanPath((current) => (current === "/Users" ? path : current));
    });
    void getCleanupSettings().then(setCleanupSettings);
    if (!IS_APP_STORE_BUILD) {
      void getAdminSessionStatus().then(setAdminSession);
    }
    const overviewTimer = window.setTimeout(() => {
      void runOverview({ background: true });
    }, 80);
    return () => window.clearTimeout(overviewTimer);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listenDeepScanProgress((progress) => {
      setDeepScanProgress(progress);
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

  useEffect(() => {
    document.documentElement.classList.toggle("black", themeMode === "black");
    document.documentElement.style.colorScheme = themeMode === "black" ? "dark" : "light";
    writeThemeMode(themeMode);
  }, [themeMode]);

  useEffect(() => {
    writeStoredBytes(TOTAL_RECOVERED_STORAGE_KEY, totalRecoveredBytes);
  }, [totalRecoveredBytes]);

  async function runOverview(options: { background?: boolean } = {}) {
    if (overviewInFlight.current) return;
    overviewInFlight.current = true;
    setScanState((current) => (current === "deepScanRunning" ? current : "loadingOverview"));
    setDeepScanWarnings(null);
    const applyOverview = (result: Awaited<ReturnType<typeof getStorageOverview>>) => {
      setOverview(result.data);
      setVolumes(result.data.volumes);
      setFindings((current) => (current.length ? current : result.data.findings));
      pushLogs(result.logs);
    };
    const finishOverview = () => {
      overviewInFlight.current = false;
      setScanState((current) => (current === "loadingOverview" ? "ready" : current));
    };

    if (options.background) {
      setError("");
      try {
        applyOverview(await getStorageOverview());
      } catch (reason) {
        const message = reason instanceof Error ? reason.message : String(reason);
        setError(message);
        pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "error", message }]);
      } finally {
        finishOverview();
      }
      return;
    }

    try {
      await withLoading(async () => {
        applyOverview(await getStorageOverview());
      });
    } finally {
      finishOverview();
    }
  }

  async function runVolumes() {
    setDeepScanWarnings(null);
    await withLoading(async () => {
      const result = await scanVolumes();
      setVolumes(result.data);
      pushLogs(result.logs);
    });
  }

  async function runDataUsage() {
    await runDeepScan(deepScanPath);
  }

  async function runDeepScan(path: string, elevated = false) {
    if (scanInFlight.current || scanState === "deepScanRunning") return;
    scanInFlight.current = true;
    setDeepScanPath(path);
    setScanState("deepScanRunning");
    setDeepScanProgress(initialDeepScanProgress(path));
    setDeepScanWarnings(null);
    setError("");
    try {
      const result = await startDeepScan(path, elevated);
      setUsage(result.data.entries);
      setDeepScanWarnings(result.data.warningsSummary);
      pushLogs(result.logs);
      setScanState(
        result.data.canceled
          ? "deepScanCanceled"
          : result.data.partial || hasScanWarnings(result.data.warningsSummary)
            ? "deepScanPartial"
            : "ready",
      );
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setError(message);
      setScanState("deepScanFailed");
      pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "error", message }]);
    } finally {
      scanInFlight.current = false;
    }
  }

  async function mountVolumeAndScan(identifier: string, elevated = false) {
    let mountPoint = "";
    await withLoading(async () => {
      const result = await mountVolume(identifier, elevated);
      setVolumes(result.data.volumes);
      pushLogs(result.logs);

      mountPoint = result.data.mountPoint ?? findVolumeByIdentifier(result.data.volumes, identifier)?.mountPoint ?? "";
      if (!mountPoint) {
        const message = `Mounted ${identifier}, but no mount point was reported. Rescan volumes and try again.`;
        setError(message);
        pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "warning", message }]);
      }
    });

    if (mountPoint) {
      setActiveView("scanner");
      await runDeepScan(mountPoint, elevated);
    }
  }

  async function unmountSelectedVolume(identifier: string, elevated = false) {
    let revealPath = "";
    await withLoading(async () => {
      const result = await unmountVolume(identifier, elevated);
      setVolumes(result.data.volumes);
      pushLogs(result.logs);
      revealPath = result.data.mountPoint ?? "";
    });

    if (revealPath) {
      setActiveView("scanner");
      await runDeepScan(revealPath, elevated);
    }
  }

  async function unmountPathAndReveal(path: string, elevated = false) {
    let revealPath = "";
    await withLoading(async () => {
      const result = await unmountVolume(path, elevated);
      setVolumes(result.data.volumes);
      pushLogs(result.logs);

      revealPath = result.data.mountPoint ?? "";
      if (!revealPath) {
        const message = `Unmount did not reveal ${path}. The mount may still be active.`;
        setError(message);
        pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "warning", message }]);
      }
    });

    if (revealPath) {
      setActiveView("scanner");
      await runDeepScan(revealPath, elevated);
    }
  }

  async function cancelRunningDeepScan() {
    const result = await cancelDeepScan();
    pushLogs(result.logs);
    if (result.data) {
      setScanState("deepScanCanceled");
    }
  }

  function scanPath(path: string) {
    if (loading || scanInFlight.current) return;
    setActiveView("scanner");
    void runDeepScan(path);
  }

  function addRecoveredBytes(deletedBytes: number) {
    if (!Number.isFinite(deletedBytes) || deletedBytes <= 0) return;
    setTotalRecoveredBytes((current) => current + Math.floor(deletedBytes));
  }

  async function updateSettings(nextSettings: CleanupSettings) {
    const saved = await updateCleanupSettings(nextSettings);
    setCleanupSettings(saved);
  }

  async function toggleAdminSession() {
    if (IS_APP_STORE_BUILD) return;
    setAdminLoading(true);
    setError("");
    try {
      const status = adminSession.unlocked ? await lockAdminSession() : await unlockAdminSession();
      setAdminSession(status);
      pushLogs([
        {
          timestamp: Math.floor(Date.now() / 1000),
          level: "info",
          message: status.unlocked ? "Admin Mode enabled for this app session." : "Admin Mode disabled for this app session.",
        },
      ]);
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setError(message);
      pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "error", message }]);
    } finally {
      setAdminLoading(false);
    }
  }

  async function runFindingsScan() {
    setDeepScanWarnings(null);
    await withLoading(async () => {
      const results = [];
      results.push(await scanAssetsV2());
      results.push(await scanDeveloperTools());
      results.push(await scanRustArtifacts());
      results.push(await scanContainers());
      results.push(await listSnapshots());
      setFindings(results.flatMap((result) => result.data));
      pushLogs(results.flatMap((result) => result.logs));
    });
  }

  async function withLoading(work: () => Promise<void>) {
    if (scanInFlight.current) {
      return;
    }
    scanInFlight.current = true;
    setLoading(true);
    setError("");
    try {
      await work();
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setError(message);
      setScanState((current) => (current === "deepScanRunning" ? "deepScanFailed" : current));
      pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "error", message }]);
    } finally {
      scanInFlight.current = false;
      setLoading(false);
    }
  }

  function pushLogs(nextLogs: ScanLog[]) {
    setLogs((current) => {
      const merged = [...current];
      for (const log of nextLogs) {
        const previous = merged[merged.length - 1];
        if (previous?.level === log.level && previous.message === log.message) {
          continue;
        }
        merged.push(log);
      }
      return merged.slice(-160);
    });
  }

  function openView(view: View) {
    setActiveView(view);
    if (!scanInFlight.current && !hasDataForView(view)) {
      void scanForView(view);
    }
  }

  function hasDataForView(view: View) {
    switch (view) {
      case "dashboard":
        return overview.summary.totalBytes != null || overview.volumes.length > 0 || overview.findings.length > 0;
      case "volumes":
        return volumes.length > 0;
      case "scanner":
        return usage.length > 0 || overview.usageRoots.length > 0;
      case "findings":
        return allFindings.length > 0;
      default:
        return true;
    }
  }

  function scanForView(view: View) {
    switch (view) {
      case "dashboard":
        return runOverview();
      case "volumes":
        return runVolumes();
      case "scanner":
        return runDataUsage();
      case "findings":
        return runFindingsScan();
      default:
        return Promise.resolve();
    }
  }

  function scanActionForView(view: View) {
    return {
      dashboard: () => runOverview(),
      volumes: () => runVolumes(),
      scanner: () => runDataUsage(),
      findings: () => runFindingsScan(),
      recovery: () => runOverview(),
      settings: () => runOverview(),
    }[view];
  }

  return (
    <div className="min-h-screen bg-surface-base text-ink-body">
      <div className="grid min-h-screen grid-cols-[248px_1fr]">
        <aside className="border-r border-slate-200 bg-white">
          <div className="border-b border-slate-200 px-5 py-5">
            <div className="text-xl font-semibold text-ink-strong">cleenosx</div>
            <div className="mt-1 text-xs font-medium text-red-700">{t("app.tagline")}</div>
          </div>
          <nav className="p-3">
            {navItems.map((item) => {
              const Icon = item.icon;
              const active = activeView === item.id;
              return (
                <button
                  key={item.id}
                  className={`mb-1 flex min-h-11 w-full items-center gap-3 rounded-lg px-3 text-left text-sm font-semibold transition ${
                    active ? "bg-blue-50 text-blue-800" : "text-ink-body hover:bg-slate-100"
                  } disabled:cursor-not-allowed disabled:opacity-50`}
                  onClick={() => openView(item.id)}
                >
                  <Icon size={18} />
                  {t(item.labelKey)}
                </button>
              );
            })}
          </nav>
        </aside>

        <main className="min-w-0">
          <header className="flex items-center justify-between gap-4 border-b border-slate-200 bg-white px-6 py-4">
            <div>
              <h1 className="text-xl font-semibold text-ink-strong">{t(titleKeyForView(activeView))}</h1>
              <div className="mt-1 text-sm text-ink-muted">{t(subtitleKeyForView(activeView))}</div>
            </div>
            <div className="flex items-center gap-3">
              <LanguageSelector variant="toolbar" />
              {!IS_APP_STORE_BUILD && (
                <button
                  className={`inline-flex min-h-10 items-center justify-center gap-2 rounded-lg px-4 text-sm font-semibold shadow-sm transition ${
                    adminSession.unlocked
                      ? "bg-emerald-50 text-emerald-800 ring-1 ring-emerald-200 hover:bg-emerald-100"
                      : "bg-white text-amber-900 ring-1 ring-amber-200 hover:bg-amber-50"
                  } disabled:cursor-not-allowed disabled:opacity-60`}
                  disabled={adminLoading}
                  onClick={() => void toggleAdminSession()}
                >
                  <ShieldCheck size={16} />
                  {adminLoading ? "Authorizing..." : adminSession.unlocked ? "Admin Ready" : "Unlock Admin"}
                </button>
              )}
              <LoadingButton loading={scanButtonLoading} onClick={scanActionForView(activeView)}>
                <RotateCw size={16} />
                {t("common.scan")}
              </LoadingButton>
              {scanState === "deepScanRunning" && (
                <button
                  className="inline-flex min-h-10 items-center justify-center gap-2 rounded-lg bg-white px-4 text-sm font-semibold text-blue-800 shadow-sm ring-1 ring-blue-200 transition hover:bg-blue-50"
                  onClick={() => void cancelRunningDeepScan()}
                >
                  <CircleStop size={16} />
                  {t("scanStatus.cancelScan")}
                </button>
              )}
            </div>
          </header>

          <div className="p-6">
            {error && <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}
            <ScanStatus
              state={scanState}
              warnings={deepScanWarnings}
              progress={deepScanProgress}
              onCancel={cancelRunningDeepScan}
              onOpenPermissions={() => void openFullDiskAccessSettings()}
              appStoreMode={IS_APP_STORE_BUILD}
            />

            {activeView === "dashboard" && (
              <div className="grid gap-6">
                <Dashboard overview={overview} totalRecoveredBytes={totalRecoveredBytes} />
                <div className="grid grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)] gap-6">
                  <FindingsList findings={allFindings.slice(0, 5)} onScanPath={scanPath} disabled={scanButtonLoading} />
                  <LogsPanel logs={logs} />
                </div>
              </div>
            )}

            {activeView === "volumes" && (
              <div className="grid gap-6">
                <VolumeTable
                  volumes={volumes}
                  onMountAndScan={(identifier) => void mountVolumeAndScan(identifier)}
                  onMountAndScanElevated={(identifier) => void mountVolumeAndScan(identifier, true)}
                  onScanPath={scanPath}
                  onUnmount={(identifier) => void unmountSelectedVolume(identifier)}
                  onUnmountElevated={(identifier) => void unmountSelectedVolume(identifier, true)}
                  disabled={loading}
                  scanDisabled={scanButtonLoading}
                />
                <LogsPanel logs={logs} />
              </div>
            )}

            {activeView === "scanner" && (
              <ReviewPanel
                initialNodes={visibleUsage}
                defaultPath={defaultScanPath}
                initialPath={deepScanPath}
                allowProjectRootCleanup={cleanupSettings.allowProjectRootCleanup}
                appStoreMode={IS_APP_STORE_BUILD}
                adminSessionUnlocked={adminSession.unlocked}
                onLogs={pushLogs}
                onCleanupRecovered={addRecoveredBytes}
                onRescanPath={(path) => void runDeepScan(path)}
                onUnmountAndRevealPath={(path, elevated) => void unmountPathAndReveal(path, elevated)}
              />
            )}

            {activeView === "findings" && (
              <div className="grid gap-6">
                <FindingsList findings={allFindings} onScanPath={scanPath} disabled={scanButtonLoading} />
                <LogsPanel logs={logs} />
              </div>
            )}
            {activeView === "recovery" && !IS_APP_STORE_BUILD && <RecoveryPanel />}
            {activeView === "settings" && (
              <SettingsPanel
                cleanupSettings={cleanupSettings}
                adminSession={adminSession}
                adminLoading={adminLoading}
                themeMode={themeMode}
                onCleanupSettingsChange={(settings) => void updateSettings(settings)}
                onThemeModeChange={setThemeMode}
                onToggleAdminSession={() => void toggleAdminSession()}
              />
            )}
          </div>
        </main>
      </div>
    </div>
  );
}

function Dashboard({ overview, totalRecoveredBytes }: { overview: Overview; totalRecoveredBytes: number }) {
  const { t } = useI18n();
  const freeBytes = overview.summary.availableBytes;
  const freeTone = freeBytes != null && freeBytes < 10 * 1024 ** 3 ? "bad" : freeBytes != null && freeBytes < 15 * 1024 ** 3 ? "warn" : "good";

  const roleCount = new Map<string, number>();
  overview.volumes.forEach((volume) => {
    const role = volume.role ?? t("common.unknown");
    roleCount.set(role, (roleCount.get(role) ?? 0) + 1);
  });

  return (
    <section className="grid gap-6">
      <div className="grid grid-cols-2 gap-4 xl:grid-cols-5">
        <StatPanel label={t("dashboard.total")} value={formatBytes(overview.summary.totalBytes)} />
        <StatPanel label={t("dashboard.used")} value={formatBytes(overview.summary.usedBytes)} />
        <StatPanel label={t("dashboard.free")} value={formatBytes(overview.summary.availableBytes)} tone={freeTone} />
        <StatPanel label={t("dashboard.recovered")} value={formatBytes(totalRecoveredBytes)} tone={totalRecoveredBytes > 0 ? "good" : "neutral"} />
        <StatPanel label={t("dashboard.apfsVolumes")} value={String(overview.volumes.length)} />
      </div>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold text-ink-strong">{t("dashboard.primaryStorage")}</h2>
          <span className="text-sm font-semibold text-ink-muted">{overview.summary.percentUsed?.toFixed(1) ?? "0.0"}%</span>
        </div>
        <ProgressBar value={overview.summary.percentUsed} />
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <h2 className="text-sm font-semibold text-ink-strong">{t("dashboard.detectedRoles")}</h2>
        <div className="mt-3 flex flex-wrap gap-2">
          {Array.from(roleCount.entries()).map(([role, count]) => (
            <span key={role} className="rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-ink-body">
              {role}: {count}
            </span>
          ))}
          {roleCount.size === 0 && <span className="text-sm text-ink-muted">{t("dashboard.noRoles")}</span>}
        </div>
      </section>
    </section>
  );
}

function ScanStatus({
  state,
  warnings,
  progress,
  onCancel,
  onOpenPermissions,
  appStoreMode,
}: {
  state: ScanState;
  warnings: DeepScanWarningsSummary | null;
  progress: DeepScanProgress | null;
  onCancel: () => void;
  onOpenPermissions: () => void;
  appStoreMode: boolean;
}) {
  const { t } = useI18n();
  if (state === "idle") {
    return null;
  }

  const displayState = state === "ready" && warnings && hasScanWarnings(warnings) ? "deepScanPartial" : state;

  const meta: Record<
    ScanState,
    {
      label: string;
      emoji: string;
      labelClassName: string;
      iconClassName: string;
      barClassName: string;
      chipClassName: string;
      detail?: string;
    }
  > = {
    idle: {
      label: "",
      emoji: "",
      labelClassName: "",
      iconClassName: "",
      barClassName: "",
      chipClassName: "",
    },
    loadingOverview: {
      label: t("scanStatus.loadingOverview"),
      emoji: "🔎",
      labelClassName: "text-blue-800",
      iconClassName: "bg-white text-blue-800 ring-blue-200",
      barClassName: "bg-blue-700",
      chipClassName: "bg-white text-blue-800 ring-blue-200",
      detail: t("scanStatus.loadingOverviewDetail"),
    },
    ready: {
      label: t("scanStatus.ready"),
      emoji: "✅",
      labelClassName: "text-emerald-800",
      iconClassName: "bg-white text-emerald-800 ring-emerald-200",
      barClassName: "bg-emerald-700",
      chipClassName: "bg-white text-emerald-800 ring-emerald-200",
    },
    deepScanRunning: {
      label: t("scanStatus.deepScanRunning"),
      emoji: "📊",
      labelClassName: "text-blue-800",
      iconClassName: "bg-white text-blue-800 ring-blue-200",
      barClassName: "bg-blue-700",
      chipClassName: "bg-white text-blue-800 ring-blue-200",
      detail: t("scanStatus.deepScanDetail"),
    },
    deepScanPartial: {
      label: t("scanStatus.deepScanPartial"),
      emoji: "⚠️",
      labelClassName: "text-amber-900",
      iconClassName: "bg-white text-amber-900 ring-amber-200",
      barClassName: "bg-amber-600",
      chipClassName: "bg-white text-amber-900 ring-amber-200",
    },
    deepScanCanceled: {
      label: t("scanStatus.deepScanCanceled"),
      emoji: "⏹️",
      labelClassName: "text-ink-body",
      iconClassName: "bg-slate-100 text-slate-800 ring-slate-200",
      barClassName: "bg-slate-500",
      chipClassName: "bg-slate-100 text-slate-800 ring-slate-200",
    },
    deepScanFailed: {
      label: t("scanStatus.deepScanFailed"),
      emoji: "❌",
      labelClassName: "text-red-800",
      iconClassName: "bg-white text-red-800 ring-red-200",
      barClassName: "bg-red-700",
      chipClassName: "bg-white text-red-800 ring-red-200",
    },
  };
  const current = meta[displayState];
  const shownProgress = Math.min(100, Math.max(0, progress?.percent ?? 0));
  const progressItems =
    progress && progress.totalItems > 0
      ? t("scanStatus.progressItems", {
          processed: progress.processedItems,
          total: progress.totalItems,
        })
      : t("scanStatus.preparingScan");

  return (
    <div className="mb-4 overflow-hidden rounded-lg border border-slate-200 bg-white text-sm text-ink-body shadow-sm">
      <div className={`h-1 ${current.barClassName}`} />
      <div className="flex flex-wrap items-center gap-3 px-3 py-3">
        <div
          className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-lg ring-1 ${
            current.iconClassName
          } ${displayState === "deepScanRunning" || displayState === "loadingOverview" ? "animate-pulse" : ""}`}
        >
          <span aria-hidden className="text-2xl leading-none">
            {current.emoji}
          </span>
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <div className={`font-semibold ${current.labelClassName}`}>{current.label}</div>
            {displayState === "deepScanRunning" && (
              <span className={`rounded-md px-2 py-0.5 text-[11px] font-semibold ring-1 ${current.chipClassName}`}>du</span>
            )}
            {displayState === "deepScanRunning" && (
              <span className={`rounded-md px-2 py-0.5 text-[11px] font-semibold tabular-nums ring-1 ${current.chipClassName}`}>
                {progressItems}
              </span>
            )}
          </div>
          {current.detail && <div className="mt-1 text-xs opacity-80">{current.detail}</div>}
        </div>
        {displayState === "deepScanRunning" && (
          <div className="ml-auto text-right">
            <div className="text-2xl font-bold leading-none tabular-nums">{shownProgress}%</div>
            <div className="mt-1 text-[11px] font-semibold uppercase opacity-70">progress</div>
          </div>
        )}
        {displayState === "deepScanRunning" && (
          <button
            className="inline-flex min-h-9 items-center gap-1 rounded-lg bg-white px-3 text-xs font-semibold text-blue-800 ring-1 ring-blue-200 hover:bg-blue-100"
            onClick={onCancel}
          >
            <CircleStop size={14} />
            {t("scanStatus.cancelScan")}
          </button>
        )}
      </div>
      {displayState === "deepScanRunning" && (
        <div className="border-t border-current/10 px-3 py-3">
          <div className="h-2.5 overflow-hidden rounded-full bg-white/80 ring-1 ring-current/10">
            <div className={`h-full rounded-full transition-[width] duration-500 ${current.barClassName}`} style={{ width: `${shownProgress}%` }} />
          </div>
          {progress?.currentPath && (
            <div className="mt-2 flex min-w-0 items-center gap-2 rounded-lg bg-white/70 px-2 py-1.5 text-xs ring-1 ring-current/10">
              <span aria-hidden>📍</span>
              <span className="truncate font-mono">{t("scanStatus.currentPath", { path: progress.currentPath })}</span>
            </div>
          )}
        </div>
      )}
      {warnings && hasScanWarnings(warnings) && (
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-current/10 px-3 py-2 text-xs">
          <span>{formatScanWarningSummary(warnings)}</span>
          {!appStoreMode && shouldOfferFullDiskAccess(warnings) && (
            <button
              className="min-h-8 rounded-md bg-white px-3 font-semibold text-amber-900 ring-1 ring-amber-200 hover:bg-amber-50"
              onClick={onOpenPermissions}
            >
              {t("scanStatus.openFullDiskAccess")}
            </button>
          )}
        </div>
      )}
    </div>
  );
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

function titleKeyForView(view: View) {
  const titles: Record<View, string> = {
    dashboard: "view.dashboard.title",
    volumes: "view.volumes.title",
    scanner: "view.scanner.title",
    findings: "view.findings.title",
    recovery: "view.recovery.title",
    settings: "view.settings.title",
  };
  return titles[view];
}

function subtitleKeyForView(view: View) {
  const subtitles: Record<View, string> = {
    dashboard: "view.dashboard.subtitle",
    volumes: "view.volumes.subtitle",
    scanner: "view.scanner.subtitle",
    findings: "view.findings.subtitle",
    recovery: "view.recovery.subtitle",
    settings: "view.settings.subtitle",
  };
  return subtitles[view];
}

function findVolumeByIdentifier(volumes: VolumeInfo[], identifier: string) {
  const normalized = identifier.replace(/^\/dev\//, "");
  return volumes.find((volume) => {
    const volumeIdentifier = volume.identifier.replace(/^\/dev\//, "");
    return volume.identifier === identifier || volumeIdentifier === normalized || volume.mountPoint === identifier;
  });
}

export default App;

function readThemeMode(): ThemeMode {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(THEME_STORAGE_KEY);
  } catch {
    stored = null;
  }
  return stored === "light" ? "light" : "black";
}

function writeThemeMode(themeMode: ThemeMode) {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, themeMode);
  } catch {
    // Storage can be unavailable in some macOS WebView contexts; theme still works for the session.
  }
}

function readStoredBytes(key: string) {
  try {
    const stored = localStorage.getItem(key);
    if (!stored) return 0;
    const value = Number(stored);
    return Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
  } catch {
    return 0;
  }
}

function writeStoredBytes(key: string, bytes: number) {
  try {
    localStorage.setItem(key, String(Math.max(0, Math.floor(bytes))));
  } catch {
    // Storage can be unavailable in some macOS WebView contexts; the in-memory total still updates.
  }
}
