/**
 * Tests for mediaEncrypt.ts
 *
 * Run with: npm run test
 */

import { describe, it, expect } from "vitest";
import { encryptMedia, decryptMedia } from "./mediaEncrypt.js";

function b64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

describe("mediaEncrypt", () => {
  it("round-trips a blob: encrypt then decrypt returns original bytes", async () => {
    const original = new Uint8Array([1, 2, 3, 4, 5, 42, 255]);
    const blob = new Blob([original], { type: "application/octet-stream" });

    const { encrypted, key, iv } = await encryptMedia(blob);

    // Encrypted output should differ from plaintext
    const encBytes = new Uint8Array(encrypted);
    expect(encBytes).not.toEqual(original);

    // Decryption must restore original bytes exactly
    const decrypted = await decryptMedia(
      encrypted,
      key,
      iv,
      "application/octet-stream",
    );
    const decBytes = new Uint8Array(await decrypted.arrayBuffer());
    expect(decBytes).toEqual(original);
  });

  it("produces different ciphertext each call (random IV)", async () => {
    const blob = new Blob([new Uint8Array([10, 20, 30])]);

    const first = await encryptMedia(blob);
    const second = await encryptMedia(blob);

    // Keys and IVs must differ (cryptographically random)
    expect(first.key).not.toBe(second.key);
    expect(first.iv).not.toBe(second.iv);

    // Ciphertext sizes should match but content will differ
    expect(first.encrypted.byteLength).toBe(second.encrypted.byteLength);
  });

  it("returns a 32-byte key and 12-byte IV", async () => {
    const blob = new Blob([new Uint8Array([9, 8, 7, 6])]);
    const { key, iv } = await encryptMedia(blob);

    expect(b64ToBytes(key)).toHaveLength(32);
    expect(b64ToBytes(iv)).toHaveLength(12);
  });

  it("adds only the AES-GCM tag to the encrypted payload", async () => {
    const original = new Uint8Array([1, 2, 3, 4, 5, 42, 255]);
    const blob = new Blob([original], { type: "application/octet-stream" });

    const { encrypted } = await encryptMedia(blob);

    expect(encrypted.byteLength).toBe(original.byteLength + 16);
  });

  it("decrypt with wrong key throws", async () => {
    const blob = new Blob([new Uint8Array([1, 2, 3])]);
    const { encrypted, iv } = await encryptMedia(blob);

    // Generate a fresh (wrong) key
    const wrongKeyBuf = crypto.getRandomValues(new Uint8Array(32));
    let wrongKeyBinary = "";
    for (let index = 0; index < wrongKeyBuf.length; index += 1) {
      wrongKeyBinary += String.fromCharCode(wrongKeyBuf[index]);
    }
    const wrongKey = btoa(wrongKeyBinary);

    await expect(
      decryptMedia(encrypted, wrongKey, iv, "application/octet-stream"),
    ).rejects.toThrow();
  });
});
