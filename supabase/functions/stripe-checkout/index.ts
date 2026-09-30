import { authenticatedUserId, licensingAdmin } from "../_shared/auth.ts";
import { jsonResponse, requiredEnvironment } from "../_shared/http.ts";
import { stripeRequest } from "../_shared/stripe.ts";

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });
  const admin = licensingAdmin();
  const userId = await authenticatedUserId(request, admin);
  if (!userId) return jsonResponse(401, { code: "AUTHENTICATION_REQUIRED", requestId });
  let plan: "pro_monthly" | "pro_annual";
  try {
    const body = await request.json();
    if (body?.plan !== "pro_monthly" && body?.plan !== "pro_annual") throw new Error();
    plan = body.plan;
  } catch { return jsonResponse(400, { code: "PLAN_INVALID", requestId }); }

  const { data: correlation, error } = await admin.rpc("licensing_create_checkout_correlation", {
    p_auth_user_id: userId, p_plan: plan,
  });
  if (error || !correlation) return jsonResponse(503, { code: "CHECKOUT_UNAVAILABLE", requestId });
  const price = requiredEnvironment(plan === "pro_monthly" ? "STRIPE_MONTHLY_PRICE_ID" : "STRIPE_ANNUAL_PRICE_ID");
  const params = new URLSearchParams({
    mode: "subscription",
    "line_items[0][price]": price,
    "line_items[0][quantity]": "1",
    client_reference_id: correlation,
    "subscription_data[metadata][sitedatum_correlation_id]": correlation,
    "managed_payments[enabled]": "true",
    success_url: requiredEnvironment("STRIPE_CHECKOUT_SUCCESS_URL"),
    cancel_url: requiredEnvironment("STRIPE_CHECKOUT_CANCEL_URL"),
  });
  try {
    const session = await stripeRequest("/checkout/sessions", { method: "POST", body: params });
    return jsonResponse(200, { url: session.url, requestId });
  } catch { return jsonResponse(503, { code: "CHECKOUT_UNAVAILABLE", requestId }); }
});
