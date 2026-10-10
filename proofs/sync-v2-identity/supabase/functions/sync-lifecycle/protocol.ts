const LABEL = new TextEncoder().encode("sitedatum.sync-v2.lifecycle.v1");
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/;

export type LifecycleRequest =
  | { action: "disableWorkspace" | "restoreWorkspace"; deviceId: string; workspaceId: string; signature: string }
  | { action: "deleteWorkspace"; deviceId: string; workspaceId: string; requestId: string; signature: string }
  | { action: "deleteAccountSyncData"; deviceId: string; requestId: string; signature: string };

function uuid(value: unknown): string {
  if (typeof value !== "string" || !UUID_PATTERN.test(value)) throw new Error("INVALID_UUID");
  return value.toLowerCase();
}

function exactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const keys = Object.keys(value).sort();
  const sorted = [...expected].sort();
  return keys.length === sorted.length && keys.every((key, index) => key === sorted[index]);
}

export function decodeBase64Url(value: unknown, length: number): Uint8Array {
  if (typeof value !== "string" || !BASE64URL_PATTERN.test(value)) throw new Error("INVALID_SIGNATURE");
  const padding = "=".repeat((4 - value.length % 4) % 4);
  const decoded = Uint8Array.from(
    atob(value.replaceAll("-", "+").replaceAll("_", "/") + padding),
    (character) => character.charCodeAt(0),
  );
  if (decoded.length !== length) throw new Error("INVALID_SIGNATURE");
  return decoded;
}

export function parseLifecycleRequest(value: unknown): LifecycleRequest {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("INVALID_REQUEST");
  const input = value as Record<string, unknown>;
  if ((input.action === "disableWorkspace" || input.action === "restoreWorkspace") &&
      exactKeys(input, ["action", "deviceId", "workspaceId", "signature"])) {
    decodeBase64Url(input.signature, 64);
    return { action: input.action, deviceId: uuid(input.deviceId), workspaceId: uuid(input.workspaceId), signature: input.signature as string };
  }
  if (input.action === "deleteWorkspace" &&
      exactKeys(input, ["action", "deviceId", "workspaceId", "requestId", "signature"])) {
    decodeBase64Url(input.signature, 64);
    return { action: input.action, deviceId: uuid(input.deviceId), workspaceId: uuid(input.workspaceId), requestId: uuid(input.requestId), signature: input.signature as string };
  }
  if (input.action === "deleteAccountSyncData" &&
      exactKeys(input, ["action", "deviceId", "requestId", "signature"])) {
    decodeBase64Url(input.signature, 64);
    return { action: input.action, deviceId: uuid(input.deviceId), requestId: uuid(input.requestId), signature: input.signature as string };
  }
  throw new Error("INVALID_REQUEST");
}

function uuidBytes(value: string): Uint8Array {
  return Uint8Array.from(uuid(value).replaceAll("-", "").match(/.{2}/g) ?? [], (pair) => Number.parseInt(pair, 16));
}

function appendField(target: number[], field: Uint8Array): void {
  target.push((field.length >>> 24) & 0xff, (field.length >>> 16) & 0xff, (field.length >>> 8) & 0xff, field.length & 0xff, ...field);
}

export function buildLifecycleMessage(ownerId: string, sessionId: string, request: LifecycleRequest): Uint8Array {
  const output: number[] = [];
  appendField(output, LABEL);
  appendField(output, uuidBytes(ownerId));
  appendField(output, uuidBytes(sessionId));
  appendField(output, uuidBytes(request.deviceId));
  appendField(output, new TextEncoder().encode(request.action));
  if ("workspaceId" in request) appendField(output, uuidBytes(request.workspaceId));
  if ("requestId" in request) appendField(output, uuidBytes(request.requestId));
  return Uint8Array.from(output);
}

export async function verifyEd25519(publicKey: Uint8Array, signature: Uint8Array, message: Uint8Array): Promise<boolean> {
  try {
    const key = await crypto.subtle.importKey("raw", publicKey, { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, key, signature, message);
  } catch {
    return false;
  }
}
