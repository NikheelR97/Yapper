import "fake-indexeddb/auto";

import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/plugins/tauri-compat.js", () => ({
  platform: () => "web",
}));

import {
  clearStoredDeviceBootstrap,
  getDeviceBootstrap,
} from "./bootstrap.js";

const INSTALLATION_ID_KEY = "yapper_installation_id";

describe("getDeviceBootstrap", () => {
  beforeEach(async () => {
    await clearStoredDeviceBootstrap();
    window.localStorage.clear();
  });

  it("persists installation identity outside localStorage", async () => {
    const first = await getDeviceBootstrap();
    const second = await getDeviceBootstrap();

    expect(first.installation_id).toBe(second.installation_id);
    expect(first.platform).toBe("web");
    expect(window.localStorage.getItem(INSTALLATION_ID_KEY)).toBeNull();
  });

  it("migrates legacy localStorage installation ids into IndexedDB", async () => {
    window.localStorage.setItem(INSTALLATION_ID_KEY, "legacy-installation-id");

    const bootstrap = await getDeviceBootstrap();
    const repeated = await getDeviceBootstrap();

    expect(bootstrap.installation_id).toBe("legacy-installation-id");
    expect(repeated.installation_id).toBe("legacy-installation-id");
    expect(window.localStorage.getItem(INSTALLATION_ID_KEY)).toBeNull();
  });

  it("fails closed when IndexedDB is unavailable", async () => {
    const originalIndexedDb = globalThis.indexedDB;
    vi.stubGlobal("indexedDB", undefined);

    await expect(getDeviceBootstrap()).rejects.toThrow(
      /secure device storage/i,
    );

    vi.stubGlobal("indexedDB", originalIndexedDb);
  });
});
