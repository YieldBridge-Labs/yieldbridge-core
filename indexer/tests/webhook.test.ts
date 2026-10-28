import { describe, it, expect } from "vitest";
import { WebhookDispatcher } from "../src/webhook.js";

describe("Webhook Dispatcher Engine", () => {
  it("should generate a deterministic HMAC-SHA256 signature", () => {
    const dispatcher = new WebhookDispatcher();
    const payload = JSON.stringify({ event: "YIELD_INJECTED", amount: "1000" });
    const secret = "test-webhook-secret-key-123456";
    const timestamp = 1730000000;

    const signatureHeader = dispatcher.generateSignature(payload, secret, timestamp);
    expect(signatureHeader).toContain(`t=${timestamp},v1=`);
  });

  it("should fail gracefully when endpoint is unreachable and exhaust retries", async () => {
    const dispatcher = new WebhookDispatcher({
      maxRetries: 2,
      initialBackoffMs: 10,
      maxBackoffMs: 50,
      requestTimeoutMs: 100,
    });

    const result = await dispatcher.dispatch(
      "sub-123",
      "http://127.0.0.1:54321/non-existent-webhook",
      "secret-key",
      {
        id: "evt-001",
        timestamp: Date.now(),
        event: "YIELD_CLAIMED",
        contractAddress: "CDXFFV6Y5ZLIWECDCFDJAR3NVZCI6DK6IBQG2CCGBTC6Q5ER6ZKSW47V",
        data: { amount: "500" },
      }
    );

    expect(result.success).toBe(false);
    expect(result.attempts).toBe(2);
  });
});
