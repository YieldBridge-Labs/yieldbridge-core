import { z } from "zod";

/**
 * On-chain event types emitted by YieldBridge contracts.
 */
export const EventTypeEnum = z.enum([
  "VAULT_INITIALIZED",
  "SHARES_UPDATED",
  "YIELD_INJECTED",
  "YIELD_CLAIMED",
  "CAMPAIGN_CREATED",
  "CAMPAIGN_APPROVED",
  "CAMPAIGN_ACTIVATED",
  "FUNDS_DEPOSITED",
  "COMMISSION_DISBURSED",
  "CAMPAIGN_FINALIZED",
  "REFUND_CLAIMED",
  "FACTORY_INITIALIZED",
  "VAULT_DEPLOYED",
]);
export type EventType = z.infer<typeof EventTypeEnum>;

/**
 * Normalized on-chain event structure.
 */
export interface NormalizedEvent {
  contractAddress: string;
  eventType: EventType;
  txHash: string;
  ledgerSequence: bigint;
  blockTimestamp: bigint;
  data: Record<string, unknown>;
}

/**
 * Sealed payload packet for encrypted metadata (AES-256-GCM + ECDH).
 */
export interface SealedDataPacket {
  ciphertext: string;
  iv: string;
  authTag: string;
  ephemeralPublicKey: string;
  auditorPublicKey: string;
  checksum: string;
}

/**
 * Webhook delivery payload.
 */
export interface WebhookPayload {
  id: string;
  timestamp: number;
  event: EventType;
  contractAddress: string;
  data: Record<string, unknown>;
}

/**
 * Webhook delivery result.
 */
export interface WebhookDeliveryResult {
  subscriptionId: string;
  statusCode: number;
  attempts: number;
  success: boolean;
  error?: string;
}

/**
 * Zod validation schema for campaign approval submission.
 */
export const ApproveCampaignSchema = z.object({
  campaignId: z.number().int().positive(),
  signerAddress: z.string().regex(/^G[A-Z0-9]{55}$/, "Invalid Stellar public key format"),
  signature: z.string().min(64, "Signature must be valid hex or base64 string"),
});
export type ApproveCampaignInput = z.infer<typeof ApproveCampaignSchema>;

/**
 * Zod validation schema for webhook subscription creation.
 */
export const WebhookSubscriptionSchema = z.object({
  targetUrl: z.string().url(),
  secret: z.string().min(16, "Secret must be at least 16 characters for HMAC security"),
  subscribedEvents: z.array(EventTypeEnum).nonempty("Must subscribe to at least one event type"),
});
export type WebhookSubscriptionInput = z.infer<typeof WebhookSubscriptionSchema>;
