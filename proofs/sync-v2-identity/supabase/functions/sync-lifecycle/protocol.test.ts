import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { describe, expect, it } from "vitest";
import { buildLifecycleMessage, parseLifecycleRequest, verifyEd25519 } from "./protocol.ts";

const ids = {
  ownerId: "81000000-0000-4000-8000-000000000001",
  sessionId: "82000000-0000-4000-8000-000000000001",
  deviceId: "83000000-0000-4000-8000-000000000001",
  workspaceId: "84000000-0000-4000-8000-000000000001",
  requestId: "85000000-0000-4000-8000-000000000001",
};

const signature = "Q".repeat(86);

describe("sync-lifecycle protocol", () => {
  it("accepts every exact lifecycle shape", () => {
    expect(parseLifecycleRequest({ action: "disableWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, signature })).toMatchObject({ action: "disableWorkspace" });
    expect(parseLifecycleRequest({ action: "restoreWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, signature })).toMatchObject({ action: "restoreWorkspace" });
    expect(parseLifecycleRequest({ action: "deleteWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, requestId: ids.requestId, signature })).toMatchObject({ action: "deleteWorkspace" });
    expect(parseLifecycleRequest({ action: "deleteAccountSyncData", deviceId: ids.deviceId, requestId: ids.requestId, signature })).toMatchObject({ action: "deleteAccountSyncData" });
  });

  it("rejects unknown fields, invalid identifiers, and malformed signatures", () => {
    expect(() => parseLifecycleRequest({ action: "disableWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, signature, extra: true })).toThrow();
    expect(() => parseLifecycleRequest({ action: "deleteWorkspace", deviceId: ids.deviceId, workspaceId: "not-a-uuid", requestId: ids.requestId, signature })).toThrow();
    expect(() => parseLifecycleRequest({ action: "deleteAccountSyncData", deviceId: ids.deviceId, requestId: ids.requestId, signature: "bad!" })).toThrow();
  });

  it("separates actions and request identifiers in the canonical message", () => {
    const disable = parseLifecycleRequest({ action: "disableWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, signature });
    const remove = parseLifecycleRequest({ action: "deleteWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, requestId: ids.requestId, signature });
    const disableDigest = createHash("sha256").update(buildLifecycleMessage(ids.ownerId, ids.sessionId, disable)).digest("hex");
    const deleteDigest = createHash("sha256").update(buildLifecycleMessage(ids.ownerId, ids.sessionId, remove)).digest("hex");
    expect(disableDigest).not.toBe(deleteDigest);
    expect(disableDigest).toHaveLength(64);
    expect(deleteDigest).toHaveLength(64);
  });

  it("requires a signature over the exact owner, session, action, and target", async () => {
    const request = parseLifecycleRequest({ action: "deleteWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId, requestId: ids.requestId, signature });
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const rawPublicKey = new Uint8Array(publicKey.export({ type: "spki", format: "der" }).subarray(-32));
    const message = buildLifecycleMessage(ids.ownerId, ids.sessionId, request);
    const signed = new Uint8Array(sign(null, message, privateKey));
    expect(await verifyEd25519(rawPublicKey, signed, message)).toBe(true);
    expect(await verifyEd25519(rawPublicKey, signed, buildLifecycleMessage(ids.ownerId, "82000000-0000-4000-8000-000000000002", request))).toBe(false);
  });
});
