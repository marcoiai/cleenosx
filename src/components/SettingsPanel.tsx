import { Moon, Settings } from "lucide-react";
import type { CleanupSettings } from "../types";

interface SettingsPanelProps {
  cleanupSettings: CleanupSettings;
  themeMode: "light" | "black";
  onCleanupSettingsChange: (settings: CleanupSettings) => void;
  onThemeModeChange: (themeMode: "light" | "black") => void;
}

export function SettingsPanel({ cleanupSettings, themeMode, onCleanupSettingsChange, onThemeModeChange }: SettingsPanelProps) {
  return (
    <section className="grid gap-4">
      <section className="rounded-lg border border-slate-200 bg-white p-6 shadow-material">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
            <Settings size={22} />
          </div>
          <div>
            <h2 className="text-base font-semibold text-ink-strong">Settings</h2>
            <p className="text-sm text-ink-muted">Cleanup behavior and safety rules.</p>
          </div>
        </div>
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex items-start justify-between gap-4">
          <div className="flex gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
              <Moon size={18} />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-ink-strong">Black Mode</h3>
              <p className="mt-1 max-w-3xl text-sm text-ink-muted">
                Use the dark interface by default for long scanning and cleanup sessions.
              </p>
            </div>
          </div>
          <label className="flex shrink-0 items-center gap-2 text-sm font-semibold text-ink-body">
            <input
              type="checkbox"
              className="h-4 w-4"
              checked={themeMode === "black"}
              onChange={(event) => onThemeModeChange(event.target.checked ? "black" : "light")}
            />
            Enabled
          </label>
        </div>
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="text-sm font-semibold text-ink-strong">Project Folder Cleanup</h3>
            <p className="mt-1 max-w-3xl text-sm text-ink-muted">
              Project roots under paths like `/Users/.../Projects` are blocked by default so source folders are not removed by accident. Build artifacts such as `target/` can still be selected.
            </p>
          </div>
          <label className="flex shrink-0 items-center gap-2 text-sm font-semibold text-ink-body">
            <input
              type="checkbox"
              className="h-4 w-4"
              checked={cleanupSettings.allowProjectRootCleanup}
              onChange={(event) =>
                onCleanupSettingsChange({
                  ...cleanupSettings,
                  allowProjectRootCleanup: event.target.checked,
                })
              }
            />
            Allow project roots
          </label>
        </div>
        {cleanupSettings.allowProjectRootCleanup && (
          <div className="mt-3 rounded-lg border border-red-200 bg-red-50 p-3 text-sm font-semibold text-red-800">
            Whole project folders can now be prepared for deletion. Use the final confirmation flow carefully.
          </div>
        )}
      </section>
    </section>
  );
}
