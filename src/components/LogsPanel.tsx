import type { ScanLog } from "../types";
import { useI18n } from "../i18n";

interface LogsPanelProps {
  logs: ScanLog[];
}

export function LogsPanel({ logs }: LogsPanelProps) {
  const { t } = useI18n();
  return (
    <section className="min-w-0 rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">{t("logs.title")}</h2>
      </div>
      <div className="max-h-56 overflow-auto p-4 font-mono text-xs text-ink-body">
        {logs.length === 0 ? (
          <div className="text-ink-muted">{t("logs.empty")}</div>
        ) : (
          logs.slice(-80).map((log, index) => (
            <div key={`${log.timestamp}-${index}`} className="grid grid-cols-[72px_minmax(0,1fr)] gap-3 py-1">
              <span className={log.level === "error" ? "text-red-700" : log.level === "warning" ? "text-amber-700" : "text-blue-700"}>
                {log.level}
              </span>
              <span className="break-words">{log.message}</span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
