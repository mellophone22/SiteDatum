type AppError = { code?: string; message?: string; recovery?: string; correlationId?: string };

export function describeAppError(error: unknown): string {
  const candidate = typeof error === "object" && error !== null ? error as AppError : {};
  const message = candidate.message ?? "The operation could not be completed.";
  const recovery = candidate.recovery ? ` ${candidate.recovery}` : "";
  const reference = candidate.correlationId ? ` Reference: ${candidate.correlationId}.` : "";
  return `${message}${recovery}${reference}`;
}
