import { FolderOpen } from "lucide-react";
import { useMemo } from "react";
import { categoryLabel, formatBytes } from "../format";
import type { ActionProfile, Finding } from "../types";
import { RiskChip } from "./RiskChip";

interface FindingsListProps {
  findings: Finding[];
  onScanPath?: (path: string) => void;
  disabled?: boolean;
}

export function FindingsList({ findings, onScanPath, disabled = false }: FindingsListProps) {
  const sortedFindings = useMemo(
    () =>
      [...findings].sort(
        (left, right) =>
          (right.sizeBytes ?? -1) - (left.sizeBytes ?? -1) ||
          left.title.localeCompare(right.title),
      ),
    [findings],
  );

  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Findings</h2>
      </div>
      <div className="grid gap-3 p-4">
        {sortedFindings.length === 0 ? (
          <div className="py-6 text-center text-sm text-ink-muted">No findings yet.</div>
        ) : (
          sortedFindings.map((finding, index) => (
            <FindingCard
              key={`${finding.title}-${finding.path ?? index}`}
              finding={finding}
              onScanPath={onScanPath}
              disabled={disabled}
            />
          ))
        )}
      </div>
    </section>
  );
}

function FindingCard({
  finding,
  onScanPath,
  disabled,
}: {
  finding: Finding;
  onScanPath?: (path: string) => void;
  disabled: boolean;
}) {
  const actionProfile = finding.actionProfile;

  return (
    <article className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="font-semibold text-ink-strong">{finding.title}</div>
          <div className="mt-1 text-xs text-ink-muted">
            {categoryLabel(finding.category)} · {formatBytes(finding.sizeBytes)}
          </div>
          {actionProfile && (
            <div className="mt-2 flex flex-wrap items-center gap-2 text-xs font-semibold">
              <span className="rounded-full bg-slate-100 px-2.5 py-1 text-ink-body">{actionProfile.ui.badge}</span>
              <span className="rounded-full bg-blue-50 px-2.5 py-1 text-blue-800">{actionProfile.deleteCapability.userFacingLevel}</span>
            </div>
          )}
        </div>
        <RiskChip risk={finding.risk} />
      </div>
      {finding.path && <div className="mt-3 truncate font-mono text-xs text-ink-body">{finding.path}</div>}
      <div className="mt-3 text-sm text-ink-body">{finding.reason}</div>
      {actionProfile && (
        <div className="mt-3 rounded-lg bg-slate-50 px-3 py-2 text-sm text-ink-body">
          {actionProfile.ui.explainLikeUser}
        </div>
      )}
      <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="text-sm font-medium text-ink-strong">{finding.recommendedAction}</div>
          {actionProfile && <div className="mt-1 text-xs text-ink-muted">{formatActionScores(actionProfile)}</div>}
          {actionProfile?.recommendation.nextAction && (
            <div className="mt-1 text-xs text-ink-muted">{actionProfile.recommendation.nextAction}</div>
          )}
        </div>
        {finding.path && (
          <button
            className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-100 px-3 text-xs font-semibold text-ink-body hover:bg-slate-200 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!onScanPath || disabled}
            onClick={() => onScanPath?.(finding.path!)}
            title="Scan this path"
          >
            <FolderOpen size={14} />
            Scan
          </button>
        )}
      </div>
    </article>
  );
}

function formatActionScores(actionProfile: ActionProfile) {
  return `Safety ${actionProfile.scores.safetyPercent}% · Reclaim ${actionProfile.scores.reclaimValuePercent}% · Automation ${actionProfile.scores.automationPercent}% · Confidence ${actionProfile.scores.confidencePercent}%`;
}
