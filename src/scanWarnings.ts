import type { DeepScanWarningsSummary } from "./types";

export function hasScanWarnings(warnings: DeepScanWarningsSummary) {
  return (
    warnings.permissionDenied > 0 ||
    warnings.operationNotPermitted > 0 ||
    warnings.vanishedPaths > 0 ||
    warnings.unexpectedErrors.length > 0 ||
    warnings.samples.length > 0
  );
}

export function shouldOfferFullDiskAccess(warnings: DeepScanWarningsSummary) {
  return warnings.permissionDenied > 0;
}

export function formatScanWarningSummary(warnings: DeepScanWarningsSummary) {
  if (warnings.unexpectedErrors.length === 0 && warnings.samples.length > 0) {
    const vanished = warnings.vanishedPaths > 0 ? ` ${warnings.vanishedPaths} vanished path(s) also skipped.` : "";

    if (warnings.permissionDenied > 0 && warnings.operationNotPermitted === 0) {
      return `Skipped ${warnings.permissionDenied} path(s) blocked by privacy permissions. Full Disk Access may reveal more; partial results are usable.${vanished}`;
    }

    if (warnings.operationNotPermitted > 0 && warnings.permissionDenied === 0) {
      return `Skipped ${warnings.operationNotPermitted} macOS-protected path(s). Full Disk Access will not unlock sealed or system-only locations; partial results are usable.${vanished}`;
    }

    if (warnings.permissionDenied > 0 && warnings.operationNotPermitted > 0) {
      return `Skipped ${warnings.permissionDenied + warnings.operationNotPermitted} protected path(s): ${warnings.permissionDenied} blocked by privacy permissions and ${warnings.operationNotPermitted} blocked by macOS protection. Partial results are usable.${vanished}`;
    }
  }

  const parts = [];
  if (warnings.permissionDenied > 0) parts.push(`${warnings.permissionDenied} permission denied`);
  if (warnings.operationNotPermitted > 0) parts.push(`${warnings.operationNotPermitted} operation not permitted`);
  if (warnings.vanishedPaths > 0) parts.push(`${warnings.vanishedPaths} vanished`);
  if (warnings.unexpectedErrors.length > 0) parts.push(`${warnings.unexpectedErrors.length} unexpected`);
  const sample = warnings.samples[0] ? ` Sample: ${warnings.samples[0]}` : "";
  const summary = parts.length > 0 ? parts.join(" · ") : "Partial scan";
  return `${summary}.${sample}`;
}
