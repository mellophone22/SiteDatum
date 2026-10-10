const PUSH_LABEL = new TextEncoder().encode("sitedatum.sync-v2.record-batch.v1");
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/;
const MAX_RECORD_CIPHERTEXT = 262_144;
const MAX_CHECKPOINT_CIPHERTEXT = 131_072;

export type RecordMutation = {
  recordId: string;
  recordKind: number;
  expectedServerVersion: number;
  mutationId: string;
  protocolVersion: number;
  workspaceKeyVersion: number;
  ciphertext: string;
  isTombstone: boolean;
};

export type RecordRequest =
  | { action: "createWorkspace"; deviceId: string; workspaceId: string }
  | {
    action: "pushBatch";
    deviceId: string;
    workspaceId: string;
    batchId: string;
    mutations: RecordMutation[];
    checkpointCounter: number;
    checkpointProtocolVersion: number;
    checkpointKeyVersion: number;
    checkpointCiphertext: string;
    signature: string;
  }
  | {
    action: "pullChanges";
    deviceId: string;
    workspaceId: string;
    afterCursor: number;
    limit: number;
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

function integer(value: unknown, minimum: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || (value as number) < minimum || (value as number) > maximum) {
    throw new Error("INVALID_INTEGER");
  }
  return value as number;
}

export function decodeBase64Url(value: unknown, minimum: number, maximum = minimum): Uint8Array {
  if (typeof value !== "string" || value.length === 0 || !BASE64URL_PATTERN.test(value)) {
    throw new Error("INVALID_BASE64URL");
  }
  const padding = "=".repeat((4 - value.length % 4) % 4);
  const decoded = Uint8Array.from(
    atob(value.replaceAll("-", "+").replaceAll("_", "/") + padding),
    (character) => character.charCodeAt(0),
  );
  if (decoded.length < minimum || decoded.length > maximum) throw new Error("INVALID_LENGTH");
  return decoded;
}

export function encodeBase64Url(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function parseMutation(value: unknown): RecordMutation {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("INVALID_MUTATION");
  const input = value as Record<string, unknown>;
  if (!exactKeys(input, [
    "recordId", "recordKind", "expectedServerVersion", "mutationId", "protocolVersion",
    "workspaceKeyVersion", "ciphertext", "isTombstone",
  ])) throw new Error("INVALID_MUTATION");
  if (typeof input.isTombstone !== "boolean") throw new Error("INVALID_MUTATION");
  decodeBase64Url(input.ciphertext, 16, MAX_RECORD_CIPHERTEXT);
  return {
    recordId: uuid(input.recordId),
    recordKind: integer(input.recordKind, 1, 13),
    expectedServerVersion: integer(input.expectedServerVersion, 0, Number.MAX_SAFE_INTEGER),
    mutationId: uuid(input.mutationId),
    protocolVersion: integer(input.protocolVersion, 1, 1),
    workspaceKeyVersion: integer(input.workspaceKeyVersion, 1, 2_147_483_647),
    ciphertext: input.ciphertext as string,
    isTombstone: input.isTombstone,
  };
}

export function parseRecordRequest(value: unknown): RecordRequest {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("INVALID_REQUEST");
  const input = value as Record<string, unknown>;
  if (input.action === "createWorkspace" && exactKeys(input, ["action", "deviceId", "workspaceId"])) {
    return { action: input.action, deviceId: uuid(input.deviceId), workspaceId: uuid(input.workspaceId) };
  }
  if (input.action === "pullChanges" && exactKeys(input, [
    "action", "deviceId", "workspaceId", "afterCursor", "limit",
  ])) {
    return {
      action: input.action,
      deviceId: uuid(input.deviceId),
      workspaceId: uuid(input.workspaceId),
      afterCursor: integer(input.afterCursor, 0, Number.MAX_SAFE_INTEGER),
      limit: integer(input.limit, 1, 100),
    };
  }
  if (input.action === "pushBatch" && exactKeys(input, [
    "action", "deviceId", "workspaceId", "batchId", "mutations", "checkpointCounter",
    "checkpointProtocolVersion", "checkpointKeyVersion", "checkpointCiphertext", "signature",
  ])) {
    if (!Array.isArray(input.mutations) || input.mutations.length < 1 || input.mutations.length > 50) {
      throw new Error("INVALID_MUTATIONS");
    }
    const mutations = input.mutations.map(parseMutation);
    if (new Set(mutations.map((item) => item.recordId)).size !== mutations.length ||
        new Set(mutations.map((item) => item.mutationId)).size !== mutations.length) {
      throw new Error("DUPLICATE_MUTATION");
    }
    decodeBase64Url(input.checkpointCiphertext, 16, MAX_CHECKPOINT_CIPHERTEXT);
    decodeBase64Url(input.signature, 64);
    return {
      action: input.action,
      deviceId: uuid(input.deviceId),
      workspaceId: uuid(input.workspaceId),
      batchId: uuid(input.batchId),
      mutations,
      checkpointCounter: integer(input.checkpointCounter, 1, Number.MAX_SAFE_INTEGER),
      checkpointProtocolVersion: integer(input.checkpointProtocolVersion, 1, 1),
      checkpointKeyVersion: integer(input.checkpointKeyVersion, 1, 2_147_483_647),
      checkpointCiphertext: input.checkpointCiphertext as string,
      signature: input.signature as string,
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

function appendU32(target: number[], value: number): void {
  target.push((value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff);
}

function appendU64(target: number[], value: number): void {
  let remaining = BigInt(value);
  const encoded = new Uint8Array(8);
  for (let index = 7; index >= 0; index -= 1) {
    encoded[index] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
  target.push(...encoded);
}

export function buildPushBatchMessage(
  ownerId: string,
  sessionId: string,
  request: Extract<RecordRequest, { action: "pushBatch" }>,
): Uint8Array {
  const output: number[] = [];
  appendField(output, PUSH_LABEL);
  appendField(output, uuidBytes(ownerId));
  appendField(output, uuidBytes(sessionId));
  appendField(output, uuidBytes(request.deviceId));
  appendField(output, uuidBytes(request.workspaceId));
  appendField(output, uuidBytes(request.batchId));
  appendU64(output, request.checkpointCounter);
  appendU32(output, request.checkpointProtocolVersion);
  appendU32(output, request.checkpointKeyVersion);
  appendU32(output, request.mutations.length);
  for (const mutation of request.mutations) {
    appendField(output, uuidBytes(mutation.recordId));
    appendU32(output, mutation.recordKind);
    appendU64(output, mutation.expectedServerVersion);
    appendField(output, uuidBytes(mutation.mutationId));
    appendU32(output, mutation.protocolVersion);
    appendU32(output, mutation.workspaceKeyVersion);
    output.push(mutation.isTombstone ? 1 : 0);
    appendField(output, decodeBase64Url(mutation.ciphertext, 16, MAX_RECORD_CIPHERTEXT));
  }
  appendField(output, decodeBase64Url(request.checkpointCiphertext, 16, MAX_CHECKPOINT_CIPHERTEXT));
  return Uint8Array.from(output);
}

export async function verifyEd25519(
  publicKey: Uint8Array,
  signature: Uint8Array,
  message: Uint8Array,
): Promise<boolean> {
  try {
    const key = await crypto.subtle.importKey("raw", publicKey, { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, key, signature, message);
  } catch {
    return false;
  }
}
