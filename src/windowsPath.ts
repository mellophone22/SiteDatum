/** Convert Windows verbatim paths from native dialogs into normal user-facing paths. */
export function displayWindowsPath(path: string): string {
  if (path.slice(0, 8).toLocaleLowerCase() === "\\\\?\\unc\\") return `\\\\${path.slice(8)}`;
  if (path.startsWith("\\\\?\\")) return path.slice(4);
  return path;
}
