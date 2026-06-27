import { FolderOpen, HardDrive, ShieldCheck, Unplug } from "lucide-react";
import { useMemo } from "react";
import { formatBytes } from "../format";
import { useI18n } from "../i18n";
import type { VolumeInfo } from "../types";
import { PathText } from "./PathText";
import { RiskChip } from "./RiskChip";

interface VolumeTableProps {
  volumes: VolumeInfo[];
  onMountAndScan?: (identifier: string) => void;
  onMountAndScanElevated?: (identifier: string) => void;
  onScanPath?: (path: string) => void;
  onUnmount?: (identifier: string) => void;
  onUnmountElevated?: (identifier: string) => void;
  disabled?: boolean;
  scanDisabled?: boolean;
}

export function VolumeTable({
  volumes,
  onMountAndScan,
  onMountAndScanElevated,
  onScanPath,
  onUnmount,
  onUnmountElevated,
  disabled = false,
  scanDisabled = false,
}: VolumeTableProps) {
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
                  {volume.mountPoint ? <PathText path={volume.mountPoint} className="text-ink-body" /> : <span className="text-amber-700">{t("volumes.notMounted")}</span>}
                </td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.usedBytes)}</td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.availableBytes)}</td>
                <td className="px-4 py-3"><RiskChip risk={volume.risk} /></td>
                <td className="px-4 py-3">
                  <div className="flex flex-wrap justify-end gap-2">
                    {volume.mountPoint ? (
                      <>
                        <button
                          className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-100 px-3 text-xs font-semibold text-ink-body hover:bg-slate-200 disabled:cursor-not-allowed disabled:opacity-40"
                          disabled={!onScanPath || disabled || scanDisabled}
                          onClick={() => volume.mountPoint && onScanPath?.(volume.mountPoint)}
                          title={t("volumes.scanMountPoint")}
                        >
                          <FolderOpen size={14} />
                          {t("common.scan")}
                        </button>
                        <button
                          className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-white px-3 text-xs font-semibold text-ink-body ring-1 ring-slate-200 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-40"
                          disabled={!onUnmount || disabled || isProtectedLiveMount(volume)}
                          onClick={() => onUnmount?.(volume.identifier)}
                          title={isProtectedLiveMount(volume) ? t("volumes.protectedLiveMount") : t("volumes.unmountVolume")}
                        >
                          <Unplug size={14} />
                          {t("volumes.unmountVolume")}
                        </button>
                        <button
                          className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-900 px-3 text-xs font-semibold text-white hover:bg-black disabled:cursor-not-allowed disabled:opacity-40"
                          disabled={!onUnmountElevated || disabled || isProtectedLiveMount(volume)}
                          onClick={() => onUnmountElevated?.(volume.identifier)}
                          title={isProtectedLiveMount(volume) ? t("volumes.protectedLiveMount") : t("volumes.unmountAsAdminTitle")}
                        >
                          <ShieldCheck size={14} />
                          {t("volumes.unmountAsAdmin")}
                        </button>
                      </>
                    ) : (
                      <>
                        <button
                          className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-blue-700 px-3 text-xs font-semibold text-white hover:bg-blue-800 disabled:cursor-not-allowed disabled:bg-slate-400"
                          disabled={!onMountAndScan || disabled || scanDisabled || Boolean(volume.locked) || isProtectedSupportVolume(volume)}
                          onClick={() => onMountAndScan?.(volume.identifier)}
                          title={volume.locked ? t("volumes.volumeLocked") : t("volumes.mountAndScanTitle")}
                        >
                          <HardDrive size={14} />
                          {t("volumes.mountAndScan")}
                        </button>
                        <button
                          className="inline-flex min-h-9 items-center justify-center gap-1 rounded-lg bg-slate-900 px-3 text-xs font-semibold text-white hover:bg-black disabled:cursor-not-allowed disabled:opacity-40"
                          disabled={!onMountAndScanElevated || disabled || scanDisabled || Boolean(volume.locked) || isProtectedSupportVolume(volume)}
                          onClick={() => onMountAndScanElevated?.(volume.identifier)}
                          title={volume.locked ? t("volumes.volumeLocked") : t("volumes.mountAsAdminTitle")}
                        >
                          <ShieldCheck size={14} />
                          {t("volumes.mountAsAdmin")}
                        </button>
                      </>
                    )}
                  </div>
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

function isProtectedSupportVolume(volume: VolumeInfo) {
  return ["System", "Preboot", "VM", "Update", "Recovery", "xART", "Hardware", "Baseband"].includes(
    volume.role ?? "",
  );
}

function isProtectedLiveMount(volume: VolumeInfo) {
  return (
    ["/", "/System/Volumes/Data", "/System/Volumes/Preboot", "/System/Volumes/VM", "/System/Volumes/Update"].includes(
      volume.mountPoint ?? "",
    ) || isProtectedSupportVolume(volume)
  );
}
