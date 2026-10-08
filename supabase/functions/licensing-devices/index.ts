import { createClient } from "npm:@supabase/supabase-js@2.117.2";
import { parseDeviceManagementRequest } from "../_shared/device_contract.ts";
import { jsonResponse, requiredEnvironment, sha256Hex } from "../_shared/http.ts";

type DeviceRow = {
  device_id: string;
  activated_at_utc: string;
  last_seen_at_utc: string;
  is_current: boolean;
};

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });

  const authorization = request.headers.get("authorization");
  if (!authorization?.startsWith("Bearer ")) {
    return jsonResponse(401, { code: "AUTHENTICATION_REQUIRED", requestId });
  }

  const admin = createClient(requiredEnvironment("SUPABASE_URL"), requiredEnvironment("SUPABASE_SERVICE_ROLE_KEY"), {
    auth: { persistSession: false, autoRefreshToken: false },
  });
  const { data: userData, error: userError } = await admin.auth.getUser(authorization.slice("Bearer ".length));
  if (userError || !userData.user) return jsonResponse(401, { code: "AUTHENTICATION_INVALID", requestId });

  let body;
  try {
    body = parseDeviceManagementRequest(await request.json());
  } catch (error) {
    return jsonResponse(400, { code: error instanceof Error ? error.message : "INVALID_REQUEST", requestId });
  }

  const subjectId = userData.user.id;
  const { data: allowed, error: rateError } = await admin.rpc("licensing_consume_rate_limit", {
    p_key_hash: await sha256Hex(subjectId), p_action: "device_manage", p_limit: 30, p_window_seconds: 3600,
  });
  if (rateError) return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
  if (!allowed) return jsonResponse(429, { code: "RATE_LIMITED", requestId });

  if (body.action === "list") {
    const { data, error } = await admin.rpc("licensing_list_devices", {
      p_auth_user_id: subjectId, p_current_fingerprint_hash: body.deviceFingerprintHash,
    });
    if (error) return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
    const devices = ((data ?? []) as DeviceRow[]).map((device) => ({
      deviceId: device.device_id,
      activatedAtUtc: device.activated_at_utc,
      lastSeenAtUtc: device.last_seen_at_utc,
      isCurrent: device.is_current,
    }));
    return jsonResponse(200, { devices, activeDeviceLimit: 2, requestId });
  }

  const { data: changed, error } = await admin.rpc("licensing_deactivate_device", {
    p_auth_user_id: subjectId, p_device_id: body.deviceId,
  });
  if (error) return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
  if (!changed) return jsonResponse(404, { code: "DEVICE_NOT_FOUND", requestId });
  await admin.rpc("licensing_record_audit_event", {
    p_auth_user_id: subjectId,
    p_device_id: body.deviceId,
    p_event_type: "device_deactivated",
    p_outcome: "succeeded",
    p_reason_code: null,
    p_request_id: requestId,
  });
  return jsonResponse(200, { deactivatedDeviceId: body.deviceId, requestId });
});
