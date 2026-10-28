import { describe, it, expect } from "vitest";
import {
  sealAes256Gcm,
  unsealAes256Gcm,
  generateAuditorKeyPair,
  sealForAuditor,
  unsealForAuditor,
} from "../src/crypto.js";
import crypto from "node:crypto";

describe("Cryptographic Sealing Engine (AES-256-GCM & ECDH X25519)", () => {
  it("should encrypt and decrypt using symmetric AES-256-GCM with authentication tag", () => {
    const key = crypto.randomBytes(32);
    const plaintext = Buffer.from("YieldBridge Protocol Sensitive Investor KYC Data", "utf-8");

    const { ciphertext, iv, authTag } = sealAes256Gcm(plaintext, key);
    expect(ciphertext).toBeDefined();
    expect(iv).toHaveLength(24); // 12 bytes = 24 hex characters
    expect(authTag).toHaveLength(32); // 16 bytes = 32 hex characters

    const decrypted = unsealAes256Gcm(ciphertext, key, iv, authTag);
    expect(decrypted.toString("utf-8")).toBe(plaintext.toString("utf-8"));
  });

  it("should fail decryption when ciphertext or authentication tag is tampered with", () => {
    const key = crypto.randomBytes(32);
    const plaintext = Buffer.from("Confidential Yield Stream Schedule", "utf-8");

    const { ciphertext, iv, authTag } = sealAes256Gcm(plaintext, key);

    // Tamper with the authentication tag
    const tamperedTag = "0" + authTag.slice(1);
    expect(() => {
      unsealAes256Gcm(ciphertext, key, iv, tamperedTag);
    }).toThrow();
  });

  it("should execute end-to-end ECDH key encapsulation and unsealing for an auditor", () => {
    const { publicKeyPem, privateKeyPem } = generateAuditorKeyPair();
    const secretMessage = JSON.stringify({
      investor: "GA23MAON7RAVLAZLWSF47WMEBY566IPUQL4MQVFSADCNEWCETXBV7TVR",
      accreditedStatus: true,
      tier: "Institutional",
      timestamp: Date.now(),
    });

    // 1. Seal for auditor using public key
    const packet = sealForAuditor(secretMessage, publicKeyPem);
    expect(packet.ciphertext).toBeDefined();
    expect(packet.ephemeralPublicKey).toBeDefined();
    expect(packet.checksum).toBeDefined();

    // 2. Unseal using auditor private key
    const decryptedBuffer = unsealForAuditor(packet, privateKeyPem);
    expect(decryptedBuffer.toString("utf-8")).toBe(secretMessage);
  });

  it("should reject unsealing if wrong auditor private key is supplied", () => {
    const auditor1 = generateAuditorKeyPair();
    const attacker = generateAuditorKeyPair();

    const packet = sealForAuditor("Highly Classified Campaign KYC", auditor1.publicKeyPem);

    expect(() => {
      unsealForAuditor(packet, attacker.privateKeyPem);
    }).toThrow();
  });
});
