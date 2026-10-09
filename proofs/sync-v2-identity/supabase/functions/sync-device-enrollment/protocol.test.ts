import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  buildEnrollmentProofMessage,
  encodeBase64Url,
  parseEnrollmentRequest,
  verifyEnrollmentProof,
} from "./protocol.ts";

const context = {
  ownerId: "61000000-0000-4000-8000-000000000001",
  sessionId: "62000000-0000-4000-8000-000000000001",
  deviceId: "63000000-0000-4000-8000-000000000001",
  enrollmentId: "64000000-0000-4000-8000-000000000001",
  publicKey: Uint8Array.from({ length: 32 }, (_, index) => index),
  challenge: new Uint8Array(32).fill(0xa5),
  expiresAt: "2027-01-15T08:00:00.000Z",
};

describe("sync-device-enrollment protocol", () => {
  it("accepts only the exact begin and completion request shapes", () => {
    const publicKey = encodeBase64Url(new Uint8Array(32).fill(0x11));
    const signature = encodeBase64Url(new Uint8Array(64).fill(0x22));
    expect(parseEnrollmentRequest({ action: "begin", deviceId: context.deviceId, publicKey })).toMatchObject({
      action: "begin",
      deviceId: context.deviceId,
    });
    expect(parseEnrollmentRequest({ action: "complete", enrollmentId: context.enrollmentId, signature })).toEqual({
      action: "complete",
      enrollmentId: context.enrollmentId,
      signature,
    });
    expect(() => parseEnrollmentRequest({ action: "begin", deviceId: context.deviceId, publicKey, extra: true })).toThrow();
    expect(() => parseEnrollmentRequest({ action: "complete", enrollmentId: context.enrollmentId, signature: "AA" })).toThrow();
  });

  it("matches the versioned cross-language canonical message vector", () => {
    const digest = createHash("sha256").update(buildEnrollmentProofMessage(context)).digest("hex");
    expect(digest).toBe("1fb13514a81b5f847ed102855decd6c9d6a49761fbc48b16dcdfb8c5665897c4");
  });

  it("verifies the exact Ed25519 context and rejects a changed challenge", async () => {
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const rawPublicKey = new Uint8Array(publicKey.export({ type: "spki", format: "der" }).subarray(-32));
    const exactContext = { ...context, publicKey: rawPublicKey };
    const message = buildEnrollmentProofMessage(exactContext);
    const signature = new Uint8Array(sign(null, message, privateKey));
    expect(await verifyEnrollmentProof(rawPublicKey, signature, message)).toBe(true);

    const changed = buildEnrollmentProofMessage({ ...exactContext, challenge: new Uint8Array(32).fill(0xa4) });
    expect(await verifyEnrollmentProof(rawPublicKey, signature, changed)).toBe(false);
  });
});
