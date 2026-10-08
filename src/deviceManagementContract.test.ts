import { describe, expect, it } from "vitest";
import { parseDeviceManagementRequest } from "../supabase/functions/_shared/device_contract";

describe("device-management request contract", () => {
  it("accepts a list request containing only a pseudonymous fingerprint", () => {
    const hash = "a".repeat(64);
    expect(parseDeviceManagementRequest({ action: "list", deviceFingerprintHash: hash }))
      .toEqual({ action: "list", deviceFingerprintHash: hash });
    expect(() => parseDeviceManagementRequest({ action: "list", deviceFingerprintHash: hash, machineName: "DESKTOP" }))
      .toThrow("INVALID_DEVICE_FINGERPRINT");
  });

  it("accepts an ownership-checked deactivation identifier", () => {
    const deviceId = "22222222-2222-4222-8222-222222222222";
    expect(parseDeviceManagementRequest({ action: "deactivate", deviceId }))
      .toEqual({ action: "deactivate", deviceId });
    expect(() => parseDeviceManagementRequest({ action: "deactivate", deviceId: "not-a-device" }))
      .toThrow("INVALID_DEVICE_ID");
  });

  it("rejects unknown actions and free-form payloads", () => {
    expect(() => parseDeviceManagementRequest({ action: "rename", deviceId: crypto.randomUUID() }))
      .toThrow("INVALID_ACTION");
    expect(() => parseDeviceManagementRequest("list")).toThrow("INVALID_REQUEST");
  });
});
