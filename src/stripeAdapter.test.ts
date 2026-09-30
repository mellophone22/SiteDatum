import { describe, expect, it } from "vitest";
import { projectStripeSubscription, verifyStripeSignature } from "../supabase/functions/_shared/stripe_contract";

async function sign(body: string, secret: string, timestamp: number): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw", new TextEncoder().encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"],
  );
  const digest = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(`${timestamp}.${body}`));
  const hex = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  return `t=${timestamp},v1=${hex}`;
}

describe("Stripe adapter", () => {
  it("accepts a valid current raw-body signature", async () => {
    const body = '{"id":"evt_test"}';
    const header = await sign(body, "whsec_test", 2_000_000_000);
    await expect(verifyStripeSignature(body, header, "whsec_test", 2_000_000_100)).resolves.toBeUndefined();
  });

  it("rejects tampered and replayed signatures", async () => {
    const header = await sign("original", "whsec_test", 2_000_000_000);
    await expect(verifyStripeSignature("changed", header, "whsec_test", 2_000_000_100)).rejects.toThrow("STRIPE_SIGNATURE_INVALID");
    await expect(verifyStripeSignature("original", header, "whsec_test", 2_000_000_301)).rejects.toThrow("STRIPE_SIGNATURE_EXPIRED");
  });

  it("preserves access through cancel-at-period-end", () => {
    const projection = projectStripeSubscription({
      id: "sub_test", customer: "cus_test", status: "active", cancel_at_period_end: true,
      created: 2_000_000_000, metadata: { sitedatum_correlation_id: "correlation" },
      items: { data: [{ current_period_end: 2_100_000_000, price: { id: "price_monthly" } }] },
    }, "price_monthly", "price_annual");
    expect(projection).toMatchObject({ plan: "pro_monthly", status: "canceled", correlationId: "correlation" });
  });

  it("maps failed payment and terminal expiration without deleting anything", () => {
    const base = { id: "sub_test", customer: "cus_test", created: 2_000_000_000, items: { data: [{ current_period_end: 2_100_000_000, price: { id: "price_annual" } }] } };
    expect(projectStripeSubscription({ ...base, status: "past_due" }, "price_monthly", "price_annual").status).toBe("past_due");
    expect(projectStripeSubscription({ ...base, status: "canceled" }, "price_monthly", "price_annual").status).toBe("expired");
    expect(projectStripeSubscription({ ...base, status: "incomplete_expired" }, "price_monthly", "price_annual").status).toBe("expired");
  });
});
