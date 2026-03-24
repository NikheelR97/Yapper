import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/api/client.js", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
}));

vi.mock("./keystore.js", () => ({
  exportSignalSnapshot: vi.fn(),
  importSignalSnapshot: vi.fn(),
  loadIdentityKeyPair: vi.fn(),
}));

import { api } from "$lib/api/client.js";
import { importSignalSnapshot, loadIdentityKeyPair } from "./keystore.js";
import { backupKeys, restoreKeys } from "./backup.js";

const PBKDF2_ITERS = 1_200_000;
const BACKUP_BLOB_VERSION = 1;

function u8ToB64(u8: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < u8.length; index += 1) {
    binary += String.fromCharCode(u8[index]);
  }
  return btoa(binary);
}

async function deriveKey(
  passphrase: string,
  salt: Uint8Array,
): Promise<CryptoKey> {
  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(passphrase),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt.slice(),
      hash: "SHA-256",
      iterations: PBKDF2_ITERS,
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

async function encryptBackupBlob(
  passphrase: string,
  snapshot: string,
): Promise<string> {
  const salt = new Uint8Array(16).fill(7);
  const iv = new Uint8Array(12).fill(9);
  const key = await deriveKey(passphrase, salt);
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt(
      { name: "AES-GCM", iv },
      key,
      new TextEncoder().encode(snapshot),
    ),
  );

  const blob = new Uint8Array(1 + salt.length + iv.length + ciphertext.length);
  blob[0] = BACKUP_BLOB_VERSION;
  blob.set(salt, 1);
  blob.set(iv, 1 + salt.length);
  blob.set(ciphertext, 1 + salt.length + iv.length);
  return u8ToB64(blob);
}

describe("signal backup restore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(loadIdentityKeyPair).mockResolvedValue({
      dhPublicKey: new Uint8Array(32),
      dhPrivateKey: new Uint8Array(32).fill(1),
      sigPublicKey: new Uint8Array(32).fill(2),
      sigPrivateKey: new Uint8Array(32).fill(7),
    });
  });

  it("restores a backup without replacing the source device by default", async () => {
    const snapshot = JSON.stringify({ bootstrapComplete: true });
    vi.mocked(api.get).mockResolvedValue({
      encrypted_blob: await encryptBackupBlob("AlphaPass2468", snapshot),
    });
    vi.mocked(api.post).mockResolvedValue({});

    const restored = await restoreKeys("AlphaPass2468", "source-device-1", {
      currentDeviceId: "pending-device-1",
    });

    expect(restored).toBe(true);
    expect(api.get).toHaveBeenCalledWith(
      "/api/v2/keys/backup?source_device_id=source-device-1",
    );
    expect(importSignalSnapshot).toHaveBeenCalledWith({
      bootstrapComplete: true,
    });
    expect(api.post).toHaveBeenCalledTimes(1);
    expect(api.post).toHaveBeenCalledWith(
      "/api/v2/keys/backup/restore",
      expect.objectContaining({
        source_device_id: "source-device-1",
        source_device_signature: expect.any(String),
        replace_source_device: false,
      }),
    );
  });

  it("can explicitly request source-device replacement", async () => {
    const snapshot = JSON.stringify({ bootstrapComplete: true });
    vi.mocked(api.get).mockResolvedValue({
      encrypted_blob: await encryptBackupBlob("AlphaPass2468", snapshot),
    });
    vi.mocked(api.post).mockResolvedValue({});

    const restored = await restoreKeys("AlphaPass2468", "source-device-1", {
      replaceSourceDevice: true,
      currentDeviceId: "pending-device-1",
    });

    expect(restored).toBe(true);
    expect(api.post).toHaveBeenCalledWith(
      "/api/v2/keys/backup/restore",
      expect.objectContaining({
        source_device_id: "source-device-1",
        source_device_signature: expect.any(String),
        replace_source_device: true,
      }),
    );
  });

  it("returns false when no backup exists for the selected source device", async () => {
    vi.mocked(api.get).mockRejectedValue({ status: 404 });

    const restored = await restoreKeys("AlphaPass2468", "missing-device", {
      currentDeviceId: "pending-device-1",
    });

    expect(restored).toBe(false);
    expect(api.post).not.toHaveBeenCalled();
    expect(importSignalSnapshot).not.toHaveBeenCalled();
  });

  it("rejects weak recovery passphrases before upload", async () => {
    await expect(backupKeys("2468")).rejects.toThrow(/at least 12 characters/i);
    expect(api.put).not.toHaveBeenCalled();
  });
});
