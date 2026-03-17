<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import Toast from "$components/Toast.svelte";
  import TitleBar from "$components/TitleBar.svelte";
  import AppLoadingScreen from "$components/AppLoadingScreen.svelte";
  import KeyboardShortcutsModal from "$components/KeyboardShortcutsModal.svelte";
  import DeviceApprovalInbox from "$lib/components/device/DeviceApprovalInbox.svelte";
  import DesktopVaultGate from "$lib/components/device/DesktopVaultGate.svelte";
  import PendingDeviceGate from "$lib/components/device/PendingDeviceGate.svelte";
  import {
    authStore,
    clearAuth,
    setAuth,
    type AuthDevice,
  } from "$stores/auth.js";
  import { ApiError, api } from "$api/client.js";
  import type { User } from "$stores/auth.js";
  import { toast } from "$stores/toast.js";
  import { onWsMessage, wsConnect, wsDisconnect, wsStore } from "$stores/ws.js";
  import {
    setupKeys,
    replenishPreKeysIfNeeded,
    restoreKeys,
  } from "$signal/index.js";
  import {
    clearCurrentSignalStore,
    configureSignalStore,
    resetSignalStoreScope,
  } from "$signal/keystore.js";
  import {
    registerDmHandler,
    fetchConversations,
  } from "$stores/conversations.js";
  import {
    registerChannelHandler,
    registerEmojiHandler,
    fetchServers,
  } from "$stores/servers.js";
  import { registerPresenceHandler } from "$stores/presence.js";
  import { registerCanvasHandler } from "$stores/canvas.js";
  import {
    reportScreenTimeUsage,
    startYapperUsageTracker,
  } from "$plugins/screentime.js";
  import {
    getDeviceBootstrap,
    normalizeServerDevice,
  } from "$lib/device/bootstrap.js";
  import {
    clearPendingDeviceSyncRequest,
    decryptDeviceSyncChunk,
    encryptDeviceSyncChunk,
    ensurePendingDeviceSyncRequest,
    exportTrustedDeviceSnapshot,
    hasPendingDeviceSyncRequest,
    importTrustedDeviceSnapshot,
  } from "$lib/device/sync.js";

  import { initUpdater } from "$lib/desktop/updater.js";
  import { initDeepLinks } from "$lib/desktop/deepLinks.js";
  import {
    desktopSignalVaultExists,
    desktopVaultSupported,
    isDesktopVaultUnlocked,
    unlockDesktopVault,
  } from "$lib/desktop/vault.js";
  import {
    requestNotificationPermission,
    initDesktopNotifications,
  } from "$lib/desktop/notifications.js";
  import UpdateBanner from "$components/desktop/UpdateBanner.svelte";
  import TopNav from "$lib/components/nav/TopNav.svelte";
  import AppSidebar from "$lib/components/nav/AppSidebar.svelte";

  let ready = false;
  let showShortcuts = false;
  let unregisterDmHandler: (() => void) | null = null;
  let unregisterChannelHandler: (() => void) | null = null;
  let unregisterPresenceHandler: (() => void) | null = null;
  let unregisterCanvasHandler: (() => void) | null = null;
  let unregisterEmojiHandler: (() => void) | null = null;
  let unregisterDeviceSyncHandler: (() => void) | null = null;
  let unregisterWsErrorHandler: (() => void) | null = null;
  let stopScreenTimeTracker: (() => void) | null = null;
  let servicesStarted = false;
  let pendingStatusRefresh = false;
  let pendingRestoreBusy = false;
  let pendingDevices: AuthDevice[] = [];
  let restoreDevices: AuthDevice[] = [];
  let devicePollTimer: ReturnType<typeof setInterval> | null = null;
  let approvingDeviceIds = new Set<string>();
  let desktopVaultMode: "setup" | "unlock" | null = null;
  let desktopVaultBusy = false;
  let desktopVaultError: string | null = null;
  let secureStoreError: string | null = null;
  let bootSequenceStarted = false;
  const BASE_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";
  const DEVICE_SYNC_CHUNK_SIZE = 24 * 1024;
  const trustedSyncPublicKeys = new Map<string, string>();
  const APPROVED_UNSYNCED_KEY = "yapper_approved_unsynced_devices";
  function loadApprovedUnsyncedDeviceIds(): Set<string> {
    if (typeof window === "undefined") return new Set();
    try {
      const raw = window.localStorage.getItem(APPROVED_UNSYNCED_KEY);
      if (!raw) return new Set();
      const arr = JSON.parse(raw);
      if (Array.isArray(arr))
        return new Set<string>(
          arr.filter((x): x is string => typeof x === "string"),
        );
    } catch {
      /* ignore */
    }
    return new Set();
  }
  function saveApprovedUnsyncedDeviceIds(ids: Set<string>): void {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(
      APPROVED_UNSYNCED_KEY,
      JSON.stringify([...ids]),
    );
  }
  const approvedUnsyncedDeviceIds = loadApprovedUnsyncedDeviceIds();
  const incomingSyncTransfers = new Map<
    string,
    { total: number; chunks: string[] }
  >();
  const seenDeviceSyncEventIds = new Set<string>();
  let lastPendingTrustRequestAt = 0;

  interface DeviceSyncEvent {
    id: string;
    event_type:
      | "device_trust_request"
      | "device_trust_approved"
      | "device_sync_chunk"
      | "device_sync_complete"
      | "device_revoked";
    source_device_id: string | null;
    payload: Record<string, unknown>;
    created_at: string;
  }

  function currentDeviceTrusted(): boolean {
    return get(authStore).device?.trustState === "trusted";
  }

  function setPendingRestoreDevices(
    devices: Array<{
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
    }>,
    currentDeviceId: string | null | undefined,
  ) {
    restoreDevices = devices
      .filter(
        (device) =>
          device.id !== currentDeviceId && device.trust_state === "trusted",
      )
      .map((device) => normalizeServerDevice(device));
  }

  async function configureActiveSignalStore() {
    const state = get(authStore);
    if (!state.user || !state.device) return;
    await configureSignalStore(state.user.id, state.device.id);
  }

  function formatSecureStoreError(error: unknown): string {
    if (error instanceof Error && error.message.trim()) {
      return error.message;
    }
    return "Secure encryption storage could not be initialized on this device.";
  }

  async function enterSecureStoreFailureState(error: unknown): Promise<void> {
    console.error("[Signal] Secure keystore initialization failed:", error);
    stopDevicePolling();
    stopAppServices();
    resetSignalStoreScope();
    secureStoreError = formatSecureStoreError(error);
    ready = true;
    bootSequenceStarted = false;
    toast.error("Secure encryption storage is unavailable on this device.");
  }

  async function retrySecureStoreSetup(): Promise<void> {
    secureStoreError = null;
    ready = false;
    await bootAppLayout();
  }

  async function signOutAfterSecureStoreFailure(): Promise<void> {
    stopDevicePolling();
    stopAppServices();
    resetSignalStoreScope();
    secureStoreError = null;
    clearAuth();
    await goto("/login");
  }

  async function initializeSignalKeys() {
    try {
      await configureActiveSignalStore();
    } catch (error) {
      await enterSecureStoreFailureState(error);
      return;
    }
    try {
      await setupKeys();
    } catch (e) {
      console.error("[Signal] Key setup failed:", e);
      toast.error("Encryption setup failed - try refreshing.");
      return;
    }

    try {
      await replenishPreKeysIfNeeded();
    } catch (e) {
      console.warn(
        "[Signal] PreKey replenish failed; will retry on next app start:",
        e,
      );
    }
  }

  function splitSyncSnapshot(snapshot: Uint8Array): Uint8Array[] {
    if (snapshot.length === 0) {
      return [new Uint8Array()];
    }

    const chunks: Uint8Array[] = [];
    for (
      let offset = 0;
      offset < snapshot.length;
      offset += DEVICE_SYNC_CHUNK_SIZE
    ) {
      chunks.push(snapshot.slice(offset, offset + DEVICE_SYNC_CHUNK_SIZE));
    }
    return chunks;
  }

  async function fetchDeviceSyncEvents(): Promise<DeviceSyncEvent[]> {
    try {
      return await api.get<DeviceSyncEvent[]>("/api/v2/devices/sync-events");
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        await recoverAfterDeviceApiUnauthorized();
      }
      return [];
    }
  }

  function dedupeDeviceSyncEvents(
    events: DeviceSyncEvent[],
  ): DeviceSyncEvent[] {
    return events.filter((event) => {
      if (seenDeviceSyncEventIds.has(event.id)) {
        return false;
      }
      seenDeviceSyncEventIds.add(event.id);
      return true;
    });
  }

  async function queuePendingTrustRequest(): Promise<void> {
    const state = get(authStore);
    if (!state.device || state.device.trustState !== "pending_trust") {
      return;
    }

    const now = Date.now();
    if (now - lastPendingTrustRequestAt < 15_000) {
      return;
    }
    lastPendingTrustRequestAt = now;

    const syncRequest = ensurePendingDeviceSyncRequest(state.device.id);
    await api
      .post("/api/v2/devices/trust-requests", {
        target_device_id: state.device.id,
        sync_public_key: syncRequest.publicKey,
      })
      .catch(() => {});
  }

  async function syncTrustedSnapshotToDevice(
    targetDeviceId: string,
  ): Promise<boolean> {
    const syncPublicKey = trustedSyncPublicKeys.get(targetDeviceId);
    if (!syncPublicKey) {
      return false;
    }

    const snapshot = await exportTrustedDeviceSnapshot();
    const snapshotBytes = new TextEncoder().encode(snapshot);
    const chunks = splitSyncSnapshot(snapshotBytes);
    const transferId = crypto.randomUUID();

    for (let index = 0; index < chunks.length; index += 1) {
      const encryptedChunk = await encryptDeviceSyncChunk(
        chunks[index],
        syncPublicKey,
      );
      await api.post("/api/v2/devices/sync-events", {
        target_device_id: targetDeviceId,
        event_type: "device_sync_chunk",
        payload: {
          transfer_id: transferId,
          index,
          total: chunks.length,
          ciphertext: encryptedChunk.ciphertext,
          ek_public: encryptedChunk.ekPublic,
        },
      });
    }

    await api.post("/api/v2/devices/sync-events", {
      target_device_id: targetDeviceId,
      event_type: "device_sync_complete",
      payload: {
        transfer_id: transferId,
        total: chunks.length,
      },
    });

    trustedSyncPublicKeys.delete(targetDeviceId);
    approvedUnsyncedDeviceIds.delete(targetDeviceId);
    saveApprovedUnsyncedDeviceIds(approvedUnsyncedDeviceIds);
    return true;
  }

  async function processTrustedSyncEvents(): Promise<void> {
    const events = dedupeDeviceSyncEvents(await fetchDeviceSyncEvents());
    for (const event of events) {
      await handleTrustedRealtimeSyncEvent(event);
    }
  }

  async function processPendingSyncEvents(
    events: DeviceSyncEvent[],
    deviceId: string,
  ): Promise<{ trustApproved: boolean; importedSnapshot: boolean }> {
    let trustApproved = false;
    let importedSnapshot = false;

    const chunkEvents = events.filter(
      (event) => event.event_type === "device_sync_chunk",
    );
    for (const event of chunkEvents) {
      const transferId =
        typeof event.payload.transfer_id === "string"
          ? event.payload.transfer_id
          : null;
      const ciphertext =
        typeof event.payload.ciphertext === "string"
          ? event.payload.ciphertext
          : null;
      const ekPublic =
        typeof event.payload.ek_public === "string"
          ? event.payload.ek_public
          : null;
      const index =
        typeof event.payload.index === "number"
          ? event.payload.index
          : Number.NaN;
      const total =
        typeof event.payload.total === "number"
          ? event.payload.total
          : Number.NaN;
      if (
        !transferId ||
        !ciphertext ||
        !ekPublic ||
        !Number.isInteger(index) ||
        !Number.isInteger(total)
      ) {
        continue;
      }

      try {
        const plaintext = await decryptDeviceSyncChunk(
          deviceId,
          ciphertext,
          ekPublic,
        );
        const transfer = incomingSyncTransfers.get(transferId) ?? {
          total,
          chunks: Array.from({ length: total }, () => ""),
        };
        transfer.total = total;
        transfer.chunks[index] = new TextDecoder().decode(plaintext);
        incomingSyncTransfers.set(transferId, transfer);
      } catch (e) {
        console.warn("[DeviceSync] Failed to decrypt sync chunk:", e);
      }
    }

    for (const event of events) {
      if (event.event_type === "device_trust_approved") {
        trustApproved = true;
        continue;
      }

      if (event.event_type !== "device_sync_complete") {
        continue;
      }

      const transferId =
        typeof event.payload.transfer_id === "string"
          ? event.payload.transfer_id
          : null;
      const total =
        typeof event.payload.total === "number"
          ? event.payload.total
          : Number.NaN;
      if (!transferId || !Number.isInteger(total)) {
        continue;
      }

      const transfer = incomingSyncTransfers.get(transferId);
      if (
        !transfer ||
        transfer.total !== total ||
        transfer.chunks.some((chunk) => !chunk)
      ) {
        continue;
      }

      await importTrustedDeviceSnapshot(transfer.chunks.join(""));
      incomingSyncTransfers.delete(transferId);
      clearPendingDeviceSyncRequest(deviceId);
      importedSnapshot = true;
    }

    return { trustApproved, importedSnapshot };
  }

  async function handleTrustedRealtimeSyncEvent(
    event: DeviceSyncEvent,
  ): Promise<void> {
    if (event.event_type === "device_trust_request") {
      const targetDeviceId =
        typeof event.payload.target_device_id === "string"
          ? event.payload.target_device_id
          : null;
      const syncPublicKey =
        typeof event.payload.sync_public_key === "string"
          ? event.payload.sync_public_key
          : null;
      if (!targetDeviceId || !syncPublicKey) {
        return;
      }

      trustedSyncPublicKeys.set(targetDeviceId, syncPublicKey);
      if (approvedUnsyncedDeviceIds.has(targetDeviceId)) {
        try {
          const synced = await syncTrustedSnapshotToDevice(targetDeviceId);
          if (synced) {
            toast.success("Encrypted history sync sent to approved device.");
          }
        } catch (e: any) {
          console.warn("[DeviceSync] Failed to replay trusted sync:", e);
        }
      }
      return;
    }

    if (event.event_type === "device_trust_approved") {
      const targetDeviceId =
        typeof event.payload.target_device_id === "string"
          ? event.payload.target_device_id
          : null;
      if (!targetDeviceId) {
        return;
      }

      approvedUnsyncedDeviceIds.add(targetDeviceId);
      saveApprovedUnsyncedDeviceIds(approvedUnsyncedDeviceIds);
      if (trustedSyncPublicKeys.has(targetDeviceId)) {
        try {
          const synced = await syncTrustedSnapshotToDevice(targetDeviceId);
          if (synced) {
            toast.success("Encrypted history sync sent to approved device.");
          }
        } catch (e: any) {
          console.warn("[DeviceSync] Failed to send sync after approval:", e);
        }
      }
      return;
    }

    if (event.event_type === "device_revoked") {
      const revokedDeviceId =
        typeof event.payload.device_id === "string"
          ? event.payload.device_id
          : null;
      if (!revokedDeviceId) {
        return;
      }
      trustedSyncPublicKeys.delete(revokedDeviceId);
      approvedUnsyncedDeviceIds.delete(revokedDeviceId);
      saveApprovedUnsyncedDeviceIds(approvedUnsyncedDeviceIds);
    }
  }

  async function maybeActivateTrustedDevice(
    deviceId: string,
    importedSnapshot: boolean,
  ): Promise<boolean> {
    const restored = await refreshSessionV2();
    if (!restored || !currentDeviceTrusted()) {
      return false;
    }

    if (!importedSnapshot && hasPendingDeviceSyncRequest(deviceId)) {
      return false;
    }

    clearPendingDeviceSyncRequest(deviceId);
    try {
      await configureActiveSignalStore();
    } catch (error) {
      await enterSecureStoreFailureState(error);
      return false;
    }
    await loadPendingApprovals();
    await processTrustedSyncEvents();
    startTrustedDevicePolling();
    void initializeSignalKeys();
    startAppServices();
    return true;
  }

  async function handleCurrentDeviceRevoked(): Promise<void> {
    const deviceId = get(authStore).device?.id;
    stopDevicePolling();
    stopAppServices();
    if (deviceId) {
      clearPendingDeviceSyncRequest(deviceId);
    }
    await clearCurrentSignalStore().catch(() => {});
    clearAuth();
    ready = true;
    toast.error("This device was revoked. Sign in again to continue.");
    await goto("/login");
  }

  async function recoverAfterDeviceApiUnauthorized(): Promise<boolean> {
    const restored = await refreshSessionV2();
    if (!restored) {
      stopDevicePolling();
      stopAppServices();
      clearAuth();
      ready = true;
      toast.error("Session expired. Please sign in again.");
      await goto("/login");
      return false;
    }

    if (get(authStore).device?.trustState === "revoked") {
      await handleCurrentDeviceRevoked();
      return false;
    }

    return true;
  }

  async function handleRealtimeDeviceSyncEvent(
    event: DeviceSyncEvent,
  ): Promise<void> {
    const [nextEvent] = dedupeDeviceSyncEvents([event]);
    if (!nextEvent) {
      return;
    }

    const currentDeviceId = get(authStore).device?.id;
    const revokedDeviceId =
      nextEvent.event_type === "device_revoked" &&
      typeof nextEvent.payload.device_id === "string"
        ? nextEvent.payload.device_id
        : null;
    if (currentDeviceId && revokedDeviceId === currentDeviceId) {
      await handleCurrentDeviceRevoked();
      return;
    }

    if (currentDeviceTrusted()) {
      await handleTrustedRealtimeSyncEvent(nextEvent);
      return;
    }

    const deviceId = get(authStore).device?.id;
    if (!deviceId) {
      return;
    }

    const { trustApproved, importedSnapshot } = await processPendingSyncEvents(
      [nextEvent],
      deviceId,
    );
    if (trustApproved || importedSnapshot) {
      await maybeActivateTrustedDevice(deviceId, importedSnapshot);
    }
  }

  function startAppServices() {
    if (servicesStarted) return;
    servicesStarted = true;

    wsConnect();
    unregisterDmHandler = registerDmHandler();
    unregisterChannelHandler = registerChannelHandler();
    unregisterPresenceHandler = registerPresenceHandler();
    unregisterCanvasHandler = registerCanvasHandler();
    unregisterEmojiHandler = registerEmojiHandler();
    fetchConversations().catch(() => {});
    fetchServers().catch(() => {});

    stopScreenTimeTracker = startYapperUsageTracker();
    reportScreenTimeUsage().catch(() => {});

    initUpdater();
    initDeepLinks();
    requestNotificationPermission();
    initDesktopNotifications();
  }

  function stopAppServices() {
    unregisterDmHandler?.();
    unregisterDmHandler = null;
    unregisterChannelHandler?.();
    unregisterChannelHandler = null;
    unregisterPresenceHandler?.();
    unregisterPresenceHandler = null;
    unregisterCanvasHandler?.();
    unregisterCanvasHandler = null;
    unregisterEmojiHandler?.();
    unregisterEmojiHandler = null;
    stopScreenTimeTracker?.();
    stopScreenTimeTracker = null;
    reportScreenTimeUsage().catch(() => {});
    wsDisconnect();
    servicesStarted = false;
  }

  async function refreshSessionV2(): Promise<boolean> {
    try {
      const res = await api.post<{
        access_token: string;
        csrf_token: string;
        user: User;
        device: {
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
        };
      }>("/api/v2/auth/refresh");
      setAuth(
        res.user,
        res.access_token,
        res.csrf_token,
        normalizeServerDevice(res.device),
      );
      return true;
    } catch {
      return false;
    }
  }

  async function attachCurrentSessionDevice(): Promise<boolean> {
    const state = get(authStore);
    if (!state.user || !state.accessToken) {
      return false;
    }

    try {
      const res = await fetch(`${BASE_URL}/api/v2/auth/attach-device`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${state.accessToken}`,
          "Content-Type": "application/json",
          ...(state.csrfToken ? { "X-CSRF-Token": state.csrfToken } : {}),
        },
        body: JSON.stringify(await getDeviceBootstrap()),
        credentials: "include",
      });
      if (!res.ok) {
        return false;
      }

      const body = await res.json();
      setAuth(
        body.user,
        body.access_token,
        body.csrf_token,
        normalizeServerDevice(body.device),
      );
      return true;
    } catch {
      return false;
    }
  }

  async function loadPendingApprovals() {
    if (!currentDeviceTrusted()) {
      pendingDevices = [];
      return;
    }
    let devices: Array<{
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
    }> = [];
    try {
      devices = await api.get("/api/v2/devices");
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        await recoverAfterDeviceApiUnauthorized();
      }
    }
    pendingDevices = devices
      .filter((device) => device.trust_state === "pending_trust")
      .map((device) => normalizeServerDevice(device));
  }

  function stopDevicePolling() {
    if (devicePollTimer) {
      clearInterval(devicePollTimer);
      devicePollTimer = null;
    }
  }

  function startTrustedDevicePolling() {
    stopDevicePolling();
    devicePollTimer = setInterval(() => {
      void loadPendingApprovals();
      void processTrustedSyncEvents();
    }, 15000);
  }

  function startPendingDevicePolling() {
    stopDevicePolling();
    devicePollTimer = setInterval(() => {
      void refreshPendingDeviceStatus();
    }, 5000);
  }

  async function refreshPendingDeviceStatus() {
    pendingStatusRefresh = true;
    try {
      const state = get(authStore);
      if (state.device) {
        await queuePendingTrustRequest();
        const syncEvents = dedupeDeviceSyncEvents(
          await fetchDeviceSyncEvents(),
        );
        const { trustApproved, importedSnapshot } =
          await processPendingSyncEvents(syncEvents, state.device.id);
        if (trustApproved || importedSnapshot) {
          if (
            await maybeActivateTrustedDevice(state.device.id, importedSnapshot)
          ) {
            return;
          }
        }
      }

      let devices: Array<{
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
      }> = [];
      try {
        devices = await api.get("/api/v2/devices");
      } catch (error) {
        if (error instanceof ApiError && error.status === 401) {
          await recoverAfterDeviceApiUnauthorized();
          return;
        }
      }
      const nextState = get(authStore);
      const current = devices.find(
        (device) => device.id === nextState.device?.id,
      );
      setPendingRestoreDevices(devices, nextState.device?.id);
      if (nextState.user && nextState.accessToken && current) {
        setAuth(
          nextState.user,
          nextState.accessToken,
          nextState.csrfToken,
          normalizeServerDevice(current),
        );
      }
      if (current?.trust_state === "trusted") {
        if (await maybeActivateTrustedDevice(current.id, false)) {
          return;
        }
      }
    } finally {
      pendingStatusRefresh = false;
    }
  }

  async function approvePendingDevice(deviceId: string) {
    if (approvingDeviceIds.has(deviceId)) return;
    approvingDeviceIds = new Set(approvingDeviceIds).add(deviceId);
    try {
      await processTrustedSyncEvents();
      await api.post(`/api/v2/devices/${deviceId}/approve`);
      await loadPendingApprovals();

      try {
        const synced = await syncTrustedSnapshotToDevice(deviceId);
        if (synced) {
          toast.success("Device approved. Encrypted history sync sent.");
        } else {
          approvedUnsyncedDeviceIds.add(deviceId);
          saveApprovedUnsyncedDeviceIds(approvedUnsyncedDeviceIds);
          toast.success(
            "Device approved. Waiting for the other device to refresh.",
          );
        }
      } catch (e: any) {
        approvedUnsyncedDeviceIds.add(deviceId);
        saveApprovedUnsyncedDeviceIds(approvedUnsyncedDeviceIds);
        toast.error(
          e.message ?? "Device approved, but encrypted history sync failed",
        );
      }
    } catch (e: any) {
      toast.error(e.message ?? "Failed to approve device");
    } finally {
      const next = new Set(approvingDeviceIds);
      next.delete(deviceId);
      approvingDeviceIds = next;
    }
  }

  async function restorePendingDeviceFromBackup(
    sourceDeviceId: string,
    pin: string,
  ): Promise<void> {
    const deviceId = get(authStore).device?.id;
    if (!deviceId) {
      toast.error("No active device session.");
      return;
    }
    if (!sourceDeviceId || !pin.trim()) {
      toast.error("Choose a source device and enter the backup PIN.");
      return;
    }

    pendingRestoreBusy = true;
    try {
      const restored = await restoreKeys(pin, sourceDeviceId);
      if (!restored) {
        toast.error("No encrypted backup found for that device.");
        return;
      }

      if (await maybeActivateTrustedDevice(deviceId, true)) {
        toast.success("Encrypted backup restored. This device is now trusted.");
        return;
      }

      toast.error("Backup restored locally, but device activation failed.");
    } catch (e: any) {
      toast.error(e.message ?? "Failed to restore encrypted backup");
    } finally {
      pendingRestoreBusy = false;
    }
  }

  async function ensureDesktopVaultReady(): Promise<boolean> {
    if (!desktopVaultSupported() || isDesktopVaultUnlocked()) {
      return true;
    }

    desktopVaultMode = (await desktopSignalVaultExists()) ? "unlock" : "setup";
    return false;
  }

  async function submitDesktopVaultPassphrase(
    passphrase: string,
  ): Promise<void> {
    desktopVaultBusy = true;
    desktopVaultError = null;
    try {
      await unlockDesktopVault(passphrase);
      desktopVaultMode = null;
      await bootAppLayout();
    } catch (e: any) {
      desktopVaultError = e.message ?? "Unable to unlock secure vault";
    } finally {
      desktopVaultBusy = false;
    }
  }

  async function bootAppLayout(): Promise<void> {
    if (bootSequenceStarted) {
      return;
    }
    bootSequenceStarted = true;

    try {
      const state = get(authStore);

      if (state.user && !state.device) {
        await attachCurrentSessionDevice();
      }

      if (!get(authStore).user || !get(authStore).device) {
        const restored = await refreshSessionV2();
        if (!restored) {
          toast.error("Session expired. Please sign in again.");
          clearAuth();
          await goto("/login");
          return;
        }
      }

      try {
        await configureActiveSignalStore();
      } catch (error) {
        await enterSecureStoreFailureState(error);
        return;
      }
      unregisterDeviceSyncHandler = onWsMessage(
        "device_sync_event",
        (payload) => {
          void handleRealtimeDeviceSyncEvent(payload as DeviceSyncEvent);
        },
      );
      unregisterWsErrorHandler = onWsMessage("ws_error", (payload) => {
        const frame = payload as { code?: number; message?: string };
        if (frame.code === 4001 && frame.message === "Device revoked") {
          void handleCurrentDeviceRevoked();
        }
      });
      wsConnect();
      if (currentDeviceTrusted()) {
        startAppServices();
        ready = true;
        // Run in background — none of these affect the initial render.
        // initializeSignalKeys is the slow one on first-ever sign-in:
        // generates 100 OPKs + 3 sequential HTTP uploads (~2–5 s).
        // On subsequent logins it's a fast no-op (bootstrapComplete flag).
        void processTrustedSyncEvents().finally(() =>
          startTrustedDevicePolling(),
        );
        void initializeSignalKeys();
        void loadPendingApprovals();
        return;
      } else {
        await refreshPendingDeviceStatus();
        if (!currentDeviceTrusted()) {
          startPendingDevicePolling();
        }
      }

      ready = true;
    } catch (error) {
      bootSequenceStarted = false;
      throw error;
    }
  }

  onMount(async () => {
    if (!(await ensureDesktopVaultReady())) {
      return;
    }

    await bootAppLayout();
  });

  onDestroy(() => {
    stopDevicePolling();
    unregisterDeviceSyncHandler?.();
    unregisterDeviceSyncHandler = null;
    unregisterWsErrorHandler?.();
    unregisterWsErrorHandler = null;
    stopAppServices();
    resetSignalStoreScope();
  });

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "/") {
      e.preventDefault();
      showShortcuts = !showShortcuts;
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="layout-root">
  <TitleBar />
  <UpdateBanner />
  <TopNav />

  {#if ready && currentDeviceTrusted() && !$wsStore.connected}
    <div class="reconnecting-banner">Reconnecting...</div>
  {/if}

  {#if desktopVaultMode}
    <DesktopVaultGate
      mode={desktopVaultMode}
      busy={desktopVaultBusy}
      error={desktopVaultError}
      onSubmit={submitDesktopVaultPassphrase}
    />
  {:else if ready}
    {#if secureStoreError}
      <div class="secure-store-error" role="alert">
        <h1>Secure Storage Unavailable</h1>
        <p>{secureStoreError}</p>
        <div class="secure-store-actions">
          <button type="button" on:click={() => void retrySecureStoreSetup()}
            >Retry</button
          >
          <button
            type="button"
            class="secondary"
            on:click={() => void signOutAfterSecureStoreFailure()}
          >
            Sign Out
          </button>
        </div>
      </div>
    {:else if currentDeviceTrusted()}
      <div class="app-shell">
        <AppSidebar />
        <main class="main-content">
          <DeviceApprovalInbox
            {pendingDevices}
            approving={approvingDeviceIds}
            onApprove={approvePendingDevice}
          />
          <slot />
        </main>
      </div>
    {:else}
      <PendingDeviceGate
        label={$authStore.device?.label ?? "This device"}
        refreshing={pendingStatusRefresh}
        {restoreDevices}
        restoring={pendingRestoreBusy}
        onRefresh={refreshPendingDeviceStatus}
        onRestore={restorePendingDeviceFromBackup}
      />
    {/if}
  {:else}
    <AppLoadingScreen />
  {/if}
</div>

<Toast />
<KeyboardShortcutsModal bind:open={showShortcuts} />

<style>
  .layout-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    height: 100dvh;
    overflow: hidden;
  }

  .app-shell {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .reconnecting-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    background: #b45309;
    color: #fff;
    text-align: center;
    padding: 4px;
    font-size: 12px;
  }

  .secure-store-error {
    margin: auto;
    max-width: 32rem;
    padding: 2rem;
    border: 1px solid var(--color-border);
    border-radius: 1rem;
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .secure-store-error h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  .secure-store-error p {
    margin: 0;
    line-height: 1.5;
    color: var(--color-text-secondary);
  }

  .secure-store-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .secure-store-actions button {
    border: 0;
    border-radius: 999px;
    padding: 0.75rem 1.25rem;
    font: inherit;
    cursor: pointer;
    background: var(--color-brand);
    color: white;
  }

  .secure-store-actions button.secondary {
    background: var(--color-bg-muted);
    color: var(--color-text-primary);
  }
</style>
