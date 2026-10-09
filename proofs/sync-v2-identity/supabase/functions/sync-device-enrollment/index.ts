import "@supabase/functions-js/edge-runtime.d.ts";
import { withSupabase } from "@supabase/server";
import {
  buildEnrollmentProofMessage,
  decodeBase64Url,
  encodeBase64Url,
  parseEnrollmentRequest,
  verifyEnrollmentProof,
} from "./protocol.ts";

type RpcRow = Record<string, unknown>;
const MAX_REQUEST_BYTES = 4096;

function response(status: number, body: Record<string, unknown>): Response {
  return Response.json(body, { status, headers: { "cache-control": "no-store" } });
}

function firstRow(value: unknown): RpcRow | null {
  return Array.isArray(value) && value.length === 1 && value[0] && typeof value[0] === "object"
    ? value[0] as RpcRow
    : null;
}

function rpcFailureStatus(message: string | undefined): number {
  if (message === "BRIDGE_AUTH_SESSION_INVALID") return 401;
  if (message === "BRIDGE_RATE_LIMITED" || message === "ENROLLMENT_RATE_LIMITED") return 429;
  if (message === "ENROLLMENT_EXISTING_DEVICE_REQUIRES_APPROVAL") return 409;
  return 503;
}

function standardBase64Bytes(value: string): Uint8Array {
  return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
}

async function readBoundedJson(request: Request): Promise<unknown> {
  if (!request.headers.get("content-type")?.toLowerCase().startsWith("application/json")) {
    throw new Error("INVALID_CONTENT_TYPE");
  }
  if (!request.body) throw new Error("INVALID_BODY");

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

export default {
  fetch: withSupabase({ auth: "user" }, async (request, context) => {
    const requestId = crypto.randomUUID();
    if (request.method !== "POST") {
      return response(405, { code: "METHOD_NOT_ALLOWED", requestId });
    }

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
      input = parseEnrollmentRequest(await readBoundedJson(request));
    } catch (error) {
      if (error instanceof Error && error.message === "REQUEST_TOO_LARGE") {
        return response(413, { code: "REQUEST_TOO_LARGE", requestId });
      }
      return response(400, { code: "INVALID_REQUEST", requestId });
    }

    const { data: authorization, error: authorizationError } = await context.supabaseAdmin.rpc(
      "sync_v2_enrollment_bridge_authorize",
      {
        p_owner_id: ownerId,
        p_auth_session_id: sessionId,
        p_action: input.action,
      },
    );
    if (authorizationError) {
      return response(503, { code: "ENROLLMENT_NOT_AVAILABLE", requestId });
    }
    if (authorization === "session_invalid") {
      return response(401, { code: "AUTHENTICATION_INVALID", requestId });
    }
    if (authorization === "rate_limited") {
      return response(429, { code: "RATE_LIMITED", requestId });
    }
    if (authorization !== "authorized") {
      return response(503, { code: "ENROLLMENT_NOT_AVAILABLE", requestId });
    }

    if (input.action === "begin") {
      const { data, error } = await context.supabaseAdmin.rpc("sync_v2_enrollment_bridge_begin", {
        p_owner_id: ownerId,
        p_auth_session_id: sessionId,
        p_requested_device_id: input.deviceId,
        p_proof_public_key_base64: input.publicKeyBase64,
      });
      if (error) {
        return response(rpcFailureStatus(error.message), { code: "ENROLLMENT_NOT_AVAILABLE", requestId });
      }

      const row = firstRow(data);
      if (!row || typeof row.enrollment_id !== "string" || typeof row.challenge_base64 !== "string" ||
          typeof row.expires_at !== "string") {
        return response(503, { code: "ENROLLMENT_NOT_AVAILABLE", requestId });
      }

      return response(200, {
        enrollmentId: row.enrollment_id,
        challenge: encodeBase64Url(standardBase64Bytes(row.challenge_base64)),
        expiresAt: row.expires_at,
        requestId,
      });
    }

    const { data: contextRows, error: contextError } = await context.supabaseAdmin.rpc(
      "sync_v2_enrollment_bridge_context",
      {
        p_enrollment_id: input.enrollmentId,
        p_owner_id: ownerId,
        p_auth_session_id: sessionId,
      },
    );
    if (contextError) {
      return response(rpcFailureStatus(contextError.message), { code: "ENROLLMENT_NOT_ACCEPTED", requestId });
    }

    const enrollment = firstRow(contextRows);
    if (!enrollment || typeof enrollment.requested_device_id !== "string" ||
        typeof enrollment.public_key_base64 !== "string" || typeof enrollment.challenge_base64 !== "string" ||
        typeof enrollment.expires_at !== "string") {
      return response(409, { code: "ENROLLMENT_NOT_ACCEPTED", requestId });
    }

    let proofValid = false;
    try {
      const publicKey = standardBase64Bytes(enrollment.public_key_base64);
      const proofMessage = buildEnrollmentProofMessage({
        ownerId,
        sessionId,
        deviceId: enrollment.requested_device_id,
        enrollmentId: input.enrollmentId,
        publicKey,
        challenge: standardBase64Bytes(enrollment.challenge_base64),
        expiresAt: enrollment.expires_at,
      });
      proofValid = await verifyEnrollmentProof(publicKey, decodeBase64Url(input.signature), proofMessage);
    } catch {
      proofValid = false;
    }

    const { data: completionRows, error: completionError } = await context.supabaseAdmin.rpc(
      "sync_v2_enrollment_bridge_complete",
      {
        p_enrollment_id: input.enrollmentId,
        p_owner_id: ownerId,
        p_auth_session_id: sessionId,
        p_proof_valid: proofValid,
      },
    );
    if (completionError) {
      return response(rpcFailureStatus(completionError.message), { code: "ENROLLMENT_NOT_ACCEPTED", requestId });
    }

    const completion = firstRow(completionRows);
    if (!completion || completion.accepted !== true || typeof completion.device_id !== "string") {
      return response(409, { code: "ENROLLMENT_NOT_ACCEPTED", requestId });
    }

    return response(200, { deviceId: completion.device_id, requestId });
  }),
};
