import { describe, expect, it } from "vitest";
import { appearanceStorageKey, readAppearanceMode, resolveTheme } from "./themeState";

const storage = (value: string | null) => ({ getItem: (key: string) => key === appearanceStorageKey ? value : null });

describe("appearance state", () => {
  it("defaults invalid and missing preferences to Windows", () => {
    expect(readAppearanceMode(storage(null))).toBe("system");
    expect(readAppearanceMode(storage("midnight"))).toBe("system");
  });

  it("restores every supported explicit preference", () => {
    expect(readAppearanceMode(storage("light"))).toBe("light");
    expect(readAppearanceMode(storage("dark"))).toBe("dark");
    expect(readAppearanceMode(storage("system"))).toBe("system");
  });

  it("follows Windows only while system mode is selected", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});
