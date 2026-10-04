export function buildCheckoutSessionParams(input: {
  price: string;
  correlation: string;
  integrationIdentifier: string;
  successUrl: string;
  cancelUrl: string;
}): URLSearchParams {
  return new URLSearchParams({
    mode: "subscription",
    "line_items[0][price]": input.price,
    "line_items[0][quantity]": "1",
    client_reference_id: input.correlation,
    integration_identifier: input.integrationIdentifier,
    "subscription_data[metadata][sitedatum_correlation_id]": input.correlation,
    "managed_payments[enabled]": "true",
    success_url: input.successUrl,
    cancel_url: input.cancelUrl,
  });
}

export function checkoutIntegrationIdentifier(randomBytes: Uint8Array): string {
  if (randomBytes.length < 4) throw new Error("INTEGRATION_IDENTIFIER_ENTROPY_REQUIRED");
  const suffix = [...randomBytes.slice(0, 4)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  return `sitedatum_windows_${suffix}`;
}
