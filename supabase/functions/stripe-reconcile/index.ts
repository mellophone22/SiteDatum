import { licensingAdmin } from "../_shared/auth.ts";
import { jsonResponse, requiredEnvironment, sha256Hex } from "../_shared/http.ts";
import { projectStripeSubscription, stripeRequest } from "../_shared/stripe.ts";

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });
  if (request.headers.get("authorization") !== `Bearer ${requiredEnvironment("LICENSING_RECONCILIATION_SECRET")}`) {
    return jsonResponse(401, { code: "AUTHENTICATION_REQUIRED", requestId });
  }
  const admin = licensingAdmin();
  const { data: rows, error } = await admin.rpc("licensing_stripe_subscription_refs");
  if (error) return jsonResponse(503, { code: "RECONCILIATION_UNAVAILABLE", requestId });
  let reconciled = 0;
  for (const row of rows ?? []) {
    try {
      const subscription = await stripeRequest(`/subscriptions/${encodeURIComponent(row.subscription_ref)}`);
      const projection = projectStripeSubscription(subscription as never, requiredEnvironment("STRIPE_MONTHLY_PRICE_ID"), requiredEnvironment("STRIPE_ANNUAL_PRICE_ID"));
      const snapshot = JSON.stringify(subscription);
      const now = new Date().toISOString();
      const { error: applyError } = await admin.rpc("licensing_apply_stripe_subscription_event", {
        p_event_ref: `reconcile_${requestId}_${row.subscription_ref}`,
        p_event_type: "reconciliation",
        p_payload_sha256: await sha256Hex(snapshot),
        p_provider_created_at_utc: now,
        p_subscription_ref: projection.subscriptionRef,
        p_customer_ref: projection.customerRef,
        p_correlation_id: null,
        p_plan: projection.plan,
        p_status: projection.status,
        p_paid_through_utc: projection.paidThroughUtc,
        p_provider_updated_at_utc: now,
      });
      if (!applyError) reconciled++;
    } catch { /* Continue; the count makes partial failure visible to operations. */ }
  }
  return jsonResponse(200, { reconciled, attempted: rows?.length ?? 0, requestId });
});
