import { describe, expect, it } from "vitest";
import {
  buildCheckoutSessionParams,
  checkoutIntegrationIdentifier,
} from "../supabase/functions/_shared/checkout_session";

describe("Stripe Managed Payments Checkout session", () => {
  it("uses only parameters supported by Managed Payments", () => {
    const params = buildCheckoutSessionParams({
      price: "price_test_monthly",
      correlation: "11111111-1111-4111-8111-111111111111",
      integrationIdentifier: "sitedatum_windows_01020304",
      successUrl: "https://sitedatum.site/?checkout=success",
      cancelUrl: "https://sitedatum.site/?checkout=cancel",
    });

    expect(params.get("mode")).toBe("subscription");
    expect(params.get("managed_payments[enabled]")).toBe("true");
    expect(params.get("integration_identifier")).toBe("sitedatum_windows_01020304");
    // Stripe Managed Payments owns the standardized Checkout surface and rejects custom_text.
    expect(params.has("custom_text[submit][message]")).toBe(false);
    expect(params.toString()).not.toMatch(/projectName|filePath|document|secret/i);
  });

  it("generates a privacy-safe per-session integration identifier", () => {
    expect(checkoutIntegrationIdentifier(new Uint8Array([1, 2, 3, 4])))
      .toBe("sitedatum_windows_01020304");
  });
});
