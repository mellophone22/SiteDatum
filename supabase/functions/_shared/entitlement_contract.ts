export const ENTITLEMENT_SCHEMA_VERSION = 1;
export const ENTITLEMENT_ISSUER = "sitedatum-licensing";
export const ENTITLEMENT_REFRESH_SECONDS = 7 * 24 * 60 * 60;

export type LicensedPlan = "pro_monthly" | "pro_annual";
export type LicensedSubscriptionStatus = "active" | "past_due" | "canceled" | "expired";
export type LicensedAccessKind = "paid" | "complimentary";

export type EntitlementClaims = {
  schemaVersion: 1;
  issuer: typeof ENTITLEMENT_ISSUER;
  keyId: string;
  subjectId: string;
  deviceId: string;
  plan: LicensedPlan;
  accessKind: LicensedAccessKind;
  subscriptionStatus: LicensedSubscriptionStatus;
  issuedAtUtc: number;
  refreshAfterUtc: number;
  paidThroughUtc: number;
};

export type IssueEntitlementRequest = {
  deviceFingerprintHash: string;
};

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const SHA256_HEX = /^[0-9a-f]{64}$/;
const KEY_ID = /^[a-zA-Z0-9._-]{1,64}$/;

export function parseIssueEntitlementRequest(value: unknown): IssueEntitlementRequest {
  if (!value || typeof value !== "object") {
    throw new Error("INVALID_REQUEST");
  }
  const fingerprint = (value as Record<string, unknown>).deviceFingerprintHash;
  if (typeof fingerprint !== "string" || !SHA256_HEX.test(fingerprint)) {
    throw new Error("INVALID_DEVICE_FINGERPRINT");
  }
  return { deviceFingerprintHash: fingerprint };
}

export function buildEntitlementClaims(input: {
  keyId: string;
  subjectId: string;
  deviceId: string;
  plan: LicensedPlan;
  accessKind?: LicensedAccessKind;
  subscriptionStatus: LicensedSubscriptionStatus;
  issuedAtUtc: number;
  paidThroughUtc: number;
}): EntitlementClaims {
  if (!KEY_ID.test(input.keyId) || !UUID.test(input.subjectId) || !UUID.test(input.deviceId)) {
    throw new Error("INVALID_ENTITLEMENT_IDENTITY");
  }
  if (!Number.isSafeInteger(input.issuedAtUtc) || !Number.isSafeInteger(input.paidThroughUtc)) {
    throw new Error("INVALID_ENTITLEMENT_TIME");
  }
  if (input.paidThroughUtc <= input.issuedAtUtc) {
    throw new Error("ENTITLEMENT_EXPIRED");
  }

  return {
    schemaVersion: ENTITLEMENT_SCHEMA_VERSION,
    issuer: ENTITLEMENT_ISSUER,
    keyId: input.keyId,
    subjectId: input.subjectId,
    deviceId: input.deviceId,
    plan: input.plan,
    accessKind: input.accessKind ?? "paid",
    subscriptionStatus: input.subscriptionStatus,
    issuedAtUtc: input.issuedAtUtc,
    refreshAfterUtc: Math.min(
      input.issuedAtUtc + ENTITLEMENT_REFRESH_SECONDS,
      input.paidThroughUtc,
    ),
    paidThroughUtc: input.paidThroughUtc,
  };
}

export async function signEntitlement(
  claims: EntitlementClaims,
  privateKeyPkcs8Base64: string,
): Promise<string> {
  const privateKey = await crypto.subtle.importKey(
    "pkcs8",
    toArrayBuffer(decodeBase64(privateKeyPkcs8Base64)),
    { name: "Ed25519" },
    false,
    ["sign"],
  );
  const payload = encodeBase64Url(new TextEncoder().encode(JSON.stringify(claims)));
  const signedBytes = new TextEncoder().encode(`sd1.${payload}`);
  const signature = await crypto.subtle.sign("Ed25519", privateKey, signedBytes);
  return `sd1.${payload}.${encodeBase64Url(new Uint8Array(signature))}`;
}

export async function verifyEntitlement(
  token: string,
  publicKeyRawBase64: string,
): Promise<EntitlementClaims> {
  const parts = token.split(".");
  if (parts.length !== 3 || parts[0] !== "sd1") {
    throw new Error("INVALID_ENTITLEMENT_TOKEN");
  }
  const publicKey = await crypto.subtle.importKey(
    "raw",
    toArrayBuffer(decodeBase64(publicKeyRawBase64)),
    { name: "Ed25519" },
    false,
    ["verify"],
  );
  const valid = await crypto.subtle.verify(
    "Ed25519",
    publicKey,
    toArrayBuffer(decodeBase64Url(parts[2])),
    new TextEncoder().encode(`sd1.${parts[1]}`),
  );
  if (!valid) {
    throw new Error("INVALID_ENTITLEMENT_SIGNATURE");
  }
  const claims = JSON.parse(new TextDecoder().decode(decodeBase64Url(parts[1]))) as unknown;
  if (!isEntitlementClaims(claims)) {
    throw new Error("INVALID_ENTITLEMENT_CLAIMS");
  }
  return claims;
}

export function encodeBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function decodeBase64(value: string): Uint8Array {
  try {
    return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
  } catch {
    throw new Error("INVALID_BASE64");
  }
}

function encodeBase64Url(bytes: Uint8Array): string {
  return encodeBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function decodeBase64Url(value: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]+$/.test(value)) throw new Error("INVALID_BASE64URL");
  const standard = value.replace(/-/g, "+").replace(/_/g, "/");
  return decodeBase64(standard.padEnd(Math.ceil(standard.length / 4) * 4, "="));
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return buffer;
}

function isEntitlementClaims(value: unknown): value is EntitlementClaims {
  if (!value || typeof value !== "object") return false;
  const claims = value as Record<string, unknown>;
  const expectedKeys = [
    "accessKind", "deviceId", "issuedAtUtc", "issuer", "keyId", "paidThroughUtc", "plan",
    "refreshAfterUtc", "schemaVersion", "subjectId", "subscriptionStatus",
  ];
  return Object.keys(claims).sort().join("|") === expectedKeys.join("|")
    && claims.schemaVersion === ENTITLEMENT_SCHEMA_VERSION
    && claims.issuer === ENTITLEMENT_ISSUER
    && typeof claims.keyId === "string" && KEY_ID.test(claims.keyId)
    && typeof claims.subjectId === "string" && UUID.test(claims.subjectId)
    && typeof claims.deviceId === "string" && UUID.test(claims.deviceId)
    && (claims.plan === "pro_monthly" || claims.plan === "pro_annual")
    && (claims.accessKind === "paid" || claims.accessKind === "complimentary")
    && ["active", "past_due", "canceled", "expired"].includes(String(claims.subscriptionStatus))
    && Number.isSafeInteger(claims.issuedAtUtc)
    && Number.isSafeInteger(claims.refreshAfterUtc)
    && Number.isSafeInteger(claims.paidThroughUtc)
    && Number(claims.issuedAtUtc) <= Number(claims.refreshAfterUtc)
    && Number(claims.refreshAfterUtc) <= Number(claims.paidThroughUtc);
}
