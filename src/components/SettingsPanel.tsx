import { Settings } from "lucide-react";

export function SettingsPanel() {
  return (
    <section className="rounded-lg border border-slate-200 bg-white p-6 shadow-material">
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
          <Settings size={22} />
        </div>
        <div>
          <h2 className="text-base font-semibold text-ink-strong">Settings</h2>
          <p className="text-sm text-ink-muted">Settings will be added later.</p>
        </div>
      </div>
    </section>
  );
}

