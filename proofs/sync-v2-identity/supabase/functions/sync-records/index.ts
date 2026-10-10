import "@supabase/functions-js/edge-runtime.d.ts";
import { withSupabase } from "@supabase/server";
import {
  buildPushBatchMessage,
  decodeBase64Url,
  encodeBase64Url,
  parseRecordRequest,
  verifyEd25519,
} from "./protocol.ts";

const MAX_REQUEST_BYTES = 2 * 1024 * 1024;

function response(status: number, body: Record<string, unknown>): Response {
  return Response.json(body, { status, headers: { "cache-control": "no-store" } });
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

function standardBase64(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function standardBase64Bytes(value: unknown): Uint8Array {
  if (typeof value !== "string") throw new Error("INVALID_BASE64");
  return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
}

function resultObject(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
}

function encodePullResult(result: Record<string, unknown>): Record<string, unknown> {
  if (result.outcome !== "available") return result;
  const changes = Array.isArray(result.changes) ? result.changes.map((value) => {
    const item = resultObject(value);
    if (!item) throw new Error("INVALID_RESULT");
    return { ...item, ciphertext: encodeBase64Url(standardBase64Bytes(item.ciphertext)) };
  }) : [];
  const checkpoint = resultObject(result.checkpoint);
  return {
    ...result,
    changes,
    checkpoint: checkpoint
      ? { ...checkpoint, ciphertext: encodeBase64Url(standardBase64Bytes(checkpoint.ciphertext)) }
      : null,
  };
}

function actionName(action: string): string {
  return ({
    createWorkspace: "create_workspace",
    pushBatch: "push_batch",
    pullChanges: "pull_changes",
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
      input = parseRecordRequest(await readBoundedJson(request));
    } catch (error) {
      return response(error instanceof Error && error.message === "REQUEST_TOO_LARGE" ? 413 : 400, {
        code: error instanceof Error && error.message === "REQUEST_TOO_LARGE"
          ? "REQUEST_TOO_LARGE"
          : "INVALID_REQUEST",
        requestId,
      });
    }

    const { data: authorization, error: authorizationError } = await context.supabaseAdmin.rpc(
      "sync_v2_record_bridge_authorize",
      { p_owner_id: ownerId, p_auth_session_id: sessionId, p_action: actionName(input.action) },
    );
    if (authorizationError) return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });
    if (authorization === "session_invalid") return response(401, { code: "AUTHENTICATION_INVALID", requestId });
    if (authorization === "rate_limited") return response(429, { code: "RATE_LIMITED", requestId });
    if (authorization !== "authorized") return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });

    try {
      if (input.action === "createWorkspace") {
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_record_bridge_create_workspace", {
          p_owner_id: ownerId,
          p_auth_session_id: sessionId,
          p_device_id: input.deviceId,
          p_workspace_id: input.workspaceId,
        });
        if (error || (data !== "created" && data !== "already_created")) {
          return response(409, { code: "WORKSPACE_NOT_CREATED", requestId });
        }
        return response(200, { outcome: data, workspaceId: input.workspaceId, requestId });
      }

      if (input.action === "pushBatch") {
        const { data: keyData, error: keyError } = await context.supabaseAdmin.rpc(
          "sync_v2_record_bridge_device_key",
          { p_owner_id: ownerId, p_auth_session_id: sessionId, p_device_id: input.deviceId },
        );
        if (keyError || typeof keyData !== "string") {
          return response(409, { code: "BATCH_NOT_ACCEPTED", requestId });
        }
        const valid = await verifyEd25519(
          standardBase64Bytes(keyData),
          decodeBase64Url(input.signature, 64),
          buildPushBatchMessage(ownerId, sessionId, input),
        );
        if (!valid) return response(409, { code: "BATCH_NOT_ACCEPTED", requestId });

        const mutations = input.mutations.map((mutation) => ({
          ...mutation,
          ciphertext: standardBase64(decodeBase64Url(mutation.ciphertext, 16, 262_144)),
        }));
        const { data, error } = await context.supabaseAdmin.rpc("sync_v2_record_bridge_push_batch", {
          p_owner_id: ownerId,
          p_auth_session_id: sessionId,
          p_device_id: input.deviceId,
          p_workspace_id: input.workspaceId,
          p_batch_id: input.batchId,
          p_mutations: mutations,
          p_checkpoint_counter: input.checkpointCounter,
          p_checkpoint_protocol_version: input.checkpointProtocolVersion,
          p_checkpoint_key_version: input.checkpointKeyVersion,
          p_checkpoint_ciphertext_base64: standardBase64(
            decodeBase64Url(input.checkpointCiphertext, 16, 131_072),
          ),
        });
        const result = resultObject(data);
        if (error || !result || (result.outcome !== "applied" && result.outcome !== "already_applied")) {
          return response(409, { code: "BATCH_NOT_ACCEPTED", requestId });
        }
        return response(200, { ...result, workspaceId: input.workspaceId, batchId: input.batchId, requestId });
      }

      const { data, error } = await context.supabaseAdmin.rpc("sync_v2_record_bridge_pull_changes", {
        p_owner_id: ownerId,
        p_auth_session_id: sessionId,
        p_device_id: input.deviceId,
        p_workspace_id: input.workspaceId,
        p_after_cursor: input.afterCursor,
        p_limit: input.limit,
      });
      const result = resultObject(data);
      if (error || !result || result.outcome !== "available") {
        return response(409, { code: "CHANGES_NOT_AVAILABLE", requestId });
      }
      return response(200, { ...encodePullResult(result), requestId });
    } catch {
      return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });
    }
  }),
};
