import { createClient } from "npm:@supabase/supabase-js@2.117.2";
import {
  buildEntitlementClaims,
  parseIssueEntitlementRequest,
  signEntitlement,
  type LicensedPlan,
  type LicensedSubscriptionStatus,
} from "../_shared/entitlement_contract.ts";
import { jsonResponse, requiredEnvironment, sha256Hex } from "../_shared/http.ts";

type ActivationRow = {
  customer_id: string;
  device_id: string;
  activation_state: "activated" | "reused";
};

type EntitlementRow = {
  subject_id: string;
  plan: LicensedPlan;
  subscription_status: LicensedSubscriptionStatus;
  paid_through_utc: string;
  device_id: string;
  access_kind: "paid" | "complimentary";
};

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") {
    return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });
  }

  const authorization = request.headers.get("authorization");
  if (!authorization?.startsWith("Bearer ")) {
    return jsonResponse(401, { code: "AUTHENTICATION_REQUIRED", requestId });
  }

  const supabaseUrl = requiredEnvironment("SUPABASE_URL");
  const secretKey = requiredEnvironment("SUPABASE_SERVICE_ROLE_KEY");
  const keyId = requiredEnvironment("LICENSING_ENTITLEMENT_KEY_ID");
  const privateKey = requiredEnvironment("LICENSING_ENTITLEMENT_PRIVATE_KEY_PKCS8_B64");
  const admin = createClient(supabaseUrl, secretKey, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  const { data: userData, error: userError } = await admin.auth.getUser(
    authorization.slice("Bearer ".length),
  );
  if (userError || !userData.user) {
    return jsonResponse(401, { code: "AUTHENTICATION_INVALID", requestId });
  }

  let body;
  try {
    body = parseIssueEntitlementRequest(await request.json());
  } catch (error) {
    return jsonResponse(400, {
      code: error instanceof Error ? error.message : "INVALID_REQUEST",
      requestId,
    });
  }

  const subjectId = userData.user.id;
  const rateLimitKey = await sha256Hex(subjectId);
  const { data: allowed, error: rateError } = await admin.rpc("licensing_consume_rate_limit", {
    p_key_hash: rateLimitKey,
    p_action: "entitlement_issue",
    p_limit: 20,
    p_window_seconds: 3600,
  });
  if (rateError) {
    return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
  }
  if (!allowed) {
    return jsonResponse(429, { code: "RATE_LIMITED", requestId });
  }

  const { data: activationData, error: activationError } = await admin.rpc(
    "licensing_activate_device",
    { p_auth_user_id: subjectId, p_fingerprint_hash: body.deviceFingerprintHash },
  );
  if (activationError) {
    const code = activationError.message.includes("LICENSING_DEVICE_LIMIT_REACHED")
      ? "DEVICE_LIMIT_REACHED"
      : activationError.message.includes("LICENSING_PRO_REQUIRED")
        ? "PRO_SUBSCRIPTION_REQUIRED"
        : "LICENSING_UNAVAILABLE";
    return jsonResponse(code === "LICENSING_UNAVAILABLE" ? 503 : 403, { code, requestId });
  }

  const activation = (activationData as ActivationRow[] | null)?.[0];
  if (!activation) {
    return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
  }

  await admin.rpc("licensing_record_audit_event", {
    p_auth_user_id: subjectId,
    p_device_id: activation.device_id,
    p_event_type: activation.activation_state === "activated" ? "device_activated" : "device_reused",
    p_outcome: "succeeded",
    p_reason_code: null,
    p_request_id: requestId,
  });

  const { data: entitlementData, error: entitlementError } = await admin.rpc(
    "licensing_current_entitlement",
    { p_auth_user_id: subjectId, p_device_id: activation.device_id },
  );
  const entitlement = (entitlementData as EntitlementRow[] | null)?.[0];
  if (entitlementError || !entitlement) {
    return jsonResponse(503, { code: "LICENSING_UNAVAILABLE", requestId });
  }

  const issuedAtUtc = Math.floor(Date.now() / 1000);
  const paidThroughUtc = Math.floor(Date.parse(entitlement.paid_through_utc) / 1000);
  try {
    const claims = buildEntitlementClaims({
      keyId,
      subjectId: entitlement.subject_id,
      deviceId: entitlement.device_id,
      plan: entitlement.plan,
      accessKind: entitlement.access_kind,
      subscriptionStatus: entitlement.subscription_status,
      issuedAtUtc,
      paidThroughUtc,
    });
    const token = await signEntitlement(claims, privateKey);
    await admin.rpc("licensing_record_audit_event", {
      p_auth_user_id: subjectId,
      p_device_id: entitlement.device_id,
      p_event_type: "entitlement_issued",
      p_outcome: "succeeded",
      p_reason_code: null,
      p_request_id: requestId,
    });
    return jsonResponse(200, { token, claims, requestId });
  } catch {
    return jsonResponse(503, { code: "ENTITLEMENT_SIGNING_UNAVAILABLE", requestId });
  }
});
