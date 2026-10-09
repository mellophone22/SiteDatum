import { generateKeyPairSync, randomUUID, sign } from "node:crypto";
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import process from "node:process";

const apiUrl = process.env.SYNC_TEST_API_URL;
const publishableKey = process.env.SYNC_TEST_PUBLISHABLE_KEY;
const secretKey = process.env.SYNC_TEST_SECRET_KEY;

if (!apiUrl || !publishableKey || !secretKey) {
  throw new Error("Local Supabase integration-test environment is incomplete.");
}

const email = `sync-bridge-${randomUUID()}@example.invalid`;
const password = `${randomUUID()}Aa1!`;
let userId;

function base64Url(value) {
  return Buffer.from(value).toString("base64url");
}

function uuidBytes(value) {
  return Buffer.from(value.replaceAll("-", ""), "hex");
}

function appendField(parts, field) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(field.length);
  parts.push(length, field);
}

function proofMessage({ ownerId, sessionId, deviceId, enrollmentId, publicKey, challenge, expiresAt }) {
  const parts = [];
  appendField(parts, Buffer.from("sitedatum.sync-v2.initial-device-possession.v1", "utf8"));
  appendField(parts, uuidBytes(ownerId));
  appendField(parts, uuidBytes(sessionId));
  appendField(parts, uuidBytes(deviceId));
  appendField(parts, uuidBytes(enrollmentId));
  appendField(parts, publicKey);
  appendField(parts, challenge);
  const expiry = Buffer.alloc(8);
  expiry.writeBigUInt64BE(BigInt(Math.floor(Date.parse(expiresAt) / 1000)));
  parts.push(expiry);
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
  assert.equal(created.response.status, 200, "fixture user creation must succeed");
  assert.equal(typeof created.payload.id, "string");
  userId = created.payload.id;

  const signedIn = await jsonRequest("/auth/v1/token?grant_type=password", {
    apiKey: publishableKey,
    bearer: publishableKey,
    body: { email, password },
  });
  assert.equal(signedIn.response.status, 200, "fixture user sign-in must succeed");
  const accessToken = signedIn.payload.access_token;
  assert.equal(typeof accessToken, "string");
  const jwtPayload = JSON.parse(Buffer.from(accessToken.split(".")[1], "base64url").toString("utf8"));
  assert.equal(jwtPayload.sub, userId);
  assert.equal(typeof jwtPayload.session_id, "string");

  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const spki = publicKey.export({ type: "spki", format: "der" });
  const rawPublicKey = spki.subarray(spki.length - 32);
  const deviceId = randomUUID();

  const begin = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "begin", deviceId, publicKey: base64Url(rawPublicKey) },
  });
  assert.equal(begin.response.status, 200, `begin failed with ${begin.payload.code ?? "unknown response"}`);
  assert.equal(typeof begin.payload.enrollmentId, "string");
  assert.equal(typeof begin.payload.challenge, "string");
  assert.equal(typeof begin.payload.expiresAt, "string");

  const message = proofMessage({
    ownerId: userId,
    sessionId: jwtPayload.session_id,
    deviceId,
    enrollmentId: begin.payload.enrollmentId,
    publicKey: rawPublicKey,
    challenge: Buffer.from(begin.payload.challenge, "base64url"),
    expiresAt: begin.payload.expiresAt,
  });
  const signature = sign(null, message, privateKey);

  const completeBody = {
    action: "complete",
    enrollmentId: begin.payload.enrollmentId,
    signature: base64Url(signature),
  };
  const completed = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: completeBody,
  });
  assert.equal(completed.response.status, 200, `complete failed with ${completed.payload.code ?? "unknown response"}`);
  assert.equal(completed.payload.deviceId, deviceId);

  const replay = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: completeBody,
  });
  assert.equal(replay.response.status, 409, "consumed proof must not be replayable");
  assert.equal(replay.payload.code, "ENROLLMENT_NOT_ACCEPTED");

  const secondDevice = await jsonRequest("/functions/v1/sync-device-enrollment", {
    apiKey: publishableKey,
    bearer: accessToken,
    body: { action: "begin", deviceId: randomUUID(), publicKey: base64Url(rawPublicKey) },
  });
  assert.equal(secondDevice.response.status, 409, "additional-device enrollment must require approval");

  const unauthenticated = await fetch(`${apiUrl}/functions/v1/sync-device-enrollment`, {
    method: "POST",
    headers: { apikey: publishableKey, "content-type": "application/json" },
    body: JSON.stringify({ action: "begin", deviceId: randomUUID(), publicKey: base64Url(rawPublicKey) }),
  });
  assert.equal(unauthenticated.status, 401, "missing user bearer token must be refused");

  const oversized = await fetch(`${apiUrl}/functions/v1/sync-device-enrollment`, {
    method: "POST",
    headers: {
      apikey: publishableKey,
      authorization: `Bearer ${accessToken}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({ action: "begin", padding: "x".repeat(5000) }),
  });
  assert.equal(oversized.status, 413, "oversized request bodies must be refused");

  console.log("Trusted enrollment bridge integration: PASS");
} finally {
  if (userId) {
    await jsonRequest(`/auth/v1/admin/users/${userId}`, {
      apiKey: secretKey,
      bearer: secretKey,
      method: "DELETE",
    });
  }
}
