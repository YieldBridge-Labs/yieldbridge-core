import crypto from "node:crypto";
import type { WebhookPayload, WebhookDeliveryResult } from "./types.js";

export interface WebhookConfig {
  maxRetries?: number;
  initialBackoffMs?: number;
  maxBackoffMs?: number;
  requestTimeoutMs?: number;
  rateLimitPerMinute?: number;
}

const DEFAULT_CONFIG: Required<WebhookConfig> = {
  maxRetries: 5,
  initialBackoffMs: 1000,
  maxBackoffMs: 30000,
  requestTimeoutMs: 10000,
  rateLimitPerMinute: 60,
};

/**
 * Token bucket rate-limiter per webhook destination URL.
 */
class HostRateLimiter {
  private buckets = new Map<string, { tokens: number; lastRefill: number }>();
  private readonly maxTokens: number;
  private readonly refillRatePerMs: number;

  constructor(limitPerMinute: number) {
    this.maxTokens = limitPerMinute;
    this.refillRatePerMs = limitPerMinute / 60000;
  }

  public async acquire(targetUrl: string): Promise<boolean> {
    const host = new URL(targetUrl).hostname;
    const now = Date.now();
    let bucket = this.buckets.get(host);

    if (!bucket) {
      bucket = { tokens: this.maxTokens, lastRefill: now };
      this.buckets.set(host, bucket);
    }

    const elapsed = now - bucket.lastRefill;
    bucket.tokens = Math.min(this.maxTokens, bucket.tokens + elapsed * this.refillRatePerMs);
    bucket.lastRefill = now;

    if (bucket.tokens >= 1) {
      bucket.tokens -= 1;
      return true;
    }
    return false;
  }
}

/**
 * Secure webhook dispatcher with HMAC-SHA256 signature, jittered exponential backoff,
 * and rate-limiting.
 */
export class WebhookDispatcher {
  private config: Required<WebhookConfig>;
  private rateLimiter: HostRateLimiter;

  constructor(config: WebhookConfig = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.rateLimiter = new HostRateLimiter(this.config.rateLimitPerMinute);
  }

  /**
   * Generates a tamper-proof HMAC-SHA256 signature header matching institutional standards.
   */
  public generateSignature(payloadString: string, secret: string, timestamp: number): string {
    const hmac = crypto.createHmac("sha256", secret);
    hmac.update(`${timestamp}.${payloadString}`);
    return `t=${timestamp},v1=${hmac.digest("hex")}`;
  }

  /**
   * Dispatches a webhook notification to an external subscriber URL with retries.
   */
  public async dispatch(
    subscriptionId: string,
    targetUrl: string,
    secret: string,
    payload: WebhookPayload
  ): Promise<WebhookDeliveryResult> {
    const payloadString = JSON.stringify(payload);
    let attempt = 0;
    let lastError = "";

    while (attempt < this.config.maxRetries) {
      attempt++;

      // Check rate limit
      const allowed = await this.rateLimiter.acquire(targetUrl);
      if (!allowed) {
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }

      const timestamp = Math.floor(Date.now() / 1000);
      const signatureHeader = this.generateSignature(payloadString, secret, timestamp);

      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.config.requestTimeoutMs);

        const response = await fetch(targetUrl, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "User-Agent": "YieldBridge-Webhook-Engine/1.0",
            "X-YieldBridge-Signature": signatureHeader,
            "X-YieldBridge-Delivery-Id": payload.id,
            "X-YieldBridge-Event": payload.event,
          },
          body: payloadString,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (response.ok) {
          return {
            subscriptionId,
            statusCode: response.status,
            attempts: attempt,
            success: true,
          };
        }

        lastError = `HTTP ${response.status}: ${response.statusText}`;
      } catch (err) {
        lastError = err instanceof Error ? err.message : String(err);
      }

      // If attempts remain, compute jittered exponential backoff
      if (attempt < this.config.maxRetries) {
        const baseDelay = this.config.initialBackoffMs * Math.pow(2, attempt - 1);
        const jitter = Math.floor(Math.random() * 500);
        const delay = Math.min(this.config.maxBackoffMs, baseDelay + jitter);
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }

    return {
      subscriptionId,
      statusCode: 0,
      attempts: attempt,
      success: false,
      error: lastError,
    };
  }
}
