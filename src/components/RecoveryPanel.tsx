import { Clipboard, FileDown, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { useI18n } from "../i18n";
import { exportRecoveryScript, generateRecoveryScript } from "../tauri";
import { LoadingButton } from "./LoadingButton";

export function RecoveryPanel() {
  const { t } = useI18n();
  const [script, setScript] = useState("");
  const [loading, setLoading] = useState(false);
  const [exportedPath, setExportedPath] = useState("");
  const [error, setError] = useState("");

  async function exportScript() {
    setLoading(true);
    setError("");
    try {
      const path = await exportRecoveryScript();
      setExportedPath(path);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  async function loadScript() {
    setLoading(true);
    setError("");
    try {
      setScript(await generateRecoveryScript());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="grid gap-4">
      <section className="rounded-lg border border-slate-200 bg-white p-4 shadow-material">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-blue-50 text-blue-700">
              <TerminalSquare size={20} />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-ink-strong">{t("recovery.oneExecutable")}</h2>
              <p className="mt-1 max-w-3xl text-sm text-ink-muted">{t("recovery.description")}</p>
            </div>
          </div>
          <LoadingButton loading={loading} onClick={exportScript}>
            <FileDown size={16} />
            {t("recovery.createShortcut")}
          </LoadingButton>
        </div>

        {error && <div className="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}

        {exportedPath && (
          <div className="mt-4 grid gap-3 rounded-lg bg-slate-50 p-3 text-sm text-ink-body">
            <div>
              <div className="text-xs font-semibold uppercase text-ink-muted">{t("recovery.created")}</div>
              <div className="mt-1 break-all font-mono text-xs text-ink-strong">{exportedPath}</div>
            </div>
            <div className="rounded-lg border border-blue-200 bg-blue-50 p-3">
              <div className="text-xs font-semibold uppercase text-blue-800">{t("recovery.stepsTitle")}</div>
              <div className="mt-2 grid gap-1 text-sm text-blue-950">
                <div>{t("recovery.steps1")}</div>
                <div>{t("recovery.steps2")}</div>
                <div>{t("recovery.steps3")}</div>
              </div>
            </div>
            <div>
              <div className="text-xs font-semibold uppercase text-ink-muted">{t("recovery.terminal")}</div>
              <code className="mt-1 block break-all rounded-lg bg-white p-2 font-mono text-xs text-ink-strong">
                zsh /Volumes/*/Users/Shared/cx.sh
              </code>
            </div>
            <div className="text-xs text-ink-muted">{t("recovery.pasteHelp")}</div>
          </div>
        )}
      </section>

      <section className="rounded-lg border border-slate-200 bg-white shadow-material">
        <div className="flex items-center justify-between gap-4 border-b border-slate-200 px-4 py-3">
          <h2 className="text-sm font-semibold text-ink-strong">{t("recovery.preview")}</h2>
          <LoadingButton loading={loading} onClick={loadScript} className="bg-slate-700 hover:bg-slate-800">
            <Clipboard size={16} />
            {t("recovery.showScript")}
          </LoadingButton>
        </div>
        <pre className="max-h-[420px] overflow-auto whitespace-pre-wrap p-4 font-mono text-xs text-ink-body">
          {script || t("recovery.emptyPreview")}
        </pre>
      </section>
    </section>
  );
}
