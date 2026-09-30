import { describe, expect, it } from "vitest";
import { displayWindowsPath } from "./windowsPath";

describe("Windows path display", () => {
  it("removes the verbatim prefix from drive paths returned by native dialogs", () => {
    expect(displayWindowsPath("\\\\?\\C:\\Users\\fcabrera\\Projects")).toBe("C:\\Users\\fcabrera\\Projects");
  });

  it("restores normal UNC syntax from extended UNC paths", () => {
    expect(displayWindowsPath("\\\\?\\UNC\\server\\share\\Projects")).toBe("\\\\server\\share\\Projects");
    expect(displayWindowsPath("\\\\?\\unc\\server\\share")).toBe("\\\\server\\share");
  });

  it("leaves ordinary paths unchanged", () => {
    expect(displayWindowsPath("C:\\Projects")).toBe("C:\\Projects");
    expect(displayWindowsPath("\\\\server\\share")).toBe("\\\\server\\share");
  });
});
