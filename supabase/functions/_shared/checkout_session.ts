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
    success_url: input.successUrl,
    cancel_url: input.cancelUrl,
  });
}
