import { useI18n } from "../i18n";
import type { ActionProfile } from "../types";

interface ActionScoreSummaryProps {
  actionProfile: ActionProfile;
  includeNextAction?: boolean;
  className?: string;
  compact?: boolean;
}

export function ActionScoreSummary({
  actionProfile,
  includeNextAction = false,
  className = "",
  compact = false,
}: ActionScoreSummaryProps) {
  const { t } = useI18n();
  const dependencyPercent = Math.max(0, Math.min(100, 100 - actionProfile.scores.automationPercent));
  const scores = [
    { key: "safety", label: t("scores.safety"), value: actionProfile.scores.safetyPercent, tone: "positive" },
    { key: "reclaim", label: t("scores.reclaim"), value: actionProfile.scores.reclaimValuePercent, tone: "positive" },
    { key: "dependency", label: t("scores.dependency"), value: dependencyPercent, tone: "caution" },
    { key: "danger", label: t("scores.danger"), value: actionProfile.ui.severityPercent, tone: "danger" },
  ];
  const nextAction = includeNextAction ? actionProfile.recommendation.nextAction : null;

  return (
    <div className={className}>
      <div className={`grid gap-2 ${compact ? "grid-cols-2" : "grid-cols-2 lg:grid-cols-4"}`}>
        {scores.map((score) => (
          <div key={score.key} className="rounded-lg border border-slate-200 bg-white px-2.5 py-2">
            <div className="flex items-center justify-between gap-2">
              <span className="truncate text-[11px] font-semibold uppercase text-ink-muted">{score.label}</span>
              <span className="font-mono text-xs font-semibold text-ink-strong">{score.value}%</span>
            </div>
            <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-slate-100">
              <div className={`h-full rounded-full ${scoreTone(score.value, score.tone)}`} style={{ width: `${score.value}%` }} />
            </div>
          </div>
        ))}
      </div>
      {nextAction && (
        <div className="mt-2 rounded-lg border border-blue-100 bg-blue-50 px-3 py-2 text-xs font-medium text-blue-800">
          {nextAction}
        </div>
      )}
    </div>
  );
}

function scoreTone(value: number, tone: string) {
  if (tone === "danger") {
    if (value >= 70) return "bg-red-600";
    if (value >= 40) return "bg-amber-500";
    return "bg-emerald-600";
  }

  if (tone === "caution") {
    if (value >= 70) return "bg-amber-500";
    if (value >= 35) return "bg-orange-500";
    return "bg-emerald-600";
  }

  if (value >= 75) return "bg-emerald-600";
  if (value >= 45) return "bg-amber-500";
  if (value > 0) return "bg-orange-500";
  return "bg-slate-300";
}
