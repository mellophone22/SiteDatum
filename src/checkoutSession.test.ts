import { describe, expect, it } from "vitest";
import {
  buildCheckoutSessionParams,
  EARLY_ACCESS_CHECKOUT_DISCLOSURE,
} from "../supabase/functions/_shared/checkout_session";

describe("Stripe Checkout session disclosure", () => {
  it("places the unsigned-installer warning beside the payment confirmation", () => {
    const params = buildCheckoutSessionParams({
      price: "price_test_monthly",
      correlation: "11111111-1111-4111-8111-111111111111",
      successUrl: "https://sitedatum.site/?checkout=success",
      cancelUrl: "https://sitedatum.site/?checkout=cancel",
    });

    expect(params.get("mode")).toBe("subscription");
    expect(params.get("custom_text[submit][message]")).toBe(EARLY_ACCESS_CHECKOUT_DISCLOSURE);
    expect(EARLY_ACCESS_CHECKOUT_DISCLOSURE).toMatch(/unsigned Windows installer/i);
    expect(EARLY_ACCESS_CHECKOUT_DISCLOSURE).toMatch(/Do not disable security protections/i);
    expect(EARLY_ACCESS_CHECKOUT_DISCLOSURE.length).toBeLessThanOrEqual(500);
    expect(params.toString()).not.toMatch(/projectName|filePath|document|secret/i);
  });
});
