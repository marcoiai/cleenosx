import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { categoryLabel, formatBytes } from "../format";
import type { UsageNode } from "../types";
import { RiskChip } from "./RiskChip";

interface UsageTreeProps {
  nodes: UsageNode[];
}

export function UsageTree({ nodes }: UsageTreeProps) {
  return (
    <section className="rounded-lg border border-slate-200 bg-white shadow-material">
      <div className="border-b border-slate-200 px-4 py-3">
        <h2 className="text-sm font-semibold text-ink-strong">Large Blocks</h2>
      </div>
      <div className="divide-y divide-slate-100">
        {nodes.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-ink-muted">No usage data yet.</div>
        ) : (
          nodes.map((node) => <UsageRow key={node.path} node={node} depth={0} />)
        )}
      </div>
    </section>
  );
}

function UsageRow({ node, depth }: { node: UsageNode; depth: number }) {
  const [open, setOpen] = useState(false);
  const hasChildren = node.children.length > 0;

  return (
    <div>
      <div className="grid min-h-14 grid-cols-[1fr_130px_120px_120px] items-center gap-3 px-4 py-3 text-sm">
        <div className="flex min-w-0 items-center gap-2" style={{ paddingLeft: depth * 18 }}>
          <button
            className="flex h-8 w-8 items-center justify-center rounded-lg text-ink-muted hover:bg-slate-100 disabled:opacity-30"
            disabled={!hasChildren}
            onClick={() => setOpen((value) => !value)}
            title={open ? "Collapse" : "Expand"}
          >
            {hasChildren ? open ? <ChevronDown size={16} /> : <ChevronRight size={16} /> : <span className="h-4 w-4" />}
          </button>
          <div className="min-w-0">
            <div className="truncate font-mono text-xs text-ink-strong">{node.path}</div>
            {node.flags.length > 0 && <div className="mt-1 text-xs text-amber-700">flags: {node.flags.join(", ")}</div>}
          </div>
        </div>
        <div className="text-right font-semibold text-ink-strong">{formatBytes(node.sizeBytes)}</div>
        <div className="text-xs text-ink-muted">{categoryLabel(node.category)}</div>
        <RiskChip risk={node.risk} />
      </div>
      {open && node.children.map((child) => <UsageRow key={child.path} node={child} depth={depth + 1} />)}
    </div>
  );
}

