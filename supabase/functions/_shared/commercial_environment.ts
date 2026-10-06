export type CommerceEnvironment = "sandbox" | "production";

export type CommerceConfiguration = {
  environment: CommerceEnvironment;
  stripeSecretKey: string;
  stripeAccountId: string;
  stripeApiVersion: string;
};

const CURRENT_STRIPE_API_VERSION = "2026-08-26.dahlia";

function required(values: Record<string, string | undefined>, name: string): string {
  const value = values[name]?.trim();
  if (!value) throw new Error(`MISSING_${name}`);
  return value;
}

export function validateCommerceConfiguration(
  values: Record<string, string | undefined>,
): CommerceConfiguration {
  const environment = required(values, "SITEDATUM_COMMERCE_ENVIRONMENT");
  if (environment !== "sandbox" && environment !== "production") {
    throw new Error("COMMERCE_ENVIRONMENT_INVALID");
  }

  const stripeSecretKey = required(values, "STRIPE_SECRET_KEY");
  const expectedPrefix = environment === "production" ? "rk_live_" : "sk_test_";
  if (!stripeSecretKey.startsWith(expectedPrefix)) {
    throw new Error("STRIPE_KEY_ENVIRONMENT_MISMATCH");
  }

  const stripeAccountId = required(values, "STRIPE_EXPECTED_ACCOUNT_ID");
  if (!/^acct_[A-Za-z0-9]+$/.test(stripeAccountId)) {
    throw new Error("STRIPE_ACCOUNT_ID_INVALID");
  }

  const stripeApiVersion = required(values, "STRIPE_API_VERSION");
  if (stripeApiVersion !== CURRENT_STRIPE_API_VERSION) {
    throw new Error("STRIPE_API_VERSION_UNAPPROVED");
  }

  return {
    environment,
    stripeSecretKey,
    stripeAccountId,
    stripeApiVersion,
  };
}

