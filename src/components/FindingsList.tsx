import { categoryLabel, formatBytes } from "../format";
import type { Finding } from "../types";
import { RiskChip } from "./RiskChip";

interface FindingsListProps {
  findings: Finding[];
}

export function FindingsList({ findings }: FindingsListProps) {
  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Findings</h2>
      </div>
      <div className="grid gap-3 p-4">
        {findings.length === 0 ? (
          <div className="py-6 text-center text-sm text-ink-muted">No findings yet.</div>
        ) : (
          findings.map((finding, index) => (
            <article key={`${finding.title}-${finding.path ?? index}`} className="rounded-lg border border-slate-200 bg-white p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div className="font-semibold text-ink-strong">{finding.title}</div>
                  <div className="mt-1 text-xs text-ink-muted">
                    {categoryLabel(finding.category)} · {formatBytes(finding.sizeBytes)}
                  </div>
                </div>
                <RiskChip risk={finding.risk} />
              </div>
              {finding.path && <div className="mt-3 truncate font-mono text-xs text-ink-body">{finding.path}</div>}
              <div className="mt-3 text-sm text-ink-body">{finding.reason}</div>
              <div className="mt-2 text-sm font-medium text-ink-strong">{finding.recommendedAction}</div>
            </article>
          ))
        )}
      </div>
    </section>
  );
}

