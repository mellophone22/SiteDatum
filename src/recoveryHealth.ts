export type RecoveryHealth = {
  missingFiles: number;
  syncConflicts: number;
};

export function recoveryIssueCount(health: RecoveryHealth) {
  return health.missingFiles + health.syncConflicts;
}

export function recoverySummary(health: RecoveryHealth) {
  const issues = recoveryIssueCount(health);
  if (!issues) return "No recovery issues detected.";
  const parts = [];
  if (health.missingFiles) parts.push(`${health.missingFiles} missing file${health.missingFiles === 1 ? "" : "s"}`);
  if (health.syncConflicts) parts.push(`${health.syncConflicts} sync conflict${health.syncConflicts === 1 ? "" : "s"}`);
  return `${parts.join(" and ")} need attention.`;
}
