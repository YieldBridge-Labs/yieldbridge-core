import { PrismaClient } from "@prisma/client";
import { rpc, scValToNative } from "@stellar/stellar-sdk";
import { WebhookDispatcher } from "./webhook.js";
import type { NormalizedEvent, EventType } from "./types.js";

const prisma = new PrismaClient();
const webhookDispatcher = new WebhookDispatcher();

export interface IndexerOptions {
  rpcUrl: string;
  contractAddresses: string[];
  pollIntervalMs?: number;
  batchLedgerWindow?: number;
}

export class SorobanEventIndexer {
  private server: rpc.Server;
  private contractAddresses: string[];
  private pollIntervalMs: number;
  private isRunning = false;

  constructor(options: IndexerOptions) {
    this.server = new rpc.Server(options.rpcUrl);
    this.contractAddresses = options.contractAddresses;
    this.pollIntervalMs = options.pollIntervalMs ?? 5000; // Aligns with Stellar's 5-second ledger close
  }

  /**
   * Starts the continuous event synchronization loop.
   */
  public async start(): Promise<void> {
    this.isRunning = true;
    // eslint-disable-next-line no-console
    console.info("[YieldBridge Indexer] Initializing Soroban event stream listener...");

    while (this.isRunning) {
      try {
        await this.syncEvents();
      } catch (error) {
        // eslint-disable-next-line no-console
        console.error("[YieldBridge Indexer] Sync cycle error, backing off:", error);
      }
      await new Promise((resolve) => setTimeout(resolve, this.pollIntervalMs));
    }
  }

  public stop(): void {
    this.isRunning = false;
  }

  /**
   * Synchronizes events from the last processed ledger cursor.
   */
  public async syncEvents(): Promise<void> {
    // 1. Fetch or initialize cursor
    let cursor = await prisma.indexerCursor.findUnique({
      where: { id: "MAIN_CURSOR" },
    });

    const latestLedger = await this.server.getLatestLedger();
    const currentLedgerSeq = BigInt(latestLedger.sequence);

    if (!cursor) {
      // Default to 100 ledgers prior if no cursor exists
      const startSeq = currentLedgerSeq > 100n ? currentLedgerSeq - 100n : 1n;
      cursor = await prisma.indexerCursor.create({
        data: {
          id: "MAIN_CURSOR",
          lastProcessedLedger: startSeq,
          lastProcessedTimestamp: BigInt(Math.floor(Date.now() / 1000)),
          status: "INITIALIZING",
        },
      });
    }

    const startLedger = Number(cursor.lastProcessedLedger) + 1;
    if (startLedger > Number(currentLedgerSeq)) {
      return; // Already up to date with ledger head
    }

    // 2. Query Soroban RPC getEvents
    const response = await this.server.getEvents({
      startLedger,
      filters: [
        {
          type: "contract",
          contractIds: this.contractAddresses,
        },
      ],
    });

    if (!response.events || response.events.length === 0) {
      await prisma.indexerCursor.update({
        where: { id: "MAIN_CURSOR" },
        data: {
          lastProcessedLedger: currentLedgerSeq,
          status: "SYNCED",
        },
      });
      return;
    }

    // 3. Process events transactionally
    for (const rawEvent of response.events) {
      const normalized = this.parseRawEvent(rawEvent);
      if (!normalized) continue;

      await this.processEventTransaction(normalized);
      await this.dispatchWebhooks(normalized);
    }

    // 4. Update cursor to latest processed event ledger
    const lastEventLedger = BigInt(response.events[response.events.length - 1]!.ledger);
    await prisma.indexerCursor.update({
      where: { id: "MAIN_CURSOR" },
      data: {
        lastProcessedLedger: lastEventLedger > currentLedgerSeq ? lastEventLedger : currentLedgerSeq,
        lastProcessedTimestamp: BigInt(Math.floor(Date.now() / 1000)),
        status: "SYNCED",
      },
    });
  }

  /**
   * Normalizes raw Soroban RPC events into strongly typed domain events.
   */
  private parseRawEvent(raw: rpc.Api.RawEventResponse): NormalizedEvent | null {
    try {
      const topics = raw.topic.map((t) => scValToNative(t));
      const eventName = String(topics[0] || "");
      const value = scValToNative(raw.value);

      return {
        contractAddress: raw.contractId,
        eventType: eventName as EventType,
        txHash: raw.txHash,
        ledgerSequence: BigInt(raw.ledger),
        blockTimestamp: BigInt(Math.floor(Date.now() / 1000)),
        data: typeof value === "object" && value !== null ? (value as Record<string, unknown>) : { value },
      };
    } catch (err) {
      // eslint-disable-next-line no-console
      console.warn("[YieldBridge Indexer] Failed to parse event XDR:", err);
      return null;
    }
  }

  /**
   * Updates PostgreSQL tables atomically within a Prisma transaction.
   */
  private async processEventTransaction(event: NormalizedEvent): Promise<void> {
    await prisma.$transaction(async (tx) => {
      // Record immutable event log
      await tx.yieldEvent.create({
        data: {
          txHash: event.txHash,
          ledgerSequence: event.ledgerSequence,
          contractAddress: event.contractAddress,
          eventType: event.eventType,
          data: event.data as Record<string, string | number | boolean | null>,
          blockTimestamp: event.blockTimestamp,
        },
      });

      // Domain-specific state projection
      switch (event.eventType) {
        case "VAULT_INITIALIZED": {
          const { admin, token, duration } = event.data as {
            admin: string;
            token: string;
            duration: number | bigint;
          };
          await tx.vault.upsert({
            where: { address: event.contractAddress },
            update: { adminAddress: admin, tokenAddress: token, streamDuration: BigInt(duration) },
            create: {
              address: event.contractAddress,
              adminAddress: admin,
              tokenAddress: token,
              streamDuration: BigInt(duration),
              lastUpdateTime: event.blockTimestamp,
              periodFinish: event.blockTimestamp + BigInt(duration),
            },
          });
          break;
        }

        case "YIELD_INJECTED": {
          const { amount, period_finish } = event.data as {
            amount: string | number;
            period_finish: number | bigint;
          };
          await tx.vault.update({
            where: { address: event.contractAddress },
            data: {
              totalFunded: {
                // Keep string arithmetic precision
                set: String(amount),
              },
              periodFinish: BigInt(period_finish),
              lastUpdateTime: event.blockTimestamp,
            },
          });
          break;
        }

        case "SHARES_UPDATED": {
          const { investor, shares, total_shares } = event.data as {
            investor: string;
            shares: string | number;
            total_shares: string | number;
          };
          await tx.investor.upsert({
            where: {
              vaultAddress_investorAddress: {
                vaultAddress: event.contractAddress,
                investorAddress: investor,
              },
            },
            update: { shares: String(shares) },
            create: {
              vaultAddress: event.contractAddress,
              investorAddress: investor,
              shares: String(shares),
            },
          });
          await tx.vault.update({
            where: { address: event.contractAddress },
            data: { totalShares: String(total_shares) },
          });
          break;
        }

        case "COMMISSION_DISBURSED": {
          const { campaign_id, treasury, commission_amount, net_payout } = event.data as {
            campaign_id?: number;
            treasury: string;
            commission_amount: string | number;
            net_payout: string | number;
          };
          await tx.commissionDisbursement.create({
            data: {
              campaignId: campaign_id ? Number(campaign_id) : null,
              vaultAddress: event.contractAddress,
              txHash: event.txHash,
              treasuryAddress: treasury,
              commissionAmount: String(commission_amount),
              netPayout: String(net_payout),
            },
          });
          break;
        }

        default:
          break;
      }
    });
  }

  /**
   * Notifies registered webhooks of matching events.
   */
  private async dispatchWebhooks(event: NormalizedEvent): Promise<void> {
    const subscriptions = await prisma.webhookSubscription.findMany({
      where: {
        isActive: true,
        subscribedEvents: { has: event.eventType },
      },
    });

    for (const sub of subscriptions) {
      webhookDispatcher
        .dispatch(sub.id, sub.targetUrl, sub.secret, {
          id: `${event.txHash}-${event.eventType}`,
          timestamp: Number(event.blockTimestamp),
          event: event.eventType,
          contractAddress: event.contractAddress,
          data: event.data,
        })
        .catch((err) => {
          // eslint-disable-next-line no-console
          console.error(`[Webhook] Delivery failure for subscription ${sub.id}:`, err);
        });
    }
  }
}
