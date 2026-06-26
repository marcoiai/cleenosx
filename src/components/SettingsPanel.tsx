import { Moon, Settings, ShieldCheck, ShieldOff } from "lucide-react";
import { useI18n } from "../i18n";
import type { AdminSessionStatus, CleanupSettings } from "../types";
import { LanguageSelector } from "./LanguageSelector";
import { LoadingButton } from "./LoadingButton";

interface SettingsPanelProps {
  cleanupSettings: CleanupSettings;
  adminSession: AdminSessionStatus;
  adminLoading?: boolean;
  themeMode: "light" | "black";
  onCleanupSettingsChange: (settings: CleanupSettings) => void;
  onThemeModeChange: (themeMode: "light" | "black") => void;
  onToggleAdminSession: () => void;
}

export function SettingsPanel({
  cleanupSettings,
  adminSession,
  adminLoading = false,
  themeMode,
  onCleanupSettingsChange,
  onThemeModeChange,
  onToggleAdminSession,
}: SettingsPanelProps) {
  const { t } = useI18n();

  return (
    <section className="grid gap-4">
      <section className="rounded-lg border border-slate-200 bg-white p-6 shadow-material">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
            <Settings size={22} />
          </div>
          <div>
            <h2 className="text-base font-semibold text-ink-strong">{t("settings.title")}</h2>
            <p className="text-sm text-ink-muted">{t("settings.subtitle")}</p>
          </div>
        </div>
      </section>

      <LanguageSelector />

      {adminSession.available && (
        <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
          <div className="flex items-start justify-between gap-4">
            <div className="flex gap-3">
              <div
                className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${
                  adminSession.unlocked ? "bg-emerald-50 text-emerald-700" : "bg-amber-50 text-amber-700"
                }`}
              >
                {adminSession.unlocked ? <ShieldCheck size={18} /> : <ShieldOff size={18} />}
              </div>
              <div>
                <h3 className="text-sm font-semibold text-ink-strong">{t("settings.adminMode.title")}</h3>
                <p className="mt-1 max-w-3xl text-sm text-ink-muted">
                  {adminSession.available
                    ? adminSession.unlocked
                      ? t("settings.adminMode.on")
                      : t("settings.adminMode.off")
                    : t("settings.adminMode.unavailable")}
                </p>
              </div>
            </div>
            <LoadingButton
              loading={adminLoading}
              className={adminSession.unlocked ? "bg-slate-700 hover:bg-slate-800" : ""}
              onClick={onToggleAdminSession}
            >
              {adminLoading
                ? t("settings.adminMode.authorizing")
                : adminSession.unlocked
                  ? t("settings.adminMode.disable")
                  : t("settings.adminMode.unlock")}
            </LoadingButton>
          </div>
        </section>
      )}

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex items-start justify-between gap-4">
          <div className="flex gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
              <Moon size={18} />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-ink-strong">{t("settings.theme.title")}</h3>
              <p className="mt-1 max-w-3xl text-sm text-ink-muted">{t("settings.theme.description")}</p>
            </div>
          </div>
          <label className="flex shrink-0 items-center gap-2 text-sm font-semibold text-ink-body">
            <input
              type="checkbox"
              className="h-4 w-4"
              checked={themeMode === "black"}
              onChange={(event) => onThemeModeChange(event.target.checked ? "black" : "light")}
            />
            {t("common.enabled")}
          </label>
        </div>
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="text-sm font-semibold text-ink-strong">{t("settings.projectCleanup.title")}</h3>
            <p className="mt-1 max-w-3xl text-sm text-ink-muted">{t("settings.projectCleanup.description")}</p>
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
            {t("settings.projectCleanup.allow")}
          </label>
        </div>
        {cleanupSettings.allowProjectRootCleanup && (
          <div className="mt-3 rounded-lg border border-red-200 bg-red-50 p-3 text-sm font-semibold text-red-800">
            {t("settings.projectCleanup.warning")}
          </div>
        )}
      </section>
    </section>
  );
}
