export type LicensedPlan = "pro_monthly" | "pro_annual";
export type LicensedStatus = "active" | "past_due" | "canceled" | "expired";

type StripeSubscription = {
  id: string; customer: string; status: string; cancel_at_period_end?: boolean;
  ended_at?: number | null; current_period_end?: number; created: number;
  metadata?: Record<string, string>;
  items?: { data?: Array<{ current_period_end?: number; price?: { id?: string } }> };
};

export type StripeProjection = {
  subscriptionRef: string; customerRef: string; correlationId: string | null;
  plan: LicensedPlan; status: LicensedStatus; paidThroughUtc: string; providerUpdatedAtUtc: string;
};

function unixIso(value: number): string {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error("STRIPE_TIMESTAMP_INVALID");
  return new Date(value * 1000).toISOString();
}

export function projectStripeSubscription(subscription: StripeSubscription, monthlyPriceId: string, annualPriceId: string): StripeProjection {
  const item = subscription.items?.data?.[0];
  const priceId = item?.price?.id;
  const plan = priceId === monthlyPriceId ? "pro_monthly" : priceId === annualPriceId ? "pro_annual" : null;
  if (!plan) throw new Error("STRIPE_PRICE_UNMAPPED");
  const paidThrough = item?.current_period_end ?? subscription.current_period_end ?? subscription.ended_at;
  if (!paidThrough) throw new Error("STRIPE_PERIOD_END_MISSING");
  const status: LicensedStatus = subscription.status === "active" || subscription.status === "trialing"
    ? (subscription.cancel_at_period_end ? "canceled" : "active")
    : subscription.status === "past_due" || subscription.status === "unpaid"
      ? "past_due"
      : "expired";
  return {
    subscriptionRef: subscription.id, customerRef: subscription.customer,
    correlationId: subscription.metadata?.sitedatum_correlation_id ?? null,
    plan, status, paidThroughUtc: unixIso(paidThrough), providerUpdatedAtUtc: unixIso(subscription.created),
  };
}

function parseSignature(header: string): { timestamp: number; signatures: string[] } {
  const parts = header.split(",");
  const timestamp = Number(parts.find((part) => part.startsWith("t="))?.slice(2));
  const signatures = parts.filter((part) => part.startsWith("v1=")).map((part) => part.slice(3));
  if (!Number.isSafeInteger(timestamp) || signatures.length === 0) throw new Error("STRIPE_SIGNATURE_INVALID");
  return { timestamp, signatures };
}

function fromHex(value: string): ArrayBuffer | null {
  if (!/^[0-9a-f]{64}$/i.test(value)) return null;
  const bytes = Uint8Array.from(value.match(/.{2}/g)!.map((byte) => Number.parseInt(byte, 16)));
  return bytes.buffer as ArrayBuffer;
}

export async function verifyStripeSignature(body: string, header: string, secret: string, nowSeconds = Math.floor(Date.now() / 1000)): Promise<void> {
  const { timestamp, signatures } = parseSignature(header);
  if (Math.abs(nowSeconds - timestamp) > 300) throw new Error("STRIPE_SIGNATURE_EXPIRED");
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["verify"]);
  const signed = new TextEncoder().encode(`${timestamp}.${body}`);
  for (const signature of signatures) {
    const bytes = fromHex(signature);
    if (bytes && await crypto.subtle.verify("HMAC", key, bytes, signed)) return;
  }
  throw new Error("STRIPE_SIGNATURE_INVALID");
}
