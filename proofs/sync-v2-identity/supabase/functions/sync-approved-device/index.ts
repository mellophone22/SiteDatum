import "@supabase/functions-js/edge-runtime.d.ts";
import { withSupabase } from "@supabase/server";
import {
  buildSourceApprovalMessage,
  buildTargetProofMessage,
  buildTransferKeyRegistrationMessage,
  decodeBase64Url,
  encodeBase64Url,
  parseApprovedDeviceRequest,
  verifyEd25519,
} from "./protocol.ts";

type RpcRow = Record<string, unknown>;
const MAX_REQUEST_BYTES = 8 * 1024;

function response(status: number, body: Record<string, unknown>): Response {
  return Response.json(body, { status, headers: { "cache-control": "no-store" } });
}

function firstRow(value: unknown): RpcRow | null {
  return Array.isArray(value) && value.length === 1 && value[0] && typeof value[0] === "object"
    ? value[0] as RpcRow
    : null;
}

function standardBase64(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function standardBase64Bytes(value: unknown, expectedLength: number): Uint8Array {
  if (typeof value !== "string") throw new Error("INVALID_BASE64");
  const bytes = Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
  if (bytes.length !== expectedLength) throw new Error("INVALID_LENGTH");
  return bytes;
}

function expiresAtUnix(value: unknown): number {
  if (typeof value !== "string") throw new Error("INVALID_EXPIRY");
  const result = Math.floor(Date.parse(value) / 1000);
  if (!Number.isSafeInteger(result) || result < 0) throw new Error("INVALID_EXPIRY");
  return result;
}

async function readBoundedJson(request: Request): Promise<unknown> {
  if (!request.headers.get("content-type")?.toLowerCase().startsWith("application/json") || !request.body) {
    throw new Error("INVALID_BODY");
  }
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > MAX_REQUEST_BYTES) {
      await reader.cancel();
      throw new Error("REQUEST_TOO_LARGE");
    }
    chunks.push(value);
  }
  const body = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(body));
}

function actionName(action: string): string {
  return ({
    registerTransferKey: "register_transfer_key",
    beginTarget: "begin_target",
    proveTarget: "prove_target",
    prepareSource: "prepare_source",
    approveSource: "approve_source",
    fetchTransfer: "fetch_transfer",
  } as Record<string, string>)[action] ?? "invalid";
}

export default {
  fetch: withSupabase({ auth: "user" }, async (request, context) => {
    const requestId = crypto.randomUUID();
    if (request.method !== "POST") return response(405, { code: "METHOD_NOT_ALLOWED", requestId });
    const contentLength = Number(request.headers.get("content-length") ?? "0");
    if (!Number.isFinite(contentLength) || contentLength > MAX_REQUEST_BYTES) {
      return response(413, { code: "REQUEST_TOO_LARGE", requestId });
    }

    const ownerId = context.userClaims?.id;
    const sessionId = context.jwtClaims?.session_id;
    if (typeof ownerId !== "string" || typeof sessionId !== "string") {
      return response(401, { code: "AUTHENTICATION_INVALID", requestId });
    }

    let input;
    try {
      input = parseApprovedDeviceRequest(await readBoundedJson(request));
    } catch (error) {
      return response(error instanceof Error && error.message === "REQUEST_TOO_LARGE" ? 413 : 400, {
        code: error instanceof Error && error.message === "REQUEST_TOO_LARGE"
          ? "REQUEST_TOO_LARGE"
          : "INVALID_REQUEST",
        requestId,
      });
    }

    const { data: authorization, error: authorizationError } = await context.supabaseAdmin.rpc(
      "sync_v2_approved_bridge_authorize",
      { p_owner_id: ownerId, p_auth_session_id: sessionId, p_action: actionName(input.action) },
    );
    if (authorizationError) return response(503, { code: "TRANSFER_NOT_AVAILABLE", requestId });
    if (authorization === "session_invalid") return response(401, { code: "AUTHENTICATION_INVALID", requestId });
    if (authorization === "rate_limited") return response(429, { code: "RATE_LIMITED", requestId });
    if (authorization !== "authorized") return response(503, { code: "TRANSFER_NOT_AVAILABLE", requestId });

    try {
      if (input.action === "registerTransferKey") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_device_context", {
          p_owner_id: ownerId,
          p_auth_session_id: sessionId,
          p_device_id: input.deviceId,
        });
        const device = firstRow(data);
        if (error || !device) return response(409, { code: "TRANSFER_KEY_NOT_REGISTERED", requestId });
        const proofPublicKey = standardBase64Bytes(device.proof_public_key_base64, 32);
        const transferPublicKey = decodeBase64Url(input.transferPublicKey, 32);
        const valid = await verifyEd25519(
          proofPublicKey,
          decodeBase64Url(input.signature, 64),
          buildTransferKeyRegistrationMessage(ownerId, sessionId, input.deviceId, transferPublicKey),
        );
        const { data: outcome, error: registerError } = await context.supabaseAdmin.rpc(
          "sync_v2_approved_bridge_register_transfer_key",
          {
            p_owner_id: ownerId,
            p_auth_session_id: sessionId,
            p_device_id: input.deviceId,
            p_transfer_public_key_base64: standardBase64(transferPublicKey),
            p_proof_valid: valid,
          },
        );
        if (registerError || (outcome !== "registered" && outcome !== "already_registered")) {
          return response(409, { code: "TRANSFER_KEY_NOT_REGISTERED", requestId });
        }
        return response(200, { deviceId: input.deviceId, outcome, requestId });
      }

      if (input.action === "beginTarget") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_begin_target", {
          p_owner_id: ownerId,
          p_target_auth_session_id: sessionId,
          p_source_device_id: input.sourceDeviceId,
          p_target_device_id: input.targetDeviceId,
          p_target_proof_public_key_base64: standardBase64(decodeBase64Url(input.targetProofPublicKey, 32)),
          p_target_transfer_public_key_base64: standardBase64(decodeBase64Url(input.targetTransferPublicKey, 32)),
        });
        const row = firstRow(data);
        if (error || !row || typeof row.enrollment_id !== "string") {
          return response(409, { code: "TARGET_ENROLLMENT_NOT_STARTED", requestId });
        }
        return response(200, {
          ownerId,
          targetSessionId: sessionId,
          sourceDeviceId: input.sourceDeviceId,
          targetDeviceId: input.targetDeviceId,
          enrollmentId: row.enrollment_id,
          targetProofPublicKey: input.targetProofPublicKey,
          targetTransferPublicKey: input.targetTransferPublicKey,
          challenge: encodeBase64Url(standardBase64Bytes(row.challenge_base64, 32)),
          expiresAtUnix: expiresAtUnix(row.expires_at),
          requestId,
        });
      }

      if (input.action === "proveTarget") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_target_context", {
          p_enrollment_id: input.enrollmentId,
          p_owner_id: ownerId,
          p_target_auth_session_id: sessionId,
        });
        const row = firstRow(data);
        if (error || !row || typeof row.source_device_id !== "string" || typeof row.target_device_id !== "string" ||
            typeof row.expires_at !== "string") {
          return response(409, { code: "TARGET_NOT_VERIFIED", requestId });
        }
        const targetProofPublicKey = standardBase64Bytes(row.target_proof_public_key_base64, 32);
        const valid = await verifyEd25519(
          targetProofPublicKey,
          decodeBase64Url(input.signature, 64),
          buildTargetProofMessage({
            ownerId,
            targetSessionId: sessionId,
            sourceDeviceId: row.source_device_id,
            targetDeviceId: row.target_device_id,
            enrollmentId: input.enrollmentId,
            targetProofPublicKey,
            targetTransferPublicKey: standardBase64Bytes(row.target_transfer_public_key_base64, 32),
            challenge: standardBase64Bytes(row.challenge_base64, 32),
            expiresAt: row.expires_at,
          }),
        );
        const { data: verification, error: verifyError } = await context.supabaseAdmin.rpc(
          "sync_v2_approved_bridge_verify_target",
          {
            p_enrollment_id: input.enrollmentId,
            p_owner_id: ownerId,
            p_target_auth_session_id: sessionId,
            p_proof_valid: valid,
          },
        );
        const result = firstRow(verification);
        if (verifyError || !result || result.ready_for_approval !== true) {
          return response(409, { code: "TARGET_NOT_VERIFIED", requestId });
        }
        return response(200, { enrollmentId: input.enrollmentId, outcome: result.outcome, requestId });
      }

      if (input.action === "prepareSource") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_source_context", {
          p_enrollment_id: input.enrollmentId,
          p_owner_id: ownerId,
          p_source_auth_session_id: sessionId,
          p_source_device_id: input.sourceDeviceId,
        });
        const row = firstRow(data);
        if (error || !row || typeof row.target_auth_session_id !== "string" ||
            typeof row.target_device_id !== "string" || typeof row.expires_at !== "string") {
          return response(409, { code: "SOURCE_CONTEXT_NOT_AVAILABLE", requestId });
        }
        return response(200, {
          ownerId,
          sourceSessionId: sessionId,
          targetSessionId: row.target_auth_session_id,
          sourceDeviceId: input.sourceDeviceId,
          targetDeviceId: row.target_device_id,
          enrollmentId: input.enrollmentId,
          targetProofPublicKey: encodeBase64Url(standardBase64Bytes(row.target_proof_public_key_base64, 32)),
          sourceTransferPublicKey: encodeBase64Url(standardBase64Bytes(row.source_transfer_public_key_base64, 32)),
          targetTransferPublicKey: encodeBase64Url(standardBase64Bytes(row.target_transfer_public_key_base64, 32)),
          challenge: encodeBase64Url(standardBase64Bytes(row.challenge_base64, 32)),
          expiresAtUnix: expiresAtUnix(row.expires_at),
          outcome: row.outcome,
          requestId,
        });
      }

      if (input.action === "approveSource") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_source_context", {
          p_enrollment_id: input.enrollmentId,
          p_owner_id: ownerId,
          p_source_auth_session_id: sessionId,
          p_source_device_id: input.sourceDeviceId,
        });
        const row = firstRow(data);
        if (error || !row || typeof row.target_auth_session_id !== "string" ||
            typeof row.target_device_id !== "string" || typeof row.expires_at !== "string") {
          return response(409, { code: "TRANSFER_NOT_ACCEPTED", requestId });
        }
        const contextValue = {
          ownerId,
          sourceSessionId: sessionId,
          targetSessionId: row.target_auth_session_id,
          sourceDeviceId: input.sourceDeviceId,
          targetDeviceId: row.target_device_id,
          enrollmentId: input.enrollmentId,
          targetProofPublicKey: standardBase64Bytes(row.target_proof_public_key_base64, 32),
          sourceTransferPublicKey: standardBase64Bytes(row.source_transfer_public_key_base64, 32),
          targetTransferPublicKey: standardBase64Bytes(row.target_transfer_public_key_base64, 32),
          challenge: standardBase64Bytes(row.challenge_base64, 32),
          expiresAt: row.expires_at,
          workspaceId: input.workspaceId,
          workspaceKeyVersion: input.workspaceKeyVersion,
          encapsulatedKey: decodeBase64Url(input.encapsulatedKey, 32),
          ciphertext: decodeBase64Url(input.ciphertext, 48),
        };
        const { data: deviceData, error: deviceError } = await context.supabaseAdmin.rpc(
          "sync_v2_approved_bridge_device_context",
          { p_owner_id: ownerId, p_auth_session_id: sessionId, p_device_id: input.sourceDeviceId },
        );
        const sourceDevice = firstRow(deviceData);
        if (deviceError || !sourceDevice) return response(409, { code: "TRANSFER_NOT_ACCEPTED", requestId });
        const valid = await verifyEd25519(
          standardBase64Bytes(sourceDevice.proof_public_key_base64, 32),
          decodeBase64Url(input.signature, 64),
          buildSourceApprovalMessage(contextValue),
        );
        const { data: completionData, error: completionError } = await context.supabaseAdmin.rpc(
          "sync_v2_approved_bridge_accept_transfer",
          {
            p_enrollment_id: input.enrollmentId,
            p_owner_id: ownerId,
            p_source_auth_session_id: sessionId,
            p_source_device_id: input.sourceDeviceId,
            p_workspace_id: input.workspaceId,
            p_workspace_key_version: input.workspaceKeyVersion,
            p_encapsulated_key_base64: standardBase64(contextValue.encapsulatedKey),
            p_ciphertext_base64: standardBase64(contextValue.ciphertext),
            p_approval_valid: valid,
          },
        );
        const completion = firstRow(completionData);
        if (completionError || !completion || completion.accepted !== true || typeof completion.device_id !== "string") {
          return response(409, { code: "TRANSFER_NOT_ACCEPTED", requestId });
        }
        return response(200, {
          enrollmentId: input.enrollmentId,
          targetDeviceId: completion.device_id,
          outcome: completion.outcome,
          requestId,
        });
      }

      const { data, error } = await context.supabaseAdmin.rpc("sync_v2_approved_bridge_fetch_transfer", {
        p_enrollment_id: input.enrollmentId,
        p_owner_id: ownerId,
        p_target_auth_session_id: sessionId,
        p_target_device_id: input.targetDeviceId,
      });
      const row = firstRow(data);
      if (error || !row || typeof row.source_device_id !== "string" || typeof row.target_device_id !== "string" ||
          typeof row.workspace_id !== "string" || typeof row.workspace_key_version !== "number" ||
          typeof row.expires_at !== "string") {
        return response(409, { code: "TRANSFER_NOT_AVAILABLE", requestId });
      }
      return response(200, {
        ownerId,
        sourceDeviceId: row.source_device_id,
        targetDeviceId: row.target_device_id,
        enrollmentId: input.enrollmentId,
        workspaceId: row.workspace_id,
        workspaceKeyVersion: row.workspace_key_version,
        sourceTransferPublicKey: encodeBase64Url(standardBase64Bytes(row.source_transfer_public_key_base64, 32)),
        targetTransferPublicKey: encodeBase64Url(standardBase64Bytes(row.target_transfer_public_key_base64, 32)),
        encapsulatedKey: encodeBase64Url(standardBase64Bytes(row.encapsulated_key_base64, 32)),
        ciphertext: encodeBase64Url(standardBase64Bytes(row.ciphertext_base64, 48)),
        expiresAtUnix: expiresAtUnix(row.expires_at),
        requestId,
      });
    } catch {
      return response(503, { code: "TRANSFER_NOT_AVAILABLE", requestId });
    }
  }),
};
