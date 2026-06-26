import { FolderOpen } from "lucide-react";
import { useMemo } from "react";
import { formatBytes } from "../format";
import { useI18n } from "../i18n";
import type { VolumeInfo } from "../types";
import { RiskChip } from "./RiskChip";

interface VolumeTableProps {
  volumes: VolumeInfo[];
  onScanPath?: (path: string) => void;
  disabled?: boolean;
}

export function VolumeTable({ volumes, onScanPath, disabled = false }: VolumeTableProps) {
  const { t } = useI18n();
  const sortedVolumes = useMemo(
    () =>
      [...volumes].sort(
        (left, right) =>
          volumeSortBytes(right) - volumeSortBytes(left) ||
          left.name.localeCompare(right.name),
      ),
    [volumes],
  );

  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">{t("volumes.title")}</h2>
      </div>
      <div className="overflow-auto">
        <table className="min-w-full border-collapse text-sm">
          <thead className="bg-slate-50 text-left text-xs uppercase text-ink-muted">
            <tr>
              <th className="px-4 py-3">{t("volumes.name")}</th>
              <th className="px-4 py-3">{t("volumes.identifier")}</th>
              <th className="px-4 py-3">{t("volumes.role")}</th>
              <th className="px-4 py-3">{t("volumes.mountPoint")}</th>
              <th className="px-4 py-3">{t("volumes.used")}</th>
              <th className="px-4 py-3">{t("volumes.available")}</th>
              <th className="px-4 py-3">{t("volumes.risk")}</th>
              <th className="px-4 py-3 text-right">{t("volumes.action")}</th>
            </tr>
          </thead>
          <tbody>
            {sortedVolumes.map((volume) => (
              <tr key={`${volume.identifier}-${volume.mountPoint ?? "none"}`} className="border-t border-slate-100 align-top">
                <td className="px-4 py-3 font-medium text-ink-strong">
                  {volume.name}
                  {volume.locked && <div className="mt-1 text-xs text-red-700">{t("volumes.locked")}</div>}
                  {volume.notes.map((note) => (
                    <div key={note} className="mt-1 text-xs text-amber-700">{note}</div>
                  ))}
                </td>
                <td className="px-4 py-3 font-mono text-xs text-ink-body">{volume.identifier}</td>
                <td className="px-4 py-3 text-ink-body">{volume.role ?? t("common.unknown")}</td>
                <td className="max-w-72 px-4 py-3 font-mono text-xs text-ink-body">
                  {volume.mountPoint ?? <span className="text-amber-700">{t("volumes.notMounted")}</span>}
                </td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.usedBytes)}</td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.availableBytes)}</td>
                <td className="px-4 py-3"><RiskChip risk={volume.risk} /></td>
                <td className="px-4 py-3 text-right">
                  <button
                    className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-100 px-3 text-xs font-semibold text-ink-body hover:bg-slate-200 disabled:cursor-not-allowed disabled:opacity-40"
                    disabled={!volume.mountPoint || !onScanPath || disabled}
                    onClick={() => volume.mountPoint && onScanPath?.(volume.mountPoint)}
                    title={volume.mountPoint ? t("volumes.scanMountPoint") : t("volumes.notMounted")}
                  >
                    <FolderOpen size={14} />
                    {t("common.scan")}
                  </button>
                </td>
              </tr>
            ))}
            {sortedVolumes.length === 0 && (
              <tr>
                <td colSpan={8} className="px-4 py-8 text-center text-ink-muted">{t("common.noVolumeData")}</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function volumeSortBytes(volume: VolumeInfo) {
  return volume.usedBytes ?? volume.capacityBytes ?? 0;
}
