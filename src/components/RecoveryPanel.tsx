import { Clipboard, FileDown, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { exportRecoveryScript, generateRecoveryScript } from "../tauri";
import { LoadingButton } from "./LoadingButton";

export function RecoveryPanel() {
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
              <h2 className="text-sm font-semibold text-ink-strong">One Recovery Executable</h2>
              <p className="mt-1 max-w-3xl text-sm text-ink-muted">
                Creates one guided `cx.sh` file in `/Users/Shared`, plus a Desktop copy when possible.
              </p>
            </div>
          </div>
          <LoadingButton loading={loading} onClick={exportScript}>
            <FileDown size={16} />
            Create Executable
          </LoadingButton>
        </div>

        {error && <div className="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}

        {exportedPath && (
          <div className="mt-4 grid gap-3 rounded-lg bg-slate-50 p-3 text-sm text-ink-body">
            <div>
              <div className="text-xs font-semibold uppercase text-ink-muted">Created</div>
              <div className="mt-1 break-all font-mono text-xs text-ink-strong">{exportedPath}</div>
            </div>
            <div>
              <div className="text-xs font-semibold uppercase text-ink-muted">Recovery Terminal</div>
              <code className="mt-1 block break-all rounded-lg bg-white p-2 font-mono text-xs text-ink-strong">
                zsh /Volumes/*/Users/Shared/cx.sh
              </code>
            </div>
            <div className="text-xs text-ink-muted">
              This avoids typing the disk name or user folder. The script auto-detects mounted Data volumes and has a built-in safe test file flow.
            </div>
          </div>
        )}
      </section>

      <section className="rounded-lg border border-slate-200 bg-white shadow-material">
        <div className="flex items-center justify-between gap-4 border-b border-slate-200 px-4 py-3">
          <h2 className="text-sm font-semibold text-ink-strong">Preview</h2>
          <LoadingButton loading={loading} onClick={loadScript} className="bg-slate-700 hover:bg-slate-800">
            <Clipboard size={16} />
            Show Script
          </LoadingButton>
        </div>
        <pre className="max-h-[420px] overflow-auto whitespace-pre-wrap p-4 font-mono text-xs text-ink-body">
          {script || "Create the executable first, or show the script for review."}
        </pre>
      </section>
    </section>
  );
}
