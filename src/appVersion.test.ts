import { beforeEach, describe, expect, it, vi } from "vitest";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { version } from "../package.json";
import tauriConfig from "../src-tauri/tauri.conf.json";
import { readAppVersion } from "./appVersion";

vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn() }));

describe("software version", () => {
  beforeEach(() => vi.resetAllMocks());
  it("reads the running native version, not a hardcoded display value", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(getVersion).mockResolvedValue("9.8.7");
    expect(await readAppVersion()).toBe("9.8.7");
  });
  it("uses package metadata in a browser preview", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    expect(await readAppVersion()).toBe(version);
    expect(getVersion).not.toHaveBeenCalled();
    expect(version).toBe(tauriConfig.version);
  });
  it("does not misreport the package version when a native lookup fails", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(getVersion).mockRejectedValue(new Error("Unavailable"));
    await expect(readAppVersion()).rejects.toThrow("Unavailable");
  });
});
