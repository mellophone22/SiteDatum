import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  buildPushBatchMessage,
  encodeBase64Url,
  parseRecordRequest,
  verifyEd25519,
} from "./protocol.ts";

const ids = {
  ownerId: "71000000-0000-4000-8000-000000000001",
  sessionId: "72000000-0000-4000-8000-000000000001",
  deviceId: "73000000-0000-4000-8000-000000000001",
  workspaceId: "74000000-0000-4000-8000-000000000001",
  batchId: "75000000-0000-4000-8000-000000000001",
  recordId: "76000000-0000-4000-8000-000000000001",
  mutationId: "77000000-0000-4000-8000-000000000001",
};

function pushRequest() {
  return parseRecordRequest({
    action: "pushBatch",
    deviceId: ids.deviceId,
    workspaceId: ids.workspaceId,
    batchId: ids.batchId,
    mutations: [{
      recordId: ids.recordId,
      recordKind: 2,
      expectedServerVersion: 0,
      mutationId: ids.mutationId,
      protocolVersion: 1,
      workspaceKeyVersion: 3,
      ciphertext: encodeBase64Url(new Uint8Array(24).fill(0x41)),
      isTombstone: false,
    }],
    checkpointCounter: 1,
    checkpointProtocolVersion: 1,
    checkpointKeyVersion: 3,
    checkpointCiphertext: encodeBase64Url(new Uint8Array(32).fill(0x42)),
    signature: encodeBase64Url(new Uint8Array(64).fill(0x43)),
  });
}

describe("sync-records protocol", () => {
  it("accepts exact bounded request shapes and rejects ambiguous input", () => {
    expect(parseRecordRequest({
      action: "createWorkspace",
      deviceId: ids.deviceId,
      workspaceId: ids.workspaceId,
    })).toEqual({ action: "createWorkspace", deviceId: ids.deviceId, workspaceId: ids.workspaceId });
    expect(parseRecordRequest({
      action: "pullChanges",
      deviceId: ids.deviceId,
      workspaceId: ids.workspaceId,
      afterCursor: 0,
      limit: 100,
    })).toMatchObject({ action: "pullChanges", limit: 100 });
    expect(pushRequest()).toMatchObject({ action: "pushBatch", checkpointCounter: 1 });
    expect(() => parseRecordRequest({
      action: "createWorkspace",
      deviceId: ids.deviceId,
      workspaceId: ids.workspaceId,
      extra: true,
    })).toThrow();
    expect(() => parseRecordRequest({
      action: "pullChanges",
      deviceId: ids.deviceId,
      workspaceId: ids.workspaceId,
      afterCursor: 0,
      limit: 101,
    })).toThrow();
  });

  it("rejects duplicate record and mutation identifiers", () => {
    const request = pushRequest();
    if (request.action !== "pushBatch") throw new Error("unexpected request");
    const mutation = request.mutations[0];
    expect(() => parseRecordRequest({
      ...request,
      mutations: [mutation, { ...mutation, mutationId: "77000000-0000-4000-8000-000000000002" }],
    })).toThrow("DUPLICATE_MUTATION");
    expect(() => parseRecordRequest({
      ...request,
      mutations: [mutation, { ...mutation, recordId: "76000000-0000-4000-8000-000000000002" }],
    })).toThrow("DUPLICATE_MUTATION");
  });

  it("matches the versioned canonical push-message vector", () => {
    const request = pushRequest();
    if (request.action !== "pushBatch") throw new Error("unexpected request");
    const digest = createHash("sha256")
      .update(buildPushBatchMessage(ids.ownerId, ids.sessionId, request))
      .digest("hex");
    expect(digest).toBe("865597634e6c1f21735c2579ee9d0feb7e00668cbaae4a5b52eed707e5dc7c89");
  });

  it("verifies only the exact signed batch", async () => {
    const request = pushRequest();
    if (request.action !== "pushBatch") throw new Error("unexpected request");
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const rawPublicKey = new Uint8Array(publicKey.export({ type: "spki", format: "der" }).subarray(-32));
    const message = buildPushBatchMessage(ids.ownerId, ids.sessionId, request);
    const signature = new Uint8Array(sign(null, message, privateKey));
    expect(await verifyEd25519(rawPublicKey, signature, message)).toBe(true);
    const changed = buildPushBatchMessage(ids.ownerId, ids.sessionId, {
      ...request,
      checkpointCounter: 2,
    });
    expect(await verifyEd25519(rawPublicKey, signature, changed)).toBe(false);
  });
});
