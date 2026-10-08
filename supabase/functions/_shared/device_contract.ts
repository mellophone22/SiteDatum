export type DeviceManagementRequest =
  | { action: "list"; deviceFingerprintHash: string }
  | { action: "deactivate"; deviceId: string };

const fingerprintPattern = /^[0-9a-f]{64}$/;
const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export function parseDeviceManagementRequest(value: unknown): DeviceManagementRequest {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("INVALID_REQUEST");
  }
  const input = value as Record<string, unknown>;
  if (input.action === "list") {
    if (Object.keys(input).sort().join(",") !== "action,deviceFingerprintHash"
      || typeof input.deviceFingerprintHash !== "string"
      || !fingerprintPattern.test(input.deviceFingerprintHash)) {
      throw new Error("INVALID_DEVICE_FINGERPRINT");
    }
    return { action: "list", deviceFingerprintHash: input.deviceFingerprintHash };
  }
  if (input.action === "deactivate") {
    if (Object.keys(input).sort().join(",") !== "action,deviceId"
      || typeof input.deviceId !== "string"
      || !uuidPattern.test(input.deviceId)) {
      throw new Error("INVALID_DEVICE_ID");
    }
    return { action: "deactivate", deviceId: input.deviceId };
  }
  throw new Error("INVALID_ACTION");
}
