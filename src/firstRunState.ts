export function needsFirstRun(projectRoot: string | null, projectCount: number) {
  return !projectRoot && projectCount === 0;
}
