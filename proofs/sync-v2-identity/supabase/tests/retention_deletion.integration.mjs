import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { generateKeyPairSync, randomUUID, sign } from "node:crypto";
import process from "node:process";

const apiUrl = process.env.SYNC_TEST_API_URL;
const publishableKey = process.env.SYNC_TEST_PUBLISHABLE_KEY;
const secretKey = process.env.SYNC_TEST_SECRET_KEY;
if (!apiUrl || !publishableKey || !secretKey) throw new Error("Local Supabase integration-test environment is incomplete.");

const email = `retention-${randomUUID()}@example.invalid`;
const password = `${randomUUID()}Aa1!`;
let userId;

function base64Url(value) { return Buffer.from(value).toString("base64url"); }
function uuidBytes(value) { return Buffer.from(value.replaceAll("-", ""), "hex"); }
function appendField(parts, value) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(value.length);
  parts.push(length, value);
}
function appendU64(parts, value) {
  const encoded = Buffer.alloc(8);
  encoded.writeBigUInt64BE(BigInt(value));
  parts.push(encoded);
}
function initialProofMessage(value) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.initial-device-possession.v1"));
  for (const id of [value.ownerId, value.sessionId, value.deviceId, value.enrollmentId]) appendField(parts, uuidBytes(id));
  appendField(parts, value.publicKey);
  appendField(parts, value.challenge);
  appendU64(parts, value.expiresAtUnix);
  return Buffer.concat(parts);
}
function lifecycleMessage(ownerId, sessionId, body) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.lifecycle.v1"));
  for (const id of [ownerId, sessionId, body.deviceId]) appendField(parts, uuidBytes(id));
  appendField(parts, Buffer.from(body.action));
  if (body.workspaceId) appendField(parts, uuidBytes(body.workspaceId));
  if (body.requestId) appendField(parts, uuidBytes(body.requestId));
  return Buffer.concat(parts);
}
async function jsonRequest(path, { apiKey, bearer, body, method = "POST" }) {
  const response = await fetch(`${apiUrl}${path}`, {
    method,
    headers: {
      apikey: apiKey,
      authorization: `Bearer ${bearer}`,
      ...(body === undefined ? {} : { "content-type": "application/json" }),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const payload = await response.json().catch(() => ({}));
  return { response, payload };
}
function signedLifecycle(ownerId, sessionId, privateKey, body) {
  return { ...body, signature: base64Url(sign(null, lifecycleMessage(ownerId, sessionId, body), privateKey)) };
}

try {
  const created = await jsonRequest("/auth/v1/admin/users", {
    apiKey: secretKey, bearer: secretKey,
    body: { email, password, email_confirm: true },
  });
  assert.equal(created.response.status, 200);
  userId = created.payload.id;

  const signedIn = await jsonRequest("/auth/v1/token?grant_type=password", {
    apiKey: publishableKey, bearer: publishableKey, body: { email, password },
  });
  assert.equal(signedIn.response.status, 200);
  const accessToken = signedIn.payload.access_token;
  const claims = JSON.parse(Buffer.from(accessToken.split(".")[1], "base64url").toString("utf8"));
  const sessionId = claims.session_id;

  const proof = generateKeyPairSync("ed25519");
  const proofPublic = proof.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const deviceId = randomUUID();
  const enrollment = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "begin", deviceId, publicKey: base64Url(proofPublic) },
  });
  assert.equal(enrollment.response.status, 200, `enrollment begin: ${enrollment.payload.code}`);
  const enrollmentSignature = sign(null, initialProofMessage({
    ownerId: userId, sessionId, deviceId, enrollmentId: enrollment.payload.enrollmentId,
    publicKey: proofPublic, challenge: Buffer.from(enrollment.payload.challenge, "base64url"),
    expiresAtUnix: enrollment.payload.expiresAtUnix,
  }), proof.privateKey);
  const enrolled = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "complete", enrollmentId: enrollment.payload.enrollmentId, signature: base64Url(enrollmentSignature) },
  });
  assert.equal(enrolled.response.status, 200, `enrollment complete: ${enrolled.payload.code}`);

  const workspaceId = randomUUID();
  const createdWorkspace = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "createWorkspace", deviceId, workspaceId },
  });
  assert.equal(createdWorkspace.response.status, 200, `workspace: ${createdWorkspace.payload.code}`);

  const invalidDisable = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "disableWorkspace", deviceId, workspaceId, signature: base64Url(Buffer.alloc(64, 7)) },
  });
  assert.equal(invalidDisable.response.status, 409);
  assert.equal(invalidDisable.payload.code, "LIFECYCLE_NOT_APPLIED");

  const disabled = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken,
    body: signedLifecycle(userId, sessionId, proof.privateKey, { action: "disableWorkspace", deviceId, workspaceId }),
  });
  assert.equal(disabled.response.status, 200, `disable: ${disabled.payload.code}`);
  assert.equal(disabled.payload.outcome, "disabled");
  assert.ok(Date.parse(disabled.payload.purgeAfter) > Date.now());

  const restored = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken,
    body: signedLifecycle(userId, sessionId, proof.privateKey, { action: "restoreWorkspace", deviceId, workspaceId }),
  });
  assert.equal(restored.response.status, 200, `restore: ${restored.payload.code}`);
  assert.equal(restored.payload.outcome, "restored");

  const deletionRequestId = randomUUID();
  const deletionBody = signedLifecycle(userId, sessionId, proof.privateKey, {
    action: "deleteWorkspace", deviceId, workspaceId, requestId: deletionRequestId,
  });
  const deleted = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken, body: deletionBody,
  });
  assert.equal(deleted.response.status, 200, `delete: ${deleted.payload.code}`);
  assert.equal(deleted.payload.outcome, "deleted");
  assert.equal(deleted.payload.policyVersion, "sync-v2-retention-v1");
  assert.ok(deleted.payload.receiptId);

  const retried = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken, body: deletionBody,
  });
  assert.equal(retried.response.status, 200, `delete retry: ${retried.payload.code}`);
  assert.equal(retried.payload.outcome, "already_deleted");
  assert.equal(retried.payload.receiptId, deleted.payload.receiptId);

  const secondWorkspace = randomUUID();
  const secondCreated = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "createWorkspace", deviceId, workspaceId: secondWorkspace },
  });
  assert.equal(secondCreated.response.status, 200);
  const accountDeleted = await jsonRequest("/functions/v1/sync-lifecycle", {
    apiKey: publishableKey, bearer: accessToken,
    body: signedLifecycle(userId, sessionId, proof.privateKey, {
      action: "deleteAccountSyncData", deviceId, requestId: randomUUID(),
    }),
  });
  assert.equal(accountDeleted.response.status, 200, `account delete: ${accountDeleted.payload.code}`);
  assert.equal(accountDeleted.payload.outcome, "deleted");

  const refusedAfterRevocation = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey, bearer: accessToken,
    body: { action: "createWorkspace", deviceId, workspaceId: randomUUID() },
  });
  assert.equal(refusedAfterRevocation.response.status, 409);

  console.log("C10-03H lifecycle integration passed: signed disable/restore, deletion receipt replay, account cleanup, and revocation.");
} finally {
  if (userId) {
    await jsonRequest(`/auth/v1/admin/users/${userId}`, { apiKey: secretKey, bearer: secretKey, method: "DELETE" });
  }
}
