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
import { useEffect, useMemo, useState } from "react";
import { FindingsList } from "./components/FindingsList";
import { LoadingButton } from "./components/LoadingButton";
import { LogsPanel } from "./components/LogsPanel";
import { ProgressBar } from "./components/ProgressBar";
import { RecoveryPanel } from "./components/RecoveryPanel";
import { ReviewPanel } from "./components/ReviewPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { StatPanel } from "./components/StatPanel";
import { UsageTree } from "./components/UsageTree";
import { VolumeTable } from "./components/VolumeTable";
import { categoryLabel, formatBytes } from "./format";
import {
  listSnapshots,
  scanAssetsV2,
  scanContainers,
  scanDataUsage,
  scanDeveloperTools,
  scanOverview,
  scanRustArtifacts,
  scanVolumes,
} from "./tauri";
import type { Finding, Overview, ScanLog, StorageCategory, UsageNode, VolumeInfo } from "./types";

type View = "dashboard" | "volumes" | "scanner" | "findings" | "review" | "recovery" | "settings";

const emptyOverview: Overview = {
  summary: {},
  volumes: [],
  usageRoots: [],
  findings: [],
};

const navItems: Array<{ id: View; label: string; icon: typeof Activity }> = [
  { id: "dashboard", label: "Dashboard", icon: Activity },
  { id: "volumes", label: "Volumes", icon: HardDrive },
  { id: "scanner", label: "Scanner", icon: FolderTree },
  { id: "findings", label: "Findings", icon: Database },
  { id: "review", label: "Review", icon: ShieldCheck },
  { id: "recovery", label: "Recovery", icon: TerminalSquare },
  { id: "settings", label: "Settings", icon: Settings },
];

function App() {
  const [activeView, setActiveView] = useState<View>("dashboard");
  const [overview, setOverview] = useState<Overview>(emptyOverview);
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [usage, setUsage] = useState<UsageNode[]>([]);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [logs, setLogs] = useState<ScanLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<StorageCategory | "all">("all");

  useEffect(() => {
    void runOverview();
  }, []);

  const allFindings = useMemo(() => {
    const byKey = new Map<string, Finding>();
    [...overview.findings, ...findings].forEach((finding) => {
      byKey.set(`${finding.title}-${finding.path ?? finding.reason}`, finding);
    });
    return Array.from(byKey.values());
  }, [findings, overview.findings]);

  const visibleUsage = useMemo(() => {
    if (categoryFilter === "all") return usage.length ? usage : overview.usageRoots;
    return (usage.length ? usage : overview.usageRoots).filter((node) => node.category === categoryFilter);
  }, [categoryFilter, overview.usageRoots, usage]);

  const categoryOptions = useMemo(() => {
    const categories = new Set<StorageCategory>();
    (usage.length ? usage : overview.usageRoots).forEach((node) => categories.add(node.category));
    return Array.from(categories);
  }, [overview.usageRoots, usage]);

  async function runOverview() {
    await withLoading(async () => {
      const result = await scanOverview();
      setOverview(result.data);
      setVolumes(result.data.volumes);
      setUsage(result.data.usageRoots);
      setFindings(result.data.findings);
      pushLogs(result.logs);
    });
  }

  async function runVolumes() {
    await withLoading(async () => {
      const result = await scanVolumes();
      setVolumes(result.data);
      pushLogs(result.logs);
    });
  }

  async function runDataUsage() {
    await withLoading(async () => {
      const result = await scanDataUsage();
      setUsage(result.data);
      pushLogs(result.logs);
    });
  }

  async function runFindingsScan() {
    await withLoading(async () => {
      const results = await Promise.all([
        scanAssetsV2(),
        scanDeveloperTools(),
        scanRustArtifacts(),
        scanContainers(),
        listSnapshots(),
      ]);
      setFindings(results.flatMap((result) => result.data));
      pushLogs(results.flatMap((result) => result.logs));
    });
  }

  async function withLoading(work: () => Promise<void>) {
    setLoading(true);
    setError("");
    try {
      await work();
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setError(message);
      pushLogs([{ timestamp: Math.floor(Date.now() / 1000), level: "error", message }]);
    } finally {
      setLoading(false);
    }
  }

  function pushLogs(nextLogs: ScanLog[]) {
    setLogs((current) => [...current, ...nextLogs].slice(-160));
  }

  return (
    <div className="min-h-screen bg-surface-base text-ink-body">
      <div className="grid min-h-screen grid-cols-[248px_1fr]">
        <aside className="border-r border-slate-200 bg-white">
          <div className="border-b border-slate-200 px-5 py-5">
            <div className="text-xl font-semibold text-ink-strong">CleanerX</div>
            <div className="mt-1 text-xs font-medium uppercase tracking-wide text-emerald-700">Safe-mode MVP</div>
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
                  }`}
                  onClick={() => setActiveView(item.id)}
                >
                  <Icon size={18} />
                  {item.label}
                </button>
              );
            })}
          </nav>
        </aside>

        <main className="min-w-0">
          <header className="flex items-center justify-between gap-4 border-b border-slate-200 bg-white px-6 py-4">
            <div>
              <h1 className="text-xl font-semibold text-ink-strong">{titleForView(activeView)}</h1>
              <div className="mt-1 text-sm text-ink-muted">{subtitleForView(activeView)}</div>
            </div>
            <LoadingButton loading={loading} onClick={activeView === "volumes" ? runVolumes : activeView === "scanner" ? runDataUsage : activeView === "findings" ? runFindingsScan : runOverview}>
              <RotateCw size={16} />
              Scan
            </LoadingButton>
          </header>

          <div className="p-6">
            {error && <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}

            {activeView === "dashboard" && (
              <div className="grid gap-6">
                <Dashboard overview={overview} />
                <div className="grid grid-cols-[1.2fr_0.8fr] gap-6">
                  <FindingsList findings={allFindings.slice(0, 5)} />
                  <LogsPanel logs={logs} />
                </div>
              </div>
            )}

            {activeView === "volumes" && (
              <div className="grid gap-6">
                <VolumeTable volumes={volumes} />
                <LogsPanel logs={logs} />
              </div>
            )}

            {activeView === "scanner" && (
              <div className="grid gap-4">
                <div className="flex flex-wrap items-center gap-2">
                  <FilterButton active={categoryFilter === "all"} onClick={() => setCategoryFilter("all")}>All</FilterButton>
                  {categoryOptions.map((category) => (
                    <FilterButton key={category} active={categoryFilter === category} onClick={() => setCategoryFilter(category)}>
                      {categoryLabel(category)}
                    </FilterButton>
                  ))}
                </div>
                <UsageTree nodes={visibleUsage} />
                <LogsPanel logs={logs} />
              </div>
            )}

            {activeView === "findings" && (
              <div className="grid gap-6">
                <FindingsList findings={allFindings} />
                <LogsPanel logs={logs} />
              </div>
            )}

            {activeView === "review" && <ReviewPanel />}
            {activeView === "recovery" && <RecoveryPanel />}
            {activeView === "settings" && <SettingsPanel />}
          </div>
        </main>
      </div>
    </div>
  );
}

function Dashboard({ overview }: { overview: Overview }) {
  const freeBytes = overview.summary.availableBytes;
  const freeTone = freeBytes != null && freeBytes < 10 * 1024 ** 3 ? "bad" : freeBytes != null && freeBytes < 15 * 1024 ** 3 ? "warn" : "good";

  const roleCount = new Map<string, number>();
  overview.volumes.forEach((volume) => {
    const role = volume.role ?? "Unknown";
    roleCount.set(role, (roleCount.get(role) ?? 0) + 1);
  });

  return (
    <section className="grid gap-6">
      <div className="grid grid-cols-4 gap-4">
        <StatPanel label="Total" value={formatBytes(overview.summary.totalBytes)} />
        <StatPanel label="Used" value={formatBytes(overview.summary.usedBytes)} />
        <StatPanel label="Free" value={formatBytes(overview.summary.availableBytes)} tone={freeTone} />
        <StatPanel label="APFS Volumes" value={String(overview.volumes.length)} />
      </div>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold text-ink-strong">Primary Storage</h2>
          <span className="text-sm font-semibold text-ink-muted">{overview.summary.percentUsed?.toFixed(1) ?? "0.0"}%</span>
        </div>
        <ProgressBar value={overview.summary.percentUsed} />
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <h2 className="text-sm font-semibold text-ink-strong">Detected Roles</h2>
        <div className="mt-3 flex flex-wrap gap-2">
          {Array.from(roleCount.entries()).map(([role, count]) => (
            <span key={role} className="rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-ink-body">
              {role}: {count}
            </span>
          ))}
          {roleCount.size === 0 && <span className="text-sm text-ink-muted">No volume roles parsed yet.</span>}
        </div>
      </section>
    </section>
  );
}

function FilterButton({ active, children, onClick }: { active: boolean; children: React.ReactNode; onClick: () => void }) {
  return (
    <button
      className={`min-h-9 rounded-full px-3 text-sm font-semibold transition ${
        active ? "bg-blue-700 text-white" : "bg-white text-ink-body ring-1 ring-slate-200 hover:bg-slate-50"
      }`}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function titleForView(view: View) {
  const titles: Record<View, string> = {
    dashboard: "Dashboard",
    volumes: "Volumes / Partitions",
    scanner: "Large Block Scanner",
    findings: "Findings",
    review: "Review",
    recovery: "Recovery",
    settings: "Settings",
  };
  return titles[view];
}

function subtitleForView(view: View) {
  const subtitles: Record<View, string> = {
    dashboard: "Storage summary, APFS roles, warnings, and scan status.",
    volumes: "Mounted and unmounted volume hints from macOS tools.",
    scanner: "Read-only scan of large storage blocks.",
    findings: "Assets, snapshots, developer tools, containers, and risk labels.",
    review: "Safe-mode cleanup review surface.",
    recovery: "Generated companion script for Recovery workflows.",
    settings: "Reserved configuration surface.",
  };
  return subtitles[view];
}

export default App;

