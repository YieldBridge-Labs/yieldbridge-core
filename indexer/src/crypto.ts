import crypto from "node:crypto";
import type { SealedDataPacket } from "./types.js";

const HKDF_INFO_PREFIX = "YieldBridge-Auditor-V1";
const GCM_IV_LENGTH = 12; // 96 bits recommended for AES-GCM
const GCM_AUTH_TAG_LENGTH = 16; // 128 bits authentication tag

/**
 * Derives a 256-bit symmetric key from shared secret material using HKDF-SHA256.
 */
export function deriveSymmetricKey(
  sharedSecret: Buffer,
  salt: Buffer = Buffer.alloc(32, 0),
  info: string = HKDF_INFO_PREFIX
): Buffer {
  return Buffer.from(crypto.hkdfSync("sha256", sharedSecret, salt, Buffer.from(info, "utf-8"), 32));
}

/**
 * Seals arbitrary data using AES-256-GCM authenticated symmetric encryption.
 */
export function sealAes256Gcm(
  plaintext: Buffer,
  key: Buffer
): { ciphertext: string; iv: string; authTag: string } {
  if (key.length !== 32) {
    throw new Error("Invalid key length: AES-256 requires a 32-byte key.");
  }

  const iv = crypto.randomBytes(GCM_IV_LENGTH);
  const cipher = crypto.createCipheriv("aes-256-gcm", key, iv, {
    authTagLength: GCM_AUTH_TAG_LENGTH,
  });

  const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const authTag = cipher.getAuthTag();

  return {
    ciphertext: encrypted.toString("hex"),
    iv: iv.toString("hex"),
    authTag: authTag.toString("hex"),
  };
}

/**
 * Unseals ciphertext using AES-256-GCM authenticated symmetric decryption.
 */
export function unsealAes256Gcm(
  ciphertextHex: string,
  key: Buffer,
  ivHex: string,
  authTagHex: string
): Buffer {
  if (key.length !== 32) {
    throw new Error("Invalid key length: AES-256 requires a 32-byte key.");
  }

  const iv = Buffer.from(ivHex, "hex");
  const authTag = Buffer.from(authTagHex, "hex");
  const ciphertext = Buffer.from(ciphertextHex, "hex");

  const decipher = crypto.createDecipheriv("aes-256-gcm", key, iv, {
    authTagLength: GCM_AUTH_TAG_LENGTH,
  });
  decipher.setAuthTag(authTag);

  return Buffer.concat([decipher.update(ciphertext), decipher.final()]);
}

/**
 * Generates an X25519 keypair for ECDH key encapsulation.
 */
export function generateAuditorKeyPair(): { publicKeyPem: string; privateKeyPem: string } {
  const { publicKey, privateKey } = crypto.generateKeyPairSync("x25519", {
    publicKeyEncoding: { type: "spki", format: "pem" },
    privateKeyEncoding: { type: "pkcs8", format: "pem" },
  });
  return { publicKeyPem: publicKey, privateKeyPem: privateKey };
}

/**
 * Encapsulates and seals sensitive metadata for an authorized auditor's X25519 public key.
 *
 * 1. Generates ephemeral X25519 keypair.
 * 2. Derives shared secret via Diffie-Hellman exchange: S = ECDH(sk_e, pk_auditor).
 * 3. Derives 256-bit AES key via HKDF-SHA256.
 * 4. Seals payload with AES-256-GCM.
 * 5. Attaches SHA-256 checksum for tamper-evident data integrity verification.
 */
export function sealForAuditor(
  payload: Buffer | string,
  auditorPublicKeyPem: string
): SealedDataPacket {
  const dataBuffer = Buffer.isBuffer(payload) ? payload : Buffer.from(payload, "utf-8");
  const checksum = crypto.createHash("sha256").update(dataBuffer).digest("hex");

  // Generate ephemeral keypair
  const ephemeral = crypto.generateKeyPairSync("x25519");
  const ephemeralPublicKeyDer = ephemeral.publicKey.export({ type: "spki", format: "der" });

  // Compute shared secret using ECDH
  const auditorKey = crypto.createPublicKey(auditorPublicKeyPem);
  const sharedSecret = crypto.diffieHellman({
    privateKey: ephemeral.privateKey,
    publicKey: auditorKey,
  });

  // Derive symmetric AES key
  const salt = crypto.createHash("sha256").update(ephemeralPublicKeyDer).digest();
  const aesKey = deriveSymmetricKey(sharedSecret, salt, "YieldBridge-Auditor-Sealing-V1");

  // Encrypt with AES-256-GCM
  const { ciphertext, iv, authTag } = sealAes256Gcm(dataBuffer, aesKey);

  return {
    ciphertext,
    iv,
    authTag,
    ephemeralPublicKey: ephemeralPublicKeyDer.toString("hex"),
    auditorPublicKey: crypto.createHash("sha256").update(auditorPublicKeyPem).digest("hex"),
    checksum,
  };
}

/**
 * Unseals an encapsulated data packet using an authorized auditor's X25519 private key.
 */
export function unsealForAuditor(
  packet: SealedDataPacket,
  auditorPrivateKeyPem: string
): Buffer {
  const auditorPrivateKey = crypto.createPrivateKey(auditorPrivateKeyPem);
  const ephemeralPublicKey = crypto.createPublicKey({
    key: Buffer.from(packet.ephemeralPublicKey, "hex"),
    type: "spki",
    format: "der",
  });

  // Decapsulate shared secret via Diffie-Hellman
  const sharedSecret = crypto.diffieHellman({
    privateKey: auditorPrivateKey,
    publicKey: ephemeralPublicKey,
  });

  const salt = crypto.createHash("sha256").update(Buffer.from(packet.ephemeralPublicKey, "hex")).digest();
  const aesKey = deriveSymmetricKey(sharedSecret, salt, "YieldBridge-Auditor-Sealing-V1");

  // Decrypt and authenticate payload
  const decrypted = unsealAes256Gcm(packet.ciphertext, aesKey, packet.iv, packet.authTag);

  // Validate integrity checksum
  const verifiedChecksum = crypto.createHash("sha256").update(decrypted).digest("hex");
  if (verifiedChecksum !== packet.checksum) {
    throw new Error("Zero-Trust Verification Failed: Checksum mismatch on decrypted auditor packet.");
  }

  return decrypted;
}
