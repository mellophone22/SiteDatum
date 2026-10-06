import { describe, expect, it } from "vitest";
import { validateCommerceConfiguration } from "../supabase/functions/_shared/commercial_environment";

const base = {
  SITEDATUM_COMMERCE_ENVIRONMENT: "sandbox",
  STRIPE_SECRET_KEY: "sk_test_example",
  STRIPE_EXPECTED_ACCOUNT_ID: "acct_123Example",
  STRIPE_API_VERSION: "2026-08-26.dahlia",
};

describe("commercial environment boundary", () => {
  it("accepts an explicitly matched sandbox configuration", () => {
    expect(validateCommerceConfiguration(base)).toMatchObject({
      environment: "sandbox",
      stripeAccountId: "acct_123Example",
    });
  });

  it("accepts an explicitly matched production configuration", () => {
    expect(validateCommerceConfiguration({
      ...base,
      SITEDATUM_COMMERCE_ENVIRONMENT: "production",
      STRIPE_SECRET_KEY: "rk_live_example",
    }).environment).toBe("production");
  });

  it("rejects an unrestricted live key for production", () => {
    expect(() => validateCommerceConfiguration({
      ...base,
      SITEDATUM_COMMERCE_ENVIRONMENT: "production",
      STRIPE_SECRET_KEY: "sk_live_example",
    })).toThrow("STRIPE_KEY_ENVIRONMENT_MISMATCH");
  });

  it.each(["rk_live_example", "sk_live_example"])(
    "rejects live key %s in the sandbox boundary",
    (stripeSecretKey) => {
      expect(() => validateCommerceConfiguration({
        ...base,
        STRIPE_SECRET_KEY: stripeSecretKey,
      })).toThrow("STRIPE_KEY_ENVIRONMENT_MISMATCH");
    },
  );

  it.each([
    [{ ...base, SITEDATUM_COMMERCE_ENVIRONMENT: undefined }, "MISSING_SITEDATUM_COMMERCE_ENVIRONMENT"],
    [{ ...base, SITEDATUM_COMMERCE_ENVIRONMENT: "production" }, "STRIPE_KEY_ENVIRONMENT_MISMATCH"],
    [{ ...base, STRIPE_EXPECTED_ACCOUNT_ID: "not-an-account" }, "STRIPE_ACCOUNT_ID_INVALID"],
    [{ ...base, STRIPE_API_VERSION: "2026-02-25.clover" }, "STRIPE_API_VERSION_UNAPPROVED"],
  ])("rejects unsafe or ambiguous configuration", (values, message) => {
    expect(() => validateCommerceConfiguration(values)).toThrow(message);
  });
});
