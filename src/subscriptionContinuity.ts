const DAY_SECONDS = 24 * 60 * 60;

export type ContinuityStage = "none" | "quiet" | "urgent";

export function continuityState(
  verifiedAtUtc: number | null,
  graceEndsAtUtc: number | null,
  nowUtc = Math.floor(Date.now() / 1000),
) {
  if (!verifiedAtUtc || !graceEndsAtUtc || nowUtc <= verifiedAtUtc) {
    return { stage: "none" as ContinuityStage, daysRemaining: null };
  }
  const elapsed = nowUtc - verifiedAtUtc;
  const daysRemaining = Math.max(0, Math.ceil((graceEndsAtUtc - nowUtc) / DAY_SECONDS));
  const stage: ContinuityStage = elapsed >= 17 * DAY_SECONDS
    ? "urgent"
    : elapsed >= 13 * DAY_SECONDS
      ? "quiet"
      : "none";
  return { stage, daysRemaining };
}
