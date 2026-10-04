export const EARLY_ACCESS_CHECKOUT_DISCLOSURE =
  "SiteDatum Early Access uses an unsigned Windows installer. Windows or an organization's security policy may warn or block installation. Do not disable security protections. Review installation and refund terms at sitedatum.site/early-access.html.";

export function buildCheckoutSessionParams(input: {
  price: string;
  correlation: string;
  successUrl: string;
  cancelUrl: string;
}): URLSearchParams {
  return new URLSearchParams({
    mode: "subscription",
    "line_items[0][price]": input.price,
    "line_items[0][quantity]": "1",
    client_reference_id: input.correlation,
    "subscription_data[metadata][sitedatum_correlation_id]": input.correlation,
    "managed_payments[enabled]": "true",
    "custom_text[submit][message]": EARLY_ACCESS_CHECKOUT_DISCLOSURE,
    success_url: input.successUrl,
    cancel_url: input.cancelUrl,
  });
}
