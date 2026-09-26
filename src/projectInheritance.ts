export function inheritedProjectId(currentProjectId: string, deliberateProjectId?: string): string {
  return deliberateProjectId === undefined ? currentProjectId : deliberateProjectId;
}
