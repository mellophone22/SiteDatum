export type AppearanceMode = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export const appearanceStorageKey = "appearance.mode";

export function isAppearanceMode(value: unknown): value is AppearanceMode {
  return value === "light" || value === "dark" || value === "system";
}

export function readAppearanceMode(storage: Pick<Storage, "getItem">): AppearanceMode {
  const stored = storage.getItem(appearanceStorageKey);
  return isAppearanceMode(stored) ? stored : "system";
}

export function resolveTheme(mode: AppearanceMode, systemDark: boolean): ResolvedTheme {
  return mode === "system" ? (systemDark ? "dark" : "light") : mode;
}
