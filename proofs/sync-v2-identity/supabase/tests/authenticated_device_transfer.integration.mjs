import { generateKeyPairSync, randomUUID, sign } from "node:crypto";
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import process from "node:process";

const apiUrl = process.env.SYNC_TEST_API_URL;
const publishableKey = process.env.SYNC_TEST_PUBLISHABLE_KEY;
const secretKey = process.env.SYNC_TEST_SECRET_KEY;
if (!apiUrl || !publishableKey || !secretKey) throw new Error("Local Supabase integration-test environment is incomplete.");

const email = `approved-transfer-${randomUUID()}@example.invalid`;
const password = `${randomUUID()}Aa1!`;
let userId;

function base64Url(value) { return Buffer.from(value).toString("base64url"); }
function uuidBytes(value) { return Buffer.from(value.replaceAll("-", ""), "hex"); }
function appendField(parts, value) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(value.length);
  parts.push(length, value);
}
function appendExpiry(parts, value) {
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
  appendExpiry(parts, value.expiresAtUnix);
  return Buffer.concat(parts);
}
function registrationMessage(value) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.transfer-key-registration.v1"));
  for (const id of [value.ownerId, value.sessionId, value.deviceId]) appendField(parts, uuidBytes(id));
  appendField(parts, value.transferPublicKey);
  return Buffer.concat(parts);
}
function targetProofMessage(value) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.approved-target-possession.v1"));
  for (const id of [value.ownerId, value.targetSessionId, value.sourceDeviceId, value.targetDeviceId, value.enrollmentId]) {
    appendField(parts, uuidBytes(id));
  }
  appendField(parts, value.targetProofPublicKey);
  appendField(parts, value.targetTransferPublicKey);
  appendField(parts, value.challenge);
  appendExpiry(parts, value.expiresAtUnix);
  return Buffer.concat(parts);
}
function sourceApprovalMessage(value) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.approved-source-transfer.v1"));
  for (const id of [
    value.ownerId, value.sourceSessionId, value.targetSessionId, value.sourceDeviceId,
    value.targetDeviceId, value.enrollmentId,
  ]) appendField(parts, uuidBytes(id));
  appendField(parts, value.targetProofPublicKey);
  appendField(parts, value.sourceTransferPublicKey);
  appendField(parts, value.targetTransferPublicKey);
  appendField(parts, value.challenge);
  appendField(parts, uuidBytes(value.workspaceId));
  const version = Buffer.alloc(4);
  version.writeUInt32BE(value.workspaceKeyVersion);
  parts.push(version);
  appendExpiry(parts, value.expiresAtUnix);
  appendField(parts, value.encapsulatedKey);
  appendField(parts, value.ciphertext);
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
async function signIn() {
  const result = await jsonRequest("/auth/v1/token?grant_type=password", {
    apiKey: publishableKey,
    bearer: publishableKey,
    body: { email, password },
  });
  assert.equal(result.response.status, 200);
  const claims = JSON.parse(Buffer.from(result.payload.access_token.split(".")[1], "base64url").toString("utf8"));
  return { accessToken: result.payload.access_token, sessionId: claims.session_id };
}

try {
  const created = await jsonRequest("/auth/v1/admin/users", {
    apiKey: secretKey,
    bearer: secretKey,
    body: { email, password, email_confirm: true },
  });
  assert.equal(created.response.status, 200);
  userId = created.payload.id;

  const sourceAuth = await signIn();
  const sourceProof = generateKeyPairSync("ed25519");
  const sourceProofPublic = sourceProof.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const sourceTransfer = generateKeyPairSync("x25519");
  const sourceTransferPublic = sourceTransfer.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const sourceDeviceId = randomUUID();

  const initialBegin = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: { action: "begin", deviceId: sourceDeviceId, publicKey: base64Url(sourceProofPublic) },
  });
  assert.equal(initialBegin.response.status, 200, `initial begin: ${initialBegin.payload.code}`);
  const initialSignature = sign(null, initialProofMessage({
    ownerId: userId,
    sessionId: sourceAuth.sessionId,
    deviceId: sourceDeviceId,
    enrollmentId: initialBegin.payload.enrollmentId,
    publicKey: sourceProofPublic,
    challenge: Buffer.from(initialBegin.payload.challenge, "base64url"),
    expiresAtUnix: initialBegin.payload.expiresAtUnix,
  }), sourceProof.privateKey);
  const initialComplete = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: { action: "complete", enrollmentId: initialBegin.payload.enrollmentId, signature: base64Url(initialSignature) },
  });
  assert.equal(initialComplete.response.status, 200, `initial complete: ${initialComplete.payload.code}`);

  const registerSignature = sign(null, registrationMessage({
    ownerId: userId,
    sessionId: sourceAuth.sessionId,
    deviceId: sourceDeviceId,
    transferPublicKey: sourceTransferPublic,
  }), sourceProof.privateKey);
  const registered = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: {
      action: "registerTransferKey",
      deviceId: sourceDeviceId,
      transferPublicKey: base64Url(sourceTransferPublic),
      signature: base64Url(registerSignature),
    },
  });
  assert.equal(registered.response.status, 200, `register: ${registered.payload.code}`);

  const targetAuth = await signIn();
  const targetProof = generateKeyPairSync("ed25519");
  const targetProofPublic = targetProof.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const targetTransfer = generateKeyPairSync("x25519");
  const targetTransferPublic = targetTransfer.publicKey.export({ type: "spki", format: "der" }).subarray(-32);
  const targetDeviceId = randomUUID();

  const begin = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: targetAuth.accessToken,
    body: {
      action: "beginTarget",
      sourceDeviceId,
      targetDeviceId,
      targetProofPublicKey: base64Url(targetProofPublic),
      targetTransferPublicKey: base64Url(targetTransferPublic),
    },
  });
  assert.equal(begin.response.status, 200, `target begin: ${begin.payload.code}`);
  const targetSignature = sign(null, targetProofMessage({
    ...begin.payload,
    targetProofPublicKey: targetProofPublic,
    targetTransferPublicKey: targetTransferPublic,
    challenge: Buffer.from(begin.payload.challenge, "base64url"),
  }), targetProof.privateKey);
  const proved = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: targetAuth.accessToken,
    body: { action: "proveTarget", enrollmentId: begin.payload.enrollmentId, signature: base64Url(targetSignature) },
  });
  assert.equal(proved.response.status, 200, `target proof: ${proved.payload.code}`);

  const prepared = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: { action: "prepareSource", enrollmentId: begin.payload.enrollmentId, sourceDeviceId },
  });
  assert.equal(prepared.response.status, 200, `source context: ${prepared.payload.code}`);
  assert.equal(prepared.payload.targetSessionId, targetAuth.sessionId);

  const workspaceId = randomUUID();
  const encapsulatedKey = Buffer.alloc(32, 0x55);
  const ciphertext = Buffer.alloc(48, 0x66);
  const approvalValues = {
    ...prepared.payload,
    workspaceId,
    workspaceKeyVersion: 1,
    targetProofPublicKey: Buffer.from(prepared.payload.targetProofPublicKey, "base64url"),
    sourceTransferPublicKey: Buffer.from(prepared.payload.sourceTransferPublicKey, "base64url"),
    targetTransferPublicKey: Buffer.from(prepared.payload.targetTransferPublicKey, "base64url"),
    challenge: Buffer.from(prepared.payload.challenge, "base64url"),
    encapsulatedKey,
    ciphertext,
  };
  const sourceSignature = sign(null, sourceApprovalMessage(approvalValues), sourceProof.privateKey);
  const approvalBody = {
    action: "approveSource",
    enrollmentId: begin.payload.enrollmentId,
    sourceDeviceId,
    workspaceId,
    workspaceKeyVersion: 1,
    encapsulatedKey: base64Url(encapsulatedKey),
    ciphertext: base64Url(ciphertext),
    signature: base64Url(sourceSignature),
  };
  const approved = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: approvalBody,
  });
  assert.equal(approved.response.status, 200, `source approval: ${approved.payload.code}`);
  const retry = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: sourceAuth.accessToken,
    body: approvalBody,
  });
  assert.equal(retry.response.status, 200, "exact approval retry must remain idempotent");

  const fetched = await jsonRequest("/functions/v1/sync-approved-device", {
    apiKey: publishableKey,
    bearer: targetAuth.accessToken,
    body: { action: "fetchTransfer", enrollmentId: begin.payload.enrollmentId, targetDeviceId },
  });
  assert.equal(fetched.response.status, 200, `target fetch: ${fetched.payload.code}`);
  assert.equal(fetched.payload.workspaceId, workspaceId);
  assert.deepEqual(Buffer.from(fetched.payload.encapsulatedKey, "base64url"), encapsulatedKey);
  assert.deepEqual(Buffer.from(fetched.payload.ciphertext, "base64url"), ciphertext);

  const missingAuth = await fetch(`${apiUrl}/functions/v1/sync-approved-device`, {
    method: "POST",
    headers: { apikey: publishableKey, "content-type": "application/json" },
    body: JSON.stringify({ action: "fetchTransfer", enrollmentId: begin.payload.enrollmentId, targetDeviceId }),
  });
  assert.equal(missingAuth.status, 401, "missing user bearer token must be refused");

  console.log("Authenticated approved-device transfer integration: PASS");
} finally {
  if (userId) {
    await jsonRequest(`/auth/v1/admin/users/${userId}`, {
      apiKey: secretKey,
      bearer: secretKey,
      method: "DELETE",
    });
  }
}
