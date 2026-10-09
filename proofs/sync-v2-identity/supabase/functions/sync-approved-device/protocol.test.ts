import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  buildSourceApprovalMessage,
  buildTargetProofMessage,
  buildTransferKeyRegistrationMessage,
  encodeBase64Url,
  parseApprovedDeviceRequest,
  verifyEd25519,
} from "./protocol.ts";

const fixture = {
  ownerId: "81000000-0000-4000-8000-000000000001",
  sourceSessionId: "82000000-0000-4000-8000-000000000001",
  targetSessionId: "82000000-0000-4000-8000-000000000002",
  sourceDeviceId: "83000000-0000-4000-8000-000000000001",
  targetDeviceId: "83000000-0000-4000-8000-000000000002",
  enrollmentId: "84000000-0000-4000-8000-000000000001",
  workspaceId: "85000000-0000-4000-8000-000000000001",
  targetProofPublicKey: new Uint8Array(32).fill(0x11),
  sourceTransferPublicKey: new Uint8Array(32).fill(0x22),
  targetTransferPublicKey: new Uint8Array(32).fill(0x33),
  challenge: new Uint8Array(32).fill(0x44),
  workspaceKeyVersion: 7,
  encapsulatedKey: new Uint8Array(32).fill(0x55),
  ciphertext: new Uint8Array(48).fill(0x66),
  expiresAt: "2027-01-15T08:00:00.000Z",
};

describe("sync-approved-device protocol", () => {
  it("accepts only exact bounded request shapes", () => {
    const key = encodeBase64Url(new Uint8Array(32).fill(1));
    const signature = encodeBase64Url(new Uint8Array(64).fill(2));
    expect(parseApprovedDeviceRequest({
      action: "registerTransferKey",
      deviceId: fixture.sourceDeviceId,
      transferPublicKey: key,
      signature,
    }).action).toBe("registerTransferKey");
    expect(() => parseApprovedDeviceRequest({
      action: "registerTransferKey",
      deviceId: fixture.sourceDeviceId,
      transferPublicKey: key,
      signature,
      extra: true,
    })).toThrow();
    expect(() => parseApprovedDeviceRequest({
      action: "approveSource",
      enrollmentId: fixture.enrollmentId,
      sourceDeviceId: fixture.sourceDeviceId,
      workspaceId: fixture.workspaceId,
      workspaceKeyVersion: 0,
      encapsulatedKey: key,
      ciphertext: encodeBase64Url(new Uint8Array(48)),
      signature,
    })).toThrow();
  });

  it("keeps the canonical registration, target, and source messages stable", () => {
    const registration = buildTransferKeyRegistrationMessage(
      fixture.ownerId,
      fixture.sourceSessionId,
      fixture.sourceDeviceId,
      fixture.sourceTransferPublicKey,
    );
    const target = buildTargetProofMessage({
      ownerId: fixture.ownerId,
      targetSessionId: fixture.targetSessionId,
      sourceDeviceId: fixture.sourceDeviceId,
      targetDeviceId: fixture.targetDeviceId,
      enrollmentId: fixture.enrollmentId,
      targetProofPublicKey: fixture.targetProofPublicKey,
      targetTransferPublicKey: fixture.targetTransferPublicKey,
      challenge: fixture.challenge,
      expiresAt: fixture.expiresAt,
    });
    const source = buildSourceApprovalMessage(fixture);
    expect(createHash("sha256").update(registration).digest("hex")).toBe("f4a5e11da7f7595b6e56523316116d5ba05d59df84974273663d7d752f487abd");
    expect(createHash("sha256").update(target).digest("hex")).toBe("56dbd5a7174016582cbd5bb11ce79702327ec265cfe71a185406d2e6a60dfd07");
    expect(createHash("sha256").update(source).digest("hex")).toBe("89088c7f8d0dbe52558cd62dc83f814c64bf184b436c37d210aaca31ab402b8b");
  });

  it("independently verifies exact Ed25519 messages", async () => {
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const rawPublicKey = new Uint8Array(publicKey.export({ type: "spki", format: "der" }).subarray(-32));
    const message = buildTargetProofMessage({
      ownerId: fixture.ownerId,
      targetSessionId: fixture.targetSessionId,
      sourceDeviceId: fixture.sourceDeviceId,
      targetDeviceId: fixture.targetDeviceId,
      enrollmentId: fixture.enrollmentId,
      targetProofPublicKey: rawPublicKey,
      targetTransferPublicKey: fixture.targetTransferPublicKey,
      challenge: fixture.challenge,
      expiresAt: fixture.expiresAt,
    });
    const signature = new Uint8Array(sign(null, message, privateKey));
    expect(await verifyEd25519(rawPublicKey, signature, message)).toBe(true);
    const changed = new Uint8Array(message);
    changed[changed.length - 1] ^= 1;
    expect(await verifyEd25519(rawPublicKey, signature, changed)).toBe(false);
  });
});
