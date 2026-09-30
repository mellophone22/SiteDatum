export { projectStripeSubscription, verifyStripeSignature } from "./stripe_contract.ts";

export async function stripeRequest(path: string, init: RequestInit = {}): Promise<Record<string, unknown>> {
  const key = Deno.env.get("STRIPE_SECRET_KEY");
  if (!key?.startsWith("sk_test_")) throw new Error("STRIPE_TEST_KEY_REQUIRED");
  const response = await fetch(`https://api.stripe.com/v1${path}`, {
    ...init,
    headers: {
      authorization: `Bearer ${key}`,
      "stripe-version": Deno.env.get("STRIPE_API_VERSION") ?? "2026-02-25.clover",
      ...(init.headers ?? {}),
    },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(`STRIPE_API_${response.status}`);
  return body as Record<string, unknown>;
}
