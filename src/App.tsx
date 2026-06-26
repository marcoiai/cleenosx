import {
  Activity,
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
import { categoryLabel, formatBytes } from "./format";
import { useI18n } from "./i18n";
import { formatScanWarningSummary, hasScanWarnings, shouldOfferFullDiskAccess } from "./scanWarnings";
import {
  cancelDeepScan,
  getAdminSessionStatus,
  getCleanupSettings,
  getDefaultScanPath,
  getStorageOverview,
  lockAdminSession,
  listSnapshots,
  scanAssetsV2,
  scanContainers,
  scanDeveloperTools,
  scanRustArtifacts,
  scanVolumes,
  openFullDiskAccessSettings,
  startDeepScan,
  unlockAdminSession,
  updateCleanupSettings,
} from "./tauri";
import type { AdminSessionStatus, CleanupSettings, DeepScanWarningsSummary, Finding, Overview, ScanLog, UsageNode, VolumeInfo } from "./types";

type View = "dashboard" | "volumes" | "scanner" | "findings" | "recovery" | "settings";
type ScanState = "idle" | "loadingOverview" | "ready" | "deepScanRunning" | "deepScanPartial" | "deepScanCanceled" | "deepScanFailed";
type ThemeMode = "light" | "black";
const THEME_STORAGE_KEY = "cleanerx.themeMode.v2";
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
  const [deepScanWarnings, setDeepScanWarnings] = useState<DeepScanWarningsSummary | null>(null);
  const [defaultScanPath, setDefaultScanPath] = useState("/Users");
  const [deepScanPath, setDeepScanPath] = useState("/Users");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => readThemeMode());
  const scanInFlight = useRef(false);
  const overviewInFlight = useRef(false);

  const allFindings = useMemo(() => {
    const byKey = new Map<string, Finding>();
    [...overview.findings, ...findings].forEach((finding) => {
      byKey.set(`${finding.title}-${finding.path ?? finding.reason}`, finding);
    });
    return Array.from(byKey.values()).sort(
      (left, right) =>
        (right.sizeBytes ?? -1) - (left.sizeBytes ?? -1) ||
        left.title.localeCompare(right.title),
    );
  }, [findings, overview.findings]);

  const visibleUsage = useMemo(() => {
    const source = usage.length ? usage : overview.usageRoots;
    return [...source].sort(
      (left, right) =>
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
    document.documentElement.classList.toggle("black", themeMode === "black");
    document.documentElement.style.colorScheme = themeMode === "black" ? "dark" : "light";
    writeThemeMode(themeMode);
  }, [themeMode]);

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

  async function runDeepScan(path: string) {
    if (scanState === "deepScanRunning") return;
    setDeepScanPath(path);
    setScanState("deepScanRunning");
    setDeepScanWarnings(null);
    await withLoading(async () => {
      const result = await startDeepScan(path);
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
    });
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
    if (!loading && !scanInFlight.current && !hasDataForView(view)) {
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
            <div className="text-xl font-semibold text-ink-strong">CleanerX</div>
            <div className="mt-1 text-xs font-medium uppercase tracking-wide text-red-700">{t("app.tagline")}</div>
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
                  disabled={loading}
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
                  disabled={adminLoading || loading}
                  onClick={() => void toggleAdminSession()}
                >
                  <ShieldCheck size={16} />
                  {adminLoading ? "Authorizing..." : adminSession.unlocked ? "Admin Ready" : "Unlock Admin"}
                </button>
              )}
              <LoadingButton loading={loading} onClick={scanActionForView(activeView)}>
                <RotateCw size={16} />
                {t("common.scan")}
              </LoadingButton>
            </div>
          </header>

          <div className="p-6">
            {error && <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}
            <ScanStatus
              state={scanState}
              warnings={deepScanWarnings}
              onCancel={cancelRunningDeepScan}
              onOpenPermissions={() => void openFullDiskAccessSettings()}
              appStoreMode={IS_APP_STORE_BUILD}
            />

            {activeView === "dashboard" && (
              <div className="grid gap-6">
                <Dashboard overview={overview} />
                <div className="grid grid-cols-[1.2fr_0.8fr] gap-6">
                  <FindingsList findings={allFindings.slice(0, 5)} onScanPath={scanPath} disabled={loading} />
                  <LogsPanel logs={logs} />
                </div>
              </div>
            )}

            {activeView === "volumes" && (
              <div className="grid gap-6">
                <VolumeTable volumes={volumes} onScanPath={scanPath} disabled={loading} />
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
                onRescanPath={(path) => void runDeepScan(path)}
              />
            )}

            {activeView === "findings" && (
              <div className="grid gap-6">
                <FindingsList findings={allFindings} onScanPath={scanPath} disabled={loading} />
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

function Dashboard({ overview }: { overview: Overview }) {
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
      <div className="grid grid-cols-4 gap-4">
        <StatPanel label={t("dashboard.total")} value={formatBytes(overview.summary.totalBytes)} />
        <StatPanel label={t("dashboard.used")} value={formatBytes(overview.summary.usedBytes)} />
        <StatPanel label={t("dashboard.free")} value={formatBytes(overview.summary.availableBytes)} tone={freeTone} />
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
  onCancel,
  onOpenPermissions,
  appStoreMode,
}: {
  state: ScanState;
  warnings: DeepScanWarningsSummary | null;
  onCancel: () => void;
  onOpenPermissions: () => void;
  appStoreMode: boolean;
}) {
  const { t } = useI18n();
  if (state === "idle") {
    return null;
  }

  const displayState = state === "ready" && warnings && hasScanWarnings(warnings) ? "deepScanPartial" : state;

  const meta: Record<ScanState, { label: string; className: string }> = {
    idle: { label: "", className: "" },
    loadingOverview: { label: t("scanStatus.loadingOverview"), className: "border-blue-200 bg-blue-50 text-blue-800" },
    ready: { label: t("scanStatus.ready"), className: "border-emerald-200 bg-emerald-50 text-emerald-800" },
    deepScanRunning: { label: t("scanStatus.deepScanRunning"), className: "border-blue-200 bg-blue-50 text-blue-800" },
    deepScanPartial: { label: t("scanStatus.deepScanPartial"), className: "border-amber-200 bg-amber-50 text-amber-900" },
    deepScanCanceled: { label: t("scanStatus.deepScanCanceled"), className: "border-slate-200 bg-slate-50 text-ink-body" },
    deepScanFailed: { label: t("scanStatus.deepScanFailed"), className: "border-red-200 bg-red-50 text-red-800" },
  };
  const current = meta[displayState];

  return (
    <div className={`mb-4 rounded-lg border px-3 py-2 text-sm ${current.className}`}>
      <div className="flex items-center justify-between gap-3">
        <div className="font-semibold">{current.label}</div>
        {displayState === "deepScanRunning" && (
          <button
            className="min-h-8 rounded-lg bg-white px-3 text-xs font-semibold text-blue-800 ring-1 ring-blue-200 hover:bg-blue-100"
            onClick={onCancel}
          >
            {t("scanStatus.cancelScan")}
          </button>
        )}
      </div>
      {(displayState === "loadingOverview" || displayState === "deepScanRunning") && (
        <div className="mt-2">
          <div className="h-2 overflow-hidden rounded-full bg-white/70">
            <div className="indeterminate-progress h-full rounded-full bg-current opacity-70" />
          </div>
          <div className="mt-1 text-xs">
            {displayState === "loadingOverview"
              ? t("scanStatus.loadingOverviewDetail")
              : t("scanStatus.deepScanDetail")}
          </div>
        </div>
      )}
      {warnings && hasScanWarnings(warnings) && (
        <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs">
          <span>{formatScanWarningSummary(warnings)}</span>
          {!appStoreMode && shouldOfferFullDiskAccess(warnings) && (
            <button
              className="min-h-8 rounded-lg bg-white px-3 font-semibold text-amber-900 ring-1 ring-amber-200 hover:bg-amber-100"
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
