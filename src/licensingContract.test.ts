import { describe, expect, it } from "vitest";
import {
  buildEntitlementClaims,
  encodeBase64,
  parseIssueEntitlementRequest,
  signEntitlement,
  verifyEntitlement,
} from "../supabase/functions/_shared/entitlement_contract";

const SUBJECT_ID = "11111111-1111-4111-8111-111111111111";
const DEVICE_ID = "22222222-2222-4222-8222-222222222222";

describe("licensing entitlement contract", () => {
  it("accepts only a local SHA-256 device fingerprint", () => {
    const hash = "a".repeat(64);
    expect(parseIssueEntitlementRequest({ deviceFingerprintHash: hash })).toEqual({
      deviceFingerprintHash: hash,
    });
    expect(() => parseIssueEntitlementRequest({ deviceFingerprintHash: "machine-name" }))
      .toThrow("INVALID_DEVICE_FINGERPRINT");
  });

  it("keeps signed claims minimal and bounds refresh by paid-through time", () => {
    const claims = buildEntitlementClaims({
      keyId: "test-key-1",
      subjectId: SUBJECT_ID,
      deviceId: DEVICE_ID,
      plan: "pro_monthly",
      subscriptionStatus: "active",
      issuedAtUtc: 2_000_000_000,
      paidThroughUtc: 2_000_000_100,
    });

    expect(claims.refreshAfterUtc).toBe(claims.paidThroughUtc);
    expect(Object.keys(claims).sort()).toEqual([
      "deviceId",
      "issuedAtUtc",
      "issuer",
      "keyId",
      "paidThroughUtc",
      "plan",
      "refreshAfterUtc",
      "schemaVersion",
      "subjectId",
      "subscriptionStatus",
    ]);
    expect(JSON.stringify(claims)).not.toMatch(/project|path|email|document/i);
  });

  it("signs and verifies Ed25519 tokens and rejects tampering", async () => {
    const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]);
    const privatePkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));
    const publicRaw = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
    const claims = buildEntitlementClaims({
      keyId: "test-key-1",
      subjectId: SUBJECT_ID,
      deviceId: DEVICE_ID,
      plan: "pro_annual",
      subscriptionStatus: "canceled",
      issuedAtUtc: 2_000_000_000,
      paidThroughUtc: 2_000_100_000,
    });

    const token = await signEntitlement(claims, encodeBase64(privatePkcs8));
    await expect(verifyEntitlement(token, encodeBase64(publicRaw))).resolves.toEqual(claims);

    const parts = token.split(".");
    const replacement = parts[1].endsWith("A") ? "B" : "A";
    const tampered = `${parts[0]}.${parts[1].slice(0, -1)}${replacement}.${parts[2]}`;
    await expect(verifyEntitlement(tampered, encodeBase64(publicRaw)))
      .rejects.toThrow("INVALID_ENTITLEMENT_SIGNATURE");
  });

  it("signs an authoritative expired state without granting Pro", async () => {
    const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]);
    const privatePkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));
    const publicRaw = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
    const claims = buildEntitlementClaims({
      keyId: "test-key-1",
      subjectId: SUBJECT_ID,
      deviceId: DEVICE_ID,
      plan: "pro_monthly",
      subscriptionStatus: "expired",
      issuedAtUtc: 2_000_000_000,
      paidThroughUtc: 2_000_100_000,
    });

    const token = await signEntitlement(claims, encodeBase64(privatePkcs8));
    await expect(verifyEntitlement(token, encodeBase64(publicRaw))).resolves.toEqual(claims);
  });
});
