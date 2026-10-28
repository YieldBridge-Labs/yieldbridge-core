import express, { type Request, type Response } from "express";
import cors from "cors";
import { PrismaClient } from "@prisma/client";
import {
  ApproveCampaignSchema,
  WebhookSubscriptionSchema,
} from "./types.js";
import {
  sealForAuditor,
  unsealForAuditor,
  generateAuditorKeyPair,
} from "./crypto.js";

const prisma = new PrismaClient();
const app = express();

app.use(cors());
app.use(express.json());

// Request timestamping middleware
app.use((req, res, next) => {
  res.setHeader("X-YieldBridge-Version", "v1.0.0");
  next();
});

/**
 * Health check endpoint reporting ledger sync status.
 */
app.get("/health", async (_req: Request, res: Response) => {
  try {
    const cursor = await prisma.indexerCursor.findUnique({
      where: { id: "MAIN_CURSOR" },
    });
    res.json({
      status: "UP",
      protocol: "YieldBridge Core",
      database: "CONNECTED",
      lastProcessedLedger: cursor ? cursor.lastProcessedLedger.toString() : "0",
      syncStatus: cursor ? cursor.status : "NOT_STARTED",
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    res.status(503).json({
      status: "DEGRADED",
      error: error instanceof Error ? error.message : "Unknown error",
    });
  }
});

/**
 * Lists all indexed YieldVault instances.
 */
app.get("/api/v1/vaults", async (_req: Request, res: Response) => {
  try {
    const vaults = await prisma.vault.findMany({
      include: {
        _count: {
          select: { investors: true, events: true },
        },
      },
      orderBy: { createdAt: "desc" },
    });

    const serialized = vaults.map((v) => ({
      ...v,
      streamDuration: v.streamDuration.toString(),
      lastUpdateTime: v.lastUpdateTime.toString(),
      periodFinish: v.periodFinish.toString(),
    }));

    res.json({ data: serialized, count: serialized.length });
  } catch (error) {
    res.status(500).json({ error: "Failed to fetch vaults" });
  }
});

/**
 * Returns detailed state for a specific vault.
 */
app.get("/api/v1/vaults/:address", async (req: Request, res: Response) => {
  try {
    const vault = await prisma.vault.findUnique({
      where: { address: req.params.address },
      include: {
        investors: true,
        commissions: { take: 10, orderBy: { disbursedAt: "desc" } },
      },
    });

    if (!vault) {
      return res.status(404).json({ error: "Vault not found" });
    }

    return res.json({
      data: {
        ...vault,
        streamDuration: vault.streamDuration.toString(),
        lastUpdateTime: vault.lastUpdateTime.toString(),
        periodFinish: vault.periodFinish.toString(),
      },
    });
  } catch (error) {
    return res.status(500).json({ error: "Failed to fetch vault details" });
  }
});

/**
 * Lists all active and pending multi-sig campaigns.
 */
app.get("/api/v1/campaigns", async (_req: Request, res: Response) => {
  try {
    const campaigns = await prisma.campaign.findMany({
      include: {
        approvals: true,
        _count: { select: { deposits: true } },
      },
      orderBy: { createdAt: "desc" },
    });

    const serialized = campaigns.map((c) => ({
      ...c,
      deadline: c.deadline.toString(),
    }));

    res.json({ data: serialized, count: serialized.length });
  } catch (error) {
    res.status(500).json({ error: "Failed to fetch campaigns" });
  }
});

/**
 * Submits an off-chain or on-chain multi-sig approval signature for a campaign.
 */
app.post("/api/v1/campaigns/:id/approve", async (req: Request, res: Response) => {
  try {
    const campaignId = Number(req.params.id);
    const parsed = ApproveCampaignSchema.parse({
      campaignId,
      ...req.body,
    });

    const campaign = await prisma.campaign.findUnique({
      where: { id: campaignId },
      include: { approvals: true },
    });

    if (!campaign) {
      return res.status(404).json({ error: "Campaign not found" });
    }

    // Record multi-sig approval in database
    const approval = await prisma.multiSigApproval.upsert({
      where: {
        campaignId_signerAddress: {
          campaignId,
          signerAddress: parsed.signerAddress,
        },
      },
      update: { signature: parsed.signature },
      create: {
        campaignId,
        signerAddress: parsed.signerAddress,
        signature: parsed.signature,
      },
    });

    return res.json({
      message: "Multi-signature approval registered successfully",
      approval,
      currentApprovals: campaign.approvals.length + 1,
      threshold: campaign.threshold,
    });
  } catch (error) {
    return res.status(400).json({
      error: error instanceof Error ? error.message : "Invalid approval payload",
    });
  }
});

/**
 * Audit log of indexed on-chain events.
 */
app.get("/api/v1/events", async (req: Request, res: Response) => {
  try {
    const limit = Math.min(Number(req.query.limit) || 50, 200);
    const contractAddress = req.query.contract as string | undefined;

    const events = await prisma.yieldEvent.findMany({
      where: contractAddress ? { contractAddress } : undefined,
      take: limit,
      orderBy: { ledgerSequence: "desc" },
    });

    const serialized = events.map((e) => ({
      ...e,
      ledgerSequence: e.ledgerSequence.toString(),
      blockTimestamp: e.blockTimestamp.toString(),
    }));

    res.json({ data: serialized, count: serialized.length });
  } catch (error) {
    res.status(500).json({ error: "Failed to retrieve events" });
  }
});

/**
 * Registers an external webhook listener.
 */
app.post("/api/v1/webhooks/subscribe", async (req: Request, res: Response) => {
  try {
    const input = WebhookSubscriptionSchema.parse(req.body);
    const subscription = await prisma.webhookSubscription.create({
      data: {
        targetUrl: input.targetUrl,
        secret: input.secret,
        subscribedEvents: input.subscribedEvents,
      },
    });

    res.status(201).json({
      message: "Webhook subscription registered successfully",
      subscriptionId: subscription.id,
      targetUrl: subscription.targetUrl,
    });
  } catch (error) {
    res.status(400).json({
      error: error instanceof Error ? error.message : "Invalid subscription request",
    });
  }
});

/**
 * Cryptographic sealing endpoint for compliance and KYC data.
 */
app.post("/api/v1/compliance/seal", async (req: Request, res: Response) => {
  try {
    const { payload, auditorPublicKeyPem } = req.body as {
      payload: string;
      auditorPublicKeyPem: string;
    };
    if (!payload || !auditorPublicKeyPem) {
      return res.status(400).json({ error: "Missing payload or auditorPublicKeyPem" });
    }

    const packet = sealForAuditor(payload, auditorPublicKeyPem);
    return res.json({ packet });
  } catch (error) {
    return res.status(500).json({
      error: error instanceof Error ? error.message : "Cryptographic sealing failed",
    });
  }
});

/**
 * Cryptographic unsealing endpoint for authorized auditor verification.
 */
app.post("/api/v1/compliance/unseal", async (req: Request, res: Response) => {
  try {
    const { packet, auditorPrivateKeyPem } = req.body as {
      packet: Parameters<typeof unsealForAuditor>[0];
      auditorPrivateKeyPem: string;
    };
    if (!packet || !auditorPrivateKeyPem) {
      return res.status(400).json({ error: "Missing packet or auditorPrivateKeyPem" });
    }

    const decrypted = unsealForAuditor(packet, auditorPrivateKeyPem);
    return res.json({ payload: decrypted.toString("utf-8") });
  } catch (error) {
    return res.status(400).json({
      error: error instanceof Error ? error.message : "Cryptographic unsealing failed",
    });
  }
});

/**
 * Helper endpoint to generate an auditor X25519 keypair for setup and staging.
 */
app.get("/api/v1/compliance/generate-keys", (_req: Request, res: Response) => {
  const keys = generateAuditorKeyPair();
  res.json(keys);
});

export default app;

if (process.env.NODE_ENV !== "test") {
  const PORT = process.env.PORT || 3001;
  app.listen(PORT, () => {
    // eslint-disable-next-line no-console
    console.info(`[YieldBridge API] Server running on port ${PORT}`);
  });
}
