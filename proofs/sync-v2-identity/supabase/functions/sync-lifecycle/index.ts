import "@supabase/functions-js/edge-runtime.d.ts";
import { withSupabase } from "@supabase/server";
import { buildLifecycleMessage, decodeBase64Url, parseLifecycleRequest, verifyEd25519 } from "./protocol.ts";

const MAX_REQUEST_BYTES = 4096;

function response(status: number, body: Record<string, unknown>): Response {
  return Response.json(body, { status, headers: { "cache-control": "no-store" } });
}

function standardBase64Bytes(value: unknown): Uint8Array {
  if (typeof value !== "string") throw new Error("INVALID_KEY");
  return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
}

function actionName(action: string): string {
  return ({
    disableWorkspace: "disable_workspace",
    restoreWorkspace: "restore_workspace",
    deleteWorkspace: "delete_workspace",
    deleteAccountSyncData: "delete_account_sync_data",
  } as Record<string, string>)[action] ?? "invalid";
}

export default {
  fetch: withSupabase({ auth: "user" }, async (request, context) => {
    const requestId = crypto.randomUUID();
    if (request.method !== "POST") return response(405, { code: "METHOD_NOT_ALLOWED", requestId });
    if (!request.headers.get("content-type")?.toLowerCase().startsWith("application/json")) {
      return response(400, { code: "INVALID_REQUEST", requestId });
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
      const body = await request.text();
      if (new TextEncoder().encode(body).length > MAX_REQUEST_BYTES) {
        return response(413, { code: "REQUEST_TOO_LARGE", requestId });
      }
      input = parseLifecycleRequest(JSON.parse(body));
    } catch {
      return response(400, { code: "INVALID_REQUEST", requestId });
    }

    const action = actionName(input.action);
    const { data: authorization, error: authorizationError } = await context.supabaseAdmin.rpc(
      "sync_v2_lifecycle_bridge_authorize",
      { p_owner_id: ownerId, p_auth_session_id: sessionId, p_action: action },
    );
    if (authorizationError) return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });
    if (authorization === "session_invalid") return response(401, { code: "AUTHENTICATION_INVALID", requestId });
    if (authorization === "rate_limited") return response(429, { code: "RATE_LIMITED", requestId });
    if (authorization !== "authorized") return response(400, { code: "INVALID_REQUEST", requestId });

    const { data: publicKey, error: keyError } = await context.supabaseAdmin.rpc(
      "sync_v2_record_bridge_device_key",
      { p_owner_id: ownerId, p_auth_session_id: sessionId, p_device_id: input.deviceId },
    );
    if (keyError || typeof publicKey !== "string") {
      return response(409, { code: "LIFECYCLE_NOT_APPLIED", requestId });
    }
    const signatureValid = await verifyEd25519(
      standardBase64Bytes(publicKey), decodeBase64Url(input.signature, 64),
      buildLifecycleMessage(ownerId, sessionId, input),
    );
    if (!signatureValid) return response(409, { code: "LIFECYCLE_NOT_APPLIED", requestId });

    try {
      const common = { p_owner_id: ownerId, p_auth_session_id: sessionId, p_device_id: input.deviceId };
      let rpc: string;
      let args: Record<string, unknown>;
      if (input.action === "disableWorkspace" || input.action === "restoreWorkspace") {
        rpc = input.action === "disableWorkspace"
          ? "sync_v2_lifecycle_bridge_disable_workspace"
          : "sync_v2_lifecycle_bridge_restore_workspace";
        args = { ...common, p_workspace_id: input.workspaceId };
      } else if (input.action === "deleteWorkspace") {
        rpc = "sync_v2_lifecycle_bridge_delete_workspace";
        args = { ...common, p_workspace_id: input.workspaceId, p_request_id: input.requestId };
      } else {
        rpc = "sync_v2_lifecycle_bridge_delete_account_sync_data";
        args = { ...common, p_request_id: input.requestId };
      }
      const { data, error } = await context.supabaseAdmin.rpc(rpc, args);
      if (error) return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });
      const outcome = typeof data === "string" ? data : data?.outcome;
      const accepted = new Set(["disabled", "already_disabled", "restored", "already_active", "deleted", "already_deleted"]);
      if (typeof outcome !== "string" || !accepted.has(outcome)) {
        return response(409, { code: "LIFECYCLE_NOT_APPLIED", requestId });
      }
      const result = typeof data === "string" ? { outcome: data } : data;
      return response(200, { ...result, requestId });
    } catch {
      return response(503, { code: "SYNC_NOT_AVAILABLE", requestId });
    }
  }),
};
