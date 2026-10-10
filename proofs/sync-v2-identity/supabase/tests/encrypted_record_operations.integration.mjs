import { generateKeyPairSync, randomUUID, sign } from "node:crypto";
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import process from "node:process";

const apiUrl = process.env.SYNC_TEST_API_URL;
const publishableKey = process.env.SYNC_TEST_PUBLISHABLE_KEY;
const secretKey = process.env.SYNC_TEST_SECRET_KEY;
if (!apiUrl || !publishableKey || !secretKey) throw new Error("Local Supabase integration-test environment is incomplete.");

const email = `encrypted-records-${randomUUID()}@example.invalid`;
const password = `${randomUUID()}Aa1!`;
let userId;

function base64Url(value) { return Buffer.from(value).toString("base64url"); }
function uuidBytes(value) { return Buffer.from(value.replaceAll("-", ""), "hex"); }
function appendField(parts, value) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(value.length);
  parts.push(length, value);
}
function appendU32(parts, value) {
  const encoded = Buffer.alloc(4);
  encoded.writeUInt32BE(value);
  parts.push(encoded);
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
function pushBatchMessage(value) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.record-batch.v1"));
  for (const id of [value.ownerId, value.sessionId, value.deviceId, value.workspaceId, value.batchId]) {
    appendField(parts, uuidBytes(id));
  }
  appendU64(parts, value.checkpointCounter);
  appendU32(parts, value.checkpointProtocolVersion);
  appendU32(parts, value.checkpointKeyVersion);
  appendU32(parts, value.mutations.length);
  for (const mutation of value.mutations) {
    appendField(parts, uuidBytes(mutation.recordId));
    appendU32(parts, mutation.recordKind);
    appendU64(parts, mutation.expectedServerVersion);
    appendField(parts, uuidBytes(mutation.mutationId));
    appendU32(parts, mutation.protocolVersion);
    appendU32(parts, mutation.workspaceKeyVersion);
    parts.push(Buffer.from([mutation.isTombstone ? 1 : 0]));
    appendField(parts, Buffer.from(mutation.ciphertext, "base64url"));
  }
  appendField(parts, Buffer.from(value.checkpointCiphertext, "base64url"));
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

try {
  const created = await jsonRequest("/auth/v1/admin/users", {
    apiKey: secretKey,
    bearer: secretKey,
    body: { email, password, email_confirm: true },
  });
  assert.equal(created.response.status, 200);
  userId = created.payload.id;

  const signedIn = await jsonRequest("/auth/v1/token?grant_type=password", {
    apiKey: publishableKey,
    bearer: publishableKey,
    body: { email, password },
  });
  assert.equal(signedIn.response.status, 200);
  const accessToken = signedIn.payload.access_token;
  const claims = JSON.parse(Buffer.from(accessToken.split(".")[1], "base64url").toString("utf8"));
  const sessionId = claims.session_id;

  const proof = generateKeyPairSync("ed25519");
  const proofPublic = proof.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const deviceId = randomUUID();
  const enrollment = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "begin", deviceId, publicKey: base64Url(proofPublic) },
  });
  assert.equal(enrollment.response.status, 200, `enrollment begin: ${enrollment.payload.code}`);
  const enrollmentSignature = sign(null, initialProofMessage({
    ownerId: userId,
    sessionId,
    deviceId,
    enrollmentId: enrollment.payload.enrollmentId,
    publicKey: proofPublic,
    challenge: Buffer.from(enrollment.payload.challenge, "base64url"),
    expiresAtUnix: enrollment.payload.expiresAtUnix,
  }), proof.privateKey);
  const enrolled = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "complete", enrollmentId: enrollment.payload.enrollmentId, signature: base64Url(enrollmentSignature) },
  });
  assert.equal(enrolled.response.status, 200, `enrollment complete: ${enrolled.payload.code}`);

  const workspaceId = randomUUID();
  const workspace = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "createWorkspace", deviceId, workspaceId },
  });
  assert.equal(workspace.response.status, 200, `workspace: ${workspace.payload.code}`);
  assert.equal(workspace.payload.outcome, "created");

  const ciphertexts = [Buffer.alloc(24, 0xa1), Buffer.alloc(31, 0xb2)];
  const checkpoint = Buffer.alloc(40, 0xc3);
  const push = {
    action: "pushBatch",
    deviceId,
    workspaceId,
    batchId: randomUUID(),
    mutations: [1, 2].map((recordKind, index) => ({
      recordId: randomUUID(),
      recordKind,
      expectedServerVersion: 0,
      mutationId: randomUUID(),
      protocolVersion: 1,
      workspaceKeyVersion: 1,
      ciphertext: base64Url(ciphertexts[index]),
      isTombstone: false,
    })),
    checkpointCounter: 1,
    checkpointProtocolVersion: 1,
    checkpointKeyVersion: 1,
    checkpointCiphertext: base64Url(checkpoint),
  };
  const signature = sign(null, pushBatchMessage({ ownerId: userId, sessionId, ...push }), proof.privateKey);
  const pushBody = { ...push, signature: base64Url(signature) };
  const pushed = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: pushBody,
  });
  assert.equal(pushed.response.status, 200, `push: ${pushed.payload.code}`);
  assert.equal(pushed.payload.outcome, "applied");
  assert.equal(pushed.payload.mutationCount, 2);

  const retry = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: pushBody,
  });
  assert.equal(retry.response.status, 200, `retry: ${retry.payload.code}`);
  assert.equal(retry.payload.outcome, "already_applied");

  const altered = { ...push, checkpointCounter: 2, batchId: randomUUID() };
  const alteredSignature = sign(null, pushBatchMessage({ ownerId: userId, sessionId, ...altered }), proof.privateKey);
  const conflicted = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { ...altered, signature: base64Url(alteredSignature) },
  });
  assert.equal(conflicted.response.status, 409, "stale record versions must reject the whole batch");

  const pulled = await jsonRequest("/functions/v1/sync-records", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "pullChanges", deviceId, workspaceId, afterCursor: 0, limit: 100 },
  });
  assert.equal(pulled.response.status, 200, `pull: ${pulled.payload.code}`);
  assert.equal(pulled.payload.changes.length, 2);
  assert.deepEqual(pulled.payload.changes.map((item) => Buffer.from(item.ciphertext, "base64url")), ciphertexts);
  assert.deepEqual(Buffer.from(pulled.payload.checkpoint.ciphertext, "base64url"), checkpoint);

  const missingAuth = await fetch(`${apiUrl}/functions/v1/sync-records`, {
    method: "POST",
    headers: { apikey: publishableKey, "content-type": "application/json" },
    body: JSON.stringify({ action: "pullChanges", deviceId, workspaceId, afterCursor: 0, limit: 100 }),
  });
  assert.equal(missingAuth.status, 401, "missing user bearer token must be refused");

  console.log("Encrypted record operations integration: PASS");
} finally {
  if (userId) {
    await jsonRequest(`/auth/v1/admin/users/${userId}`, {
      apiKey: secretKey,
      bearer: secretKey,
      method: "DELETE",
    });
  }
}
