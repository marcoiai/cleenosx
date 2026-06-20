import { riskMeta } from "../format";
import type { RiskLevel } from "../types";

interface RiskChipProps {
  risk: RiskLevel;
}

export function RiskChip({ risk }: RiskChipProps) {
  const meta = riskMeta(risk);
  return (
    <span className={`inline-flex items-center rounded-full px-2.5 py-1 text-xs font-semibold ring-1 ${meta.className}`}>
      {meta.label}
    </span>
  );
}

