import { describe, expect, it, vi } from "vitest";
import {
  CHECKOUT_RATE_LIMIT,
  CHECKOUT_RATE_WINDOW_SECONDS,
  consumeCheckoutRateLimit,
} from "../supabase/functions/_shared/checkout_rate_limit";
import {
  PayloadTooLargeError,
  readTextBodyLimited,
} from "../supabase/functions/_shared/request_body";

describe("checkout rate limiting", () => {
  it("uses one hashed subject bucket for every checkout plan", async () => {
    const rpc = vi.fn().mockResolvedValue({ data: true, error: null });
    await expect(consumeCheckoutRateLimit({ rpc }, "customer-subject")).resolves.toBe("allowed");
    expect(rpc).toHaveBeenCalledWith("licensing_consume_rate_limit", {
      p_key_hash: expect.stringMatching(/^[0-9a-f]{64}$/),
      p_action: "stripe_checkout",
      p_limit: CHECKOUT_RATE_LIMIT,
      p_window_seconds: CHECKOUT_RATE_WINDOW_SECONDS,
    });
  });

  it("fails closed when the database is unavailable and reports exhaustion", async () => {
    const unavailable = vi.fn().mockResolvedValue({ data: null, error: new Error("offline") });
    const limited = vi.fn().mockResolvedValue({ data: false, error: null });
    await expect(consumeCheckoutRateLimit({ rpc: unavailable }, "subject")).resolves.toBe("unavailable");
    await expect(consumeCheckoutRateLimit({ rpc: limited }, "subject")).resolves.toBe("limited");
  });
});

describe("bounded webhook request bodies", () => {
  it("accepts a body at the byte limit", async () => {
    const request = new Request("https://example.invalid", { method: "POST", body: "éé" });
    await expect(readTextBodyLimited(request, 4)).resolves.toBe("éé");
  });

  it("rejects a declared oversized body before reading it", async () => {
    const request = new Request("https://example.invalid", {
      method: "POST",
      headers: { "content-length": "5" },
      body: "tiny",
    });
    await expect(readTextBodyLimited(request, 4)).rejects.toBeInstanceOf(PayloadTooLargeError);
  });

  it("rejects actual bytes when content length is missing or understated", async () => {
    const stream = () => new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("123"));
        controller.enqueue(new TextEncoder().encode("456"));
        controller.close();
      },
    });
    await expect(readTextBodyLimited(new Request("https://example.invalid", {
      method: "POST", body: stream(), duplex: "half",
    } as RequestInit), 5)).rejects.toBeInstanceOf(PayloadTooLargeError);
    await expect(readTextBodyLimited(new Request("https://example.invalid", {
      method: "POST", headers: { "content-length": "2" }, body: stream(), duplex: "half",
    } as RequestInit), 5)).rejects.toBeInstanceOf(PayloadTooLargeError);
  });
});
