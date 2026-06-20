import { ShieldCheck } from "lucide-react";

export function ReviewPanel() {
  return (
    <section className="rounded-lg border border-slate-200 bg-white p-6 shadow-material">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-50 text-emerald-700">
          <ShieldCheck size={22} />
        </div>
        <div>
          <h2 className="text-base font-semibold text-ink-strong">Review</h2>
          <p className="text-sm text-ink-muted">Cleanup actions are disabled in this MVP.</p>
        </div>
      </div>
      <div className="mt-6 rounded-lg border border-dashed border-slate-300 bg-slate-50 p-5 text-sm text-ink-body">
        Selected cleanup items will appear here after destructive actions are implemented with strong confirmation.
      </div>
    </section>
  );
}

