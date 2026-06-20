import { FileDown } from "lucide-react";
import { useState } from "react";
import { generateRecoveryScript } from "../tauri";
import { LoadingButton } from "./LoadingButton";

export function RecoveryPanel() {
  const [script, setScript] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

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
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="flex items-center justify-between gap-4 border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Recovery Script</h2>
        <LoadingButton loading={loading} onClick={loadScript}>
          <FileDown size={16} />
          Generate
        </LoadingButton>
      </div>
      {error && <div className="m-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">{error}</div>}
      <pre className="max-h-[540px] overflow-auto whitespace-pre-wrap p-4 font-mono text-xs text-ink-body">
        {script || "No script generated yet."}
      </pre>
    </section>
  );
}

