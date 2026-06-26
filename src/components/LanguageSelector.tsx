import { ChevronDown, Languages } from "lucide-react";
import { languageOptions, useI18n } from "../i18n";
import type { Language } from "../i18n";

interface LanguageSelectorProps {
  variant?: "section" | "toolbar";
}

export function LanguageSelector({ variant = "section" }: LanguageSelectorProps) {
  const { language, setLanguage, t } = useI18n();
  const options = languageOptions();
  const currentLanguage = options.find((option) => option.value === language) ?? options[0];

  const sectionSelect = (
    <label className="relative inline-flex shrink-0 items-center">
      <span className="sr-only">{t("settings.language.title")}</span>
      <Languages className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-ink-muted" size={16} />
      <ChevronDown className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-ink-muted" size={16} />
      <select
        className="min-h-10 min-w-[9.75rem] shrink-0 appearance-none rounded-lg border border-slate-300 bg-white py-0 pl-9 pr-9 text-sm font-semibold text-ink-body outline-none transition focus:border-blue-500"
        value={language}
        onChange={(event) => setLanguage(event.target.value as Language)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );

  if (variant === "toolbar") {
    return (
      <label className="relative inline-flex h-10 min-w-[13rem] shrink-0 items-center rounded-xl border border-slate-200 bg-slate-50 px-3 shadow-sm transition hover:border-slate-300 hover:bg-white focus-within:border-blue-500 focus-within:bg-white">
        <span className="sr-only">{t("settings.language.title")}</span>
        <div className="pointer-events-none flex min-w-0 items-center gap-2 pr-7">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-slate-200 bg-white text-blue-700">
            <Languages size={15} />
          </div>
          <div className="min-w-0">
            <div className="text-[11px] font-semibold uppercase tracking-wide text-ink-muted">
              {t("settings.language.title")}
            </div>
            <div className="truncate text-sm font-semibold leading-tight text-ink-strong">{currentLanguage.label}</div>
          </div>
        </div>
        <ChevronDown className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-ink-muted" size={16} />
        <select
          className="absolute inset-0 cursor-pointer appearance-none rounded-xl opacity-0"
          value={language}
          onChange={(event) => setLanguage(event.target.value as Language)}
        >
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    );
  }

  return (
    <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
      <div className="flex items-start justify-between gap-4">
        <div className="flex gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-slate-100 text-ink-body">
            <Languages size={18} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-ink-strong">{t("settings.language.title")}</h3>
            <p className="mt-1 max-w-3xl text-sm text-ink-muted">{t("settings.language.description")}</p>
          </div>
        </div>
        {sectionSelect}
      </div>
    </section>
  );
}
