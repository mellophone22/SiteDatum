const CONTEXT_LABEL = new TextEncoder().encode("sitedatum.sync-v2.initial-device-possession.v1");
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/;

export type EnrollmentRequest =
  | { action: "begin"; deviceId: string; publicKeyBase64: string }
  | { action: "complete"; enrollmentId: string; signature: string };

export type EnrollmentProofContext = {
  ownerId: string;
  sessionId: string;
  deviceId: string;
  enrollmentId: string;
  publicKey: Uint8Array;
  challenge: Uint8Array;
  expiresAt: string;
};

function exactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const keys = Object.keys(value).sort();
  const sortedExpected = [...expected].sort();
  return keys.length === sortedExpected.length && keys.every((key, index) => key === sortedExpected[index]);
}

function uuid(value: unknown): string {
  if (typeof value !== "string" || !UUID_PATTERN.test(value)) throw new Error("INVALID_UUID");
  return value.toLowerCase();
}

export function decodeBase64Url(value: unknown): Uint8Array {
  if (typeof value !== "string" || value.length === 0 || !BASE64URL_PATTERN.test(value)) {
    throw new Error("INVALID_BASE64URL");
  }
  const padding = "=".repeat((4 - value.length % 4) % 4);
  const decoded = atob(value.replaceAll("-", "+").replaceAll("_", "/") + padding);
  return Uint8Array.from(decoded, (character) => character.charCodeAt(0));
}

export function encodeBase64Url(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export function parseEnrollmentRequest(value: unknown): EnrollmentRequest {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("INVALID_REQUEST");
  const input = value as Record<string, unknown>;
  if (input.action === "begin" && exactKeys(input, ["action", "deviceId", "publicKey"])) {
    const publicKey = decodeBase64Url(input.publicKey);
    if (publicKey.length !== 32) throw new Error("INVALID_PUBLIC_KEY");
    return {
      action: "begin",
      deviceId: uuid(input.deviceId),
      publicKeyBase64: btoa(String.fromCharCode(...publicKey)),
    };
  }
  if (input.action === "complete" && exactKeys(input, ["action", "enrollmentId", "signature"])) {
    const signature = decodeBase64Url(input.signature);
    if (signature.length !== 64) throw new Error("INVALID_SIGNATURE");
    return { action: "complete", enrollmentId: uuid(input.enrollmentId), signature: input.signature as string };
  }
  throw new Error("INVALID_REQUEST");
}

function uuidBytes(value: string): Uint8Array {
  const normalized = uuid(value).replaceAll("-", "");
  return Uint8Array.from(normalized.match(/.{2}/g) ?? [], (pair) => Number.parseInt(pair, 16));
}

function appendField(target: number[], field: Uint8Array): void {
  const length = field.length;
  target.push((length >>> 24) & 0xff, (length >>> 16) & 0xff, (length >>> 8) & 0xff, length & 0xff, ...field);
}

export function buildEnrollmentProofMessage(context: EnrollmentProofContext): Uint8Array {
  if (context.publicKey.length !== 32 || context.challenge.length !== 32) throw new Error("INVALID_CONTEXT");
  const expiresAtMillis = Date.parse(context.expiresAt);
  if (!Number.isFinite(expiresAtMillis) || expiresAtMillis < 0) throw new Error("INVALID_EXPIRY");

  const output: number[] = [];
  appendField(output, CONTEXT_LABEL);
  appendField(output, uuidBytes(context.ownerId));
  appendField(output, uuidBytes(context.sessionId));
  appendField(output, uuidBytes(context.deviceId));
  appendField(output, uuidBytes(context.enrollmentId));
  appendField(output, context.publicKey);
  appendField(output, context.challenge);

  let seconds = BigInt(Math.floor(expiresAtMillis / 1000));
  const encodedSeconds = new Uint8Array(8);
  for (let index = 7; index >= 0; index -= 1) {
    encodedSeconds[index] = Number(seconds & 0xffn);
    seconds >>= 8n;
  }
  output.push(...encodedSeconds);
  return Uint8Array.from(output);
}

export async function verifyEnrollmentProof(
  publicKey: Uint8Array,
  signature: Uint8Array,
  message: Uint8Array,
): Promise<boolean> {
  if (publicKey.length !== 32 || signature.length !== 64) return false;
  try {
    const key = await crypto.subtle.importKey("raw", publicKey, { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, key, signature, message);
  } catch {
    return false;
  }
}
