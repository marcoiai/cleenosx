interface StatPanelProps {
  label: string;
  value: string;
  tone?: "neutral" | "good" | "warn" | "bad";
}

export function StatPanel({ label, value, tone = "neutral" }: StatPanelProps) {
  const toneClass =
    tone === "good"
      ? "border-emerald-200 bg-emerald-50"
      : tone === "warn"
        ? "border-amber-200 bg-amber-50"
        : tone === "bad"
          ? "border-red-200 bg-red-50"
          : "border-slate-200 bg-white";

  return (
    <section className={`rounded-lg border p-4 shadow-material ${toneClass}`}>
      <div className="text-xs font-semibold uppercase tracking-wide text-ink-muted">{label}</div>
      <div className="mt-2 text-2xl font-semibold text-ink-strong">{value}</div>
    </section>
  );
}

