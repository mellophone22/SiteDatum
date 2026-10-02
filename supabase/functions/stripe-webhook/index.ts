import { licensingAdmin } from "../_shared/auth.ts";
import {
  jsonResponse,
  requiredEnvironment,
} from "../_shared/http.ts";
import { PayloadTooLargeError, readTextBodyLimited } from "../_shared/request_body.ts";
import { sha256Hex } from "../_shared/sha256.ts";
import { projectStripeSubscription, stripeRequest, verifyStripeSignature } from "../_shared/stripe.ts";

type StripeEvent = { id: string; type: string; created: number; data: { object: Record<string, unknown> } };
const MAXIMUM_WEBHOOK_BYTES = 1024 * 1024;

function subscriptionRef(event: StripeEvent): string | null {
  const object = event.data.object;
  if (event.type.startsWith("customer.subscription.")) return String(object.id ?? "") || null;
  if (event.type === "checkout.session.completed") return typeof object.subscription === "string" ? object.subscription : null;
  if (event.type.startsWith("invoice.")) {
    if (typeof object.subscription === "string") return object.subscription;
    const parent = object.parent as { subscription_details?: { subscription?: string } } | undefined;
    return parent?.subscription_details?.subscription ?? null;
  }
  return null;
}

Deno.serve(async (request) => {
  const requestId = crypto.randomUUID();
  if (request.method !== "POST") return jsonResponse(405, { code: "METHOD_NOT_ALLOWED", requestId });
  let rawBody: string;
  try {
    rawBody = await readTextBodyLimited(request, MAXIMUM_WEBHOOK_BYTES);
  } catch (error) {
    return error instanceof PayloadTooLargeError
      ? jsonResponse(413, { code: "PAYLOAD_TOO_LARGE", requestId })
      : jsonResponse(400, { code: "PAYLOAD_INVALID", requestId });
  }
  try {
    await verifyStripeSignature(rawBody, request.headers.get("stripe-signature") ?? "", requiredEnvironment("STRIPE_WEBHOOK_SECRET"));
  } catch { return jsonResponse(400, { code: "SIGNATURE_INVALID", requestId }); }

  let event: StripeEvent;
  try { event = JSON.parse(rawBody); } catch { return jsonResponse(400, { code: "PAYLOAD_INVALID", requestId }); }
  const supported = event.type === "checkout.session.completed"
    || event.type.startsWith("customer.subscription.")
    || ["invoice.paid", "invoice.payment_failed", "invoice.payment_action_required"].includes(event.type);
  if (!supported) return jsonResponse(200, { received: true, ignored: true, requestId });
  const ref = subscriptionRef(event);
  if (!event.id || !ref || !Number.isSafeInteger(event.created)) {
    return jsonResponse(400, { code: "PAYLOAD_INVALID", requestId });
  }

  try {
    // Re-read authoritative current state so delayed event delivery cannot regress access.
    const subscription = await stripeRequest(`/subscriptions/${encodeURIComponent(ref)}`);
    const projection = projectStripeSubscription(
      subscription as never,
      requiredEnvironment("STRIPE_MONTHLY_PRICE_ID"),
      requiredEnvironment("STRIPE_ANNUAL_PRICE_ID"),
    );
    const admin = licensingAdmin();
    const { data, error } = await admin.rpc("licensing_apply_stripe_subscription_event", {
      p_event_ref: event.id,
      p_event_type: event.type,
      p_payload_sha256: await sha256Hex(rawBody),
      p_provider_created_at_utc: new Date(event.created * 1000).toISOString(),
      p_subscription_ref: projection.subscriptionRef,
      p_customer_ref: projection.customerRef,
      p_correlation_id: projection.correlationId,
      p_plan: projection.plan,
      p_status: projection.status,
      p_paid_through_utc: projection.paidThroughUtc,
      p_provider_updated_at_utc: new Date().toISOString(),
    });
    if (error) throw error;
    return jsonResponse(200, { received: true, result: data, requestId });
  } catch { return jsonResponse(503, { code: "WEBHOOK_PROCESSING_FAILED", requestId }); }
});
