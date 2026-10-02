import { sha256Hex } from "./sha256.ts";

type RpcClient = {
  rpc(name: string, parameters: Record<string, unknown>): PromiseLike<{
    data: unknown;
    error: unknown;
  }>;
};

export const CHECKOUT_RATE_LIMIT = 10;
export const CHECKOUT_RATE_WINDOW_SECONDS = 60 * 60;

export async function consumeCheckoutRateLimit(
  admin: RpcClient,
  userId: string,
): Promise<"allowed" | "limited" | "unavailable"> {
  const { data, error } = await admin.rpc("licensing_consume_rate_limit", {
    p_key_hash: await sha256Hex(userId),
    p_action: "stripe_checkout",
    p_limit: CHECKOUT_RATE_LIMIT,
    p_window_seconds: CHECKOUT_RATE_WINDOW_SECONDS,
  });
  if (error) return "unavailable";
  return data === true ? "allowed" : "limited";
}
