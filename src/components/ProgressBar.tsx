interface ProgressBarProps {
  value?: number | null;
}

export function ProgressBar({ value }: ProgressBarProps) {
  const percent = Math.min(Math.max(value ?? 0, 0), 100);
  const tone = percent >= 90 ? "bg-red-600" : percent >= 75 ? "bg-orange-500" : "bg-emerald-600";

  return (
    <div className="h-3 w-full overflow-hidden rounded-full bg-slate-200" aria-label="Used storage">
      <div className={`h-full rounded-full ${tone}`} style={{ width: `${percent}%` }} />
    </div>
  );
}

