export { latestSubscriptionInvoiceIsPaid, projectStripeSubscription, verifyStripeSignature } from "./stripe_contract.ts";

import { validateCommerceConfiguration } from "./commercial_environment.ts";

let verifiedStripeAccount: string | null = null;

async function verifiedConfiguration() {
  const configuration = validateCommerceConfiguration({
    SITEDATUM_COMMERCE_ENVIRONMENT: Deno.env.get("SITEDATUM_COMMERCE_ENVIRONMENT"),
    STRIPE_SECRET_KEY: Deno.env.get("STRIPE_SECRET_KEY"),
    STRIPE_EXPECTED_ACCOUNT_ID: Deno.env.get("STRIPE_EXPECTED_ACCOUNT_ID"),
    STRIPE_API_VERSION: Deno.env.get("STRIPE_API_VERSION"),
  });
  if (verifiedStripeAccount === configuration.stripeAccountId) return configuration;

  const response = await fetch("https://api.stripe.com/v1/account", {
    headers: {
      authorization: `Bearer ${configuration.stripeSecretKey}`,
      "stripe-version": configuration.stripeApiVersion,
    },
  });
  const account = await response.json() as Record<string, unknown>;
  if (!response.ok) throw new Error(`STRIPE_ACCOUNT_API_${response.status}`);
  if (account.id !== configuration.stripeAccountId) throw new Error("STRIPE_ACCOUNT_MISMATCH");
  verifiedStripeAccount = configuration.stripeAccountId;
  return configuration;
}

export async function stripeRequest(path: string, init: RequestInit = {}): Promise<Record<string, unknown>> {
  const configuration = await verifiedConfiguration();
  const response = await fetch(`https://api.stripe.com/v1${path}`, {
    ...init,
    headers: {
      authorization: `Bearer ${configuration.stripeSecretKey}`,
      "stripe-version": configuration.stripeApiVersion,
      ...(init.headers ?? {}),
    },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(`STRIPE_API_${response.status}`);
  return body as Record<string, unknown>;
}
