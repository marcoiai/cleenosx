import type { ScanLog } from "../types";

interface LogsPanelProps {
  logs: ScanLog[];
}

export function LogsPanel({ logs }: LogsPanelProps) {
  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Logs</h2>
      </div>
      <div className="max-h-56 overflow-auto p-4 font-mono text-xs text-ink-body">
        {logs.length === 0 ? (
          <div className="text-ink-muted">No logs yet.</div>
        ) : (
          logs.slice(-80).map((log, index) => (
            <div key={`${log.timestamp}-${index}`} className="grid grid-cols-[72px_1fr] gap-3 py-1">
              <span className={log.level === "error" ? "text-red-700" : log.level === "warning" ? "text-amber-700" : "text-blue-700"}>
                {log.level}
              </span>
              <span>{log.message}</span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

