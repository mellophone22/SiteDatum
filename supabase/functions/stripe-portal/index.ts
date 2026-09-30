import { authenticatedUserId, licensingAdmin } from "../_shared/auth.ts";
import { jsonResponse, requiredEnvironment } from "../_shared/http.ts";
import { stripeRequest } from "../_shared/stripe.ts";

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });
  const admin = licensingAdmin();
  const userId = await authenticatedUserId(request, admin);
  if (!userId) return jsonResponse(401, { code: "AUTHENTICATION_REQUIRED", requestId });
  const { data: subscriptionRef } = await admin.rpc("licensing_stripe_subscription_ref", { p_auth_user_id: userId });
  if (!subscriptionRef) return jsonResponse(404, { code: "SUBSCRIPTION_NOT_FOUND", requestId });
  try {
    const subscription = await stripeRequest(`/subscriptions/${encodeURIComponent(subscriptionRef)}`);
    const params = new URLSearchParams({
      customer: String(subscription.customer),
      return_url: requiredEnvironment("STRIPE_PORTAL_RETURN_URL"),
    });
    const session = await stripeRequest("/billing_portal/sessions", { method: "POST", body: params });
    return jsonResponse(200, { url: session.url, requestId });
  } catch { return jsonResponse(503, { code: "PORTAL_UNAVAILABLE", requestId }); }
});
