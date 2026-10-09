const REGISTER_LABEL = new TextEncoder().encode("sitedatum.sync-v2.transfer-key-registration.v1");
const TARGET_LABEL = new TextEncoder().encode("sitedatum.sync-v2.approved-target-possession.v1");
const SOURCE_LABEL = new TextEncoder().encode("sitedatum.sync-v2.approved-source-transfer.v1");
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/;

export type ApprovedDeviceRequest =
  | { action: "registerTransferKey"; deviceId: string; transferPublicKey: string; signature: string }
  | {
    action: "beginTarget";
    sourceDeviceId: string;
    targetDeviceId: string;
    targetProofPublicKey: string;
    targetTransferPublicKey: string;
  }
  | { action: "proveTarget"; enrollmentId: string; signature: string }
  | { action: "prepareSource"; enrollmentId: string; sourceDeviceId: string }
  | {
    action: "approveSource";
    enrollmentId: string;
    sourceDeviceId: string;
    workspaceId: string;
    workspaceKeyVersion: number;
    encapsulatedKey: string;
    ciphertext: string;
    signature: string;
  }
  | { action: "fetchTransfer"; enrollmentId: string; targetDeviceId: string };

export type TargetProofContext = {
  ownerId: string;
  targetSessionId: string;
  sourceDeviceId: string;
  targetDeviceId: string;
  enrollmentId: string;
  targetProofPublicKey: Uint8Array;
  targetTransferPublicKey: Uint8Array;
  challenge: Uint8Array;
  expiresAt: string;
};

export type SourceApprovalContext = TargetProofContext & {
  sourceSessionId: string;
  sourceTransferPublicKey: Uint8Array;
  workspaceId: string;
  workspaceKeyVersion: number;
  encapsulatedKey: Uint8Array;
  ciphertext: Uint8Array;
};

function exactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const keys = Object.keys(value).sort();
  const sorted = [...expected].sort();
  return keys.length === sorted.length && keys.every((key, index) => key === sorted[index]);
}

function uuid(value: unknown): string {
  if (typeof value !== "string" || !UUID_PATTERN.test(value)) throw new Error("INVALID_UUID");
  return value.toLowerCase();
}

export function decodeBase64Url(value: unknown, expectedLength?: number): Uint8Array {
  if (typeof value !== "string" || value.length === 0 || !BASE64URL_PATTERN.test(value)) {
    throw new Error("INVALID_BASE64URL");
  }
  const padding = "=".repeat((4 - value.length % 4) % 4);
  const decoded = Uint8Array.from(
    atob(value.replaceAll("-", "+").replaceAll("_", "/") + padding),
    (character) => character.charCodeAt(0),
  );
  if (expectedLength !== undefined && decoded.length !== expectedLength) throw new Error("INVALID_LENGTH");
  return decoded;
}

export function encodeBase64Url(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export function parseApprovedDeviceRequest(value: unknown): ApprovedDeviceRequest {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("INVALID_REQUEST");
  const input = value as Record<string, unknown>;
  if (input.action === "registerTransferKey" &&
      exactKeys(input, ["action", "deviceId", "transferPublicKey", "signature"])) {
    decodeBase64Url(input.transferPublicKey, 32);
    decodeBase64Url(input.signature, 64);
    return {
      action: input.action,
      deviceId: uuid(input.deviceId),
      transferPublicKey: input.transferPublicKey as string,
      signature: input.signature as string,
    };
  }
  if (input.action === "beginTarget" && exactKeys(input, [
    "action", "sourceDeviceId", "targetDeviceId", "targetProofPublicKey", "targetTransferPublicKey",
  ])) {
    decodeBase64Url(input.targetProofPublicKey, 32);
    decodeBase64Url(input.targetTransferPublicKey, 32);
    return {
      action: input.action,
      sourceDeviceId: uuid(input.sourceDeviceId),
      targetDeviceId: uuid(input.targetDeviceId),
      targetProofPublicKey: input.targetProofPublicKey as string,
      targetTransferPublicKey: input.targetTransferPublicKey as string,
    };
  }
  if (input.action === "proveTarget" && exactKeys(input, ["action", "enrollmentId", "signature"])) {
    decodeBase64Url(input.signature, 64);
    return { action: input.action, enrollmentId: uuid(input.enrollmentId), signature: input.signature as string };
  }
  if (input.action === "prepareSource" && exactKeys(input, ["action", "enrollmentId", "sourceDeviceId"])) {
    return {
      action: input.action,
      enrollmentId: uuid(input.enrollmentId),
      sourceDeviceId: uuid(input.sourceDeviceId),
    };
  }
  if (input.action === "approveSource" && exactKeys(input, [
    "action", "enrollmentId", "sourceDeviceId", "workspaceId", "workspaceKeyVersion",
    "encapsulatedKey", "ciphertext", "signature",
  ])) {
    if (!Number.isSafeInteger(input.workspaceKeyVersion) || (input.workspaceKeyVersion as number) < 1 ||
        (input.workspaceKeyVersion as number) > 2_147_483_647) throw new Error("INVALID_VERSION");
    decodeBase64Url(input.encapsulatedKey, 32);
    decodeBase64Url(input.ciphertext, 48);
    decodeBase64Url(input.signature, 64);
    return {
      action: input.action,
      enrollmentId: uuid(input.enrollmentId),
      sourceDeviceId: uuid(input.sourceDeviceId),
      workspaceId: uuid(input.workspaceId),
      workspaceKeyVersion: input.workspaceKeyVersion as number,
      encapsulatedKey: input.encapsulatedKey as string,
      ciphertext: input.ciphertext as string,
      signature: input.signature as string,
    };
  }
  if (input.action === "fetchTransfer" && exactKeys(input, ["action", "enrollmentId", "targetDeviceId"])) {
    return {
      action: input.action,
      enrollmentId: uuid(input.enrollmentId),
      targetDeviceId: uuid(input.targetDeviceId),
    };
  }
  throw new Error("INVALID_REQUEST");
}

function uuidBytes(value: string): Uint8Array {
  return Uint8Array.from(uuid(value).replaceAll("-", "").match(/.{2}/g) ?? [], (pair) => Number.parseInt(pair, 16));
}

function appendField(target: number[], field: Uint8Array): void {
  const length = field.length;
  target.push((length >>> 24) & 0xff, (length >>> 16) & 0xff, (length >>> 8) & 0xff, length & 0xff, ...field);
}

function appendExpiry(target: number[], expiresAt: string): void {
  const millis = Date.parse(expiresAt);
  if (!Number.isFinite(millis) || millis < 0) throw new Error("INVALID_EXPIRY");
  let seconds = BigInt(Math.floor(millis / 1000));
  const encoded = new Uint8Array(8);
  for (let index = 7; index >= 0; index -= 1) {
    encoded[index] = Number(seconds & 0xffn);
    seconds >>= 8n;
  }
  target.push(...encoded);
}

export function buildTransferKeyRegistrationMessage(
  ownerId: string,
  sessionId: string,
  deviceId: string,
  transferPublicKey: Uint8Array,
): Uint8Array {
  if (transferPublicKey.length !== 32) throw new Error("INVALID_CONTEXT");
  const output: number[] = [];
  appendField(output, REGISTER_LABEL);
  appendField(output, uuidBytes(ownerId));
  appendField(output, uuidBytes(sessionId));
  appendField(output, uuidBytes(deviceId));
  appendField(output, transferPublicKey);
  return Uint8Array.from(output);
}

export function buildTargetProofMessage(context: TargetProofContext): Uint8Array {
  if (context.targetProofPublicKey.length !== 32 || context.targetTransferPublicKey.length !== 32 ||
      context.challenge.length !== 32) throw new Error("INVALID_CONTEXT");
  const output: number[] = [];
  appendField(output, TARGET_LABEL);
  appendField(output, uuidBytes(context.ownerId));
  appendField(output, uuidBytes(context.targetSessionId));
  appendField(output, uuidBytes(context.sourceDeviceId));
  appendField(output, uuidBytes(context.targetDeviceId));
  appendField(output, uuidBytes(context.enrollmentId));
  appendField(output, context.targetProofPublicKey);
  appendField(output, context.targetTransferPublicKey);
  appendField(output, context.challenge);
  appendExpiry(output, context.expiresAt);
  return Uint8Array.from(output);
}

export function buildSourceApprovalMessage(context: SourceApprovalContext): Uint8Array {
  if (context.sourceTransferPublicKey.length !== 32 || context.encapsulatedKey.length !== 32 ||
      context.ciphertext.length !== 48 || !Number.isSafeInteger(context.workspaceKeyVersion) ||
      context.workspaceKeyVersion < 1) throw new Error("INVALID_CONTEXT");
  const output: number[] = [];
  appendField(output, SOURCE_LABEL);
  appendField(output, uuidBytes(context.ownerId));
  appendField(output, uuidBytes(context.sourceSessionId));
  appendField(output, uuidBytes(context.targetSessionId));
  appendField(output, uuidBytes(context.sourceDeviceId));
  appendField(output, uuidBytes(context.targetDeviceId));
  appendField(output, uuidBytes(context.enrollmentId));
  appendField(output, context.targetProofPublicKey);
  appendField(output, context.sourceTransferPublicKey);
  appendField(output, context.targetTransferPublicKey);
  appendField(output, context.challenge);
  appendField(output, uuidBytes(context.workspaceId));
  output.push(
    (context.workspaceKeyVersion >>> 24) & 0xff,
    (context.workspaceKeyVersion >>> 16) & 0xff,
    (context.workspaceKeyVersion >>> 8) & 0xff,
    context.workspaceKeyVersion & 0xff,
  );
  appendExpiry(output, context.expiresAt);
  appendField(output, context.encapsulatedKey);
  appendField(output, context.ciphertext);
  return Uint8Array.from(output);
}

export async function verifyEd25519(
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
