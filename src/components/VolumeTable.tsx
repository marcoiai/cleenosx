import { formatBytes } from "../format";
import type { VolumeInfo } from "../types";
import { RiskChip } from "./RiskChip";

interface VolumeTableProps {
  volumes: VolumeInfo[];
}

export function VolumeTable({ volumes }: VolumeTableProps) {
  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Volumes / Partitions</h2>
      </div>
      <div className="overflow-auto">
        <table className="min-w-full border-collapse text-sm">
          <thead className="bg-slate-50 text-left text-xs uppercase text-ink-muted">
            <tr>
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Identifier</th>
              <th className="px-4 py-3">Role</th>
              <th className="px-4 py-3">Mount Point</th>
              <th className="px-4 py-3">Used</th>
              <th className="px-4 py-3">Available</th>
              <th className="px-4 py-3">Risk</th>
            </tr>
          </thead>
          <tbody>
            {volumes.map((volume) => (
              <tr key={`${volume.identifier}-${volume.mountPoint ?? "none"}`} className="border-t border-slate-100 align-top">
                <td className="px-4 py-3 font-medium text-ink-strong">
                  {volume.name}
                  {volume.locked && <div className="mt-1 text-xs text-red-700">Locked / FileVault</div>}
                  {volume.notes.map((note) => (
                    <div key={note} className="mt-1 text-xs text-amber-700">{note}</div>
                  ))}
                </td>
                <td className="px-4 py-3 font-mono text-xs text-ink-body">{volume.identifier}</td>
                <td className="px-4 py-3 text-ink-body">{volume.role ?? "Unknown"}</td>
                <td className="max-w-72 px-4 py-3 font-mono text-xs text-ink-body">
                  {volume.mountPoint ?? <span className="text-amber-700">Not mounted</span>}
                </td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.usedBytes)}</td>
                <td className="px-4 py-3 text-ink-body">{formatBytes(volume.availableBytes)}</td>
                <td className="px-4 py-3"><RiskChip risk={volume.risk} /></td>
              </tr>
            ))}
            {volumes.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-8 text-center text-ink-muted">No volume data yet.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

