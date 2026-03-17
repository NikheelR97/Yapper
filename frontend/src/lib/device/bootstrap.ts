import { openDB, type DBSchema, type IDBPDatabase } from "idb";

import { platform as runtimePlatform } from "$lib/plugins/tauri-compat.js";
import type { AuthDevice } from "$stores/auth.js";

const INSTALLATION_ID_KEY = "yapper_installation_id";
const DEVICE_DB_NAME = "yapper-device-bootstrap";
const DEVICE_STORE_NAME = "state";
const DEVICE_DB_VERSION = 1;

interface DeviceBootstrapDb extends DBSchema {
  [DEVICE_STORE_NAME]: {
    key: string;
    value: string;
  };
}

let _deviceDb: IDBPDatabase<DeviceBootstrapDb> | null = null;
let _deviceDbPromise: Promise<IDBPDatabase<DeviceBootstrapDb>> | null = null;

export interface DeviceBootstrap {
  installation_id: string;
  platform: "web" | "tauri" | "capacitor";
  label: string;
}

interface ServerDeviceShape {
  id: string;
  signal_device_id: number;
  installation_id: string | null;
  platform: "web" | "tauri" | "capacitor";
  label: string;
  trust_state: "trusted" | "pending_trust" | "revoked";
  created_at: string;
  last_seen_at: string | null;
  approved_at: string | null;
  revoked_at: string | null;
}

async function getDeviceDb(): Promise<IDBPDatabase<DeviceBootstrapDb>> {
  if (_deviceDb) return _deviceDb;
  if (!_deviceDbPromise) {
    _deviceDbPromise = openDB<DeviceBootstrapDb>(DEVICE_DB_NAME, DEVICE_DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(DEVICE_STORE_NAME)) {
          db.createObjectStore(DEVICE_STORE_NAME);
        }
      },
    }).then((db) => {
      _deviceDb = db;
      return db;
    });
  }
  return _deviceDbPromise;
}

async function loadStoredInstallationId(): Promise<string | null> {
  try {
    const db = await getDeviceDb();
    return (await db.get(DEVICE_STORE_NAME, INSTALLATION_ID_KEY)) ?? null;
  } catch {
    return null;
  }
}

async function persistInstallationId(installationId: string): Promise<void> {
  const db = await getDeviceDb();
  await db.put(DEVICE_STORE_NAME, installationId, INSTALLATION_ID_KEY);
}

export async function getDeviceBootstrap(): Promise<DeviceBootstrap> {
  const installationId = await getOrCreateInstallationId();
  const platform = runtimePlatform();
  return {
    installation_id: installationId,
    platform,
    label: defaultDeviceLabel(platform),
  };
}

export function normalizeServerDevice(device: ServerDeviceShape): AuthDevice {
  return {
    id: device.id,
    signalDeviceId: device.signal_device_id,
    installationId: device.installation_id,
    platform: device.platform,
    label: device.label,
    trustState: device.trust_state,
    createdAt: device.created_at,
    lastSeenAt: device.last_seen_at,
    approvedAt: device.approved_at,
    revokedAt: device.revoked_at,
  };
}

async function getOrCreateInstallationId(): Promise<string> {
  if (typeof window === "undefined") {
    return crypto.randomUUID();
  }
  if (typeof indexedDB === "undefined") {
    throw new Error("IndexedDB is unavailable for secure device storage");
  }

  const persisted = await loadStoredInstallationId();
  if (persisted) {
    try {
      window.localStorage.removeItem(INSTALLATION_ID_KEY);
    } catch {
      // ignore legacy cleanup failures
    }
    return persisted;
  }

  let legacyStored: string | null = null;
  try {
    legacyStored = window.localStorage.getItem(INSTALLATION_ID_KEY);
  } catch {
    legacyStored = null;
  }

  const installationId = legacyStored ?? crypto.randomUUID();
  await persistInstallationId(installationId);

  try {
    window.localStorage.removeItem(INSTALLATION_ID_KEY);
  } catch {
    // ignore legacy cleanup failures
  }

  return installationId;
}

function defaultDeviceLabel(platform: DeviceBootstrap["platform"]): string {
  switch (platform) {
    case "tauri":
      return "Desktop App";
    case "capacitor":
      return "Mobile App";
    default:
      return "Web Browser";
  }
}

export async function clearStoredDeviceBootstrap(): Promise<void> {
  if (typeof indexedDB !== "undefined") {
    try {
      const db = await getDeviceDb();
      await db.delete(DEVICE_STORE_NAME, INSTALLATION_ID_KEY);
    } catch {
      // ignore cleanup failures
    }
  }

  if (typeof window !== "undefined") {
    try {
      window.localStorage.removeItem(INSTALLATION_ID_KEY);
    } catch {
      // ignore cleanup failures
    }
  }
}
