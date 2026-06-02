<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { authStore, clearAuth } from "$stores/auth.js";
	import { api } from "$api/client.js";
	import { toast } from "$stores/toast.js";
	import { get } from "svelte/store";
	import { clearCurrentSignalStore } from "$signal/keystore.js";
	import { API_URL } from "$lib/env.js";
	import ProfileForm from "$components/settings/ProfileForm.svelte";
	import PrivacySafety from "$components/settings/PrivacySafety.svelte";
	import Appearance from "$components/settings/Appearance.svelte";
	import VoiceVideo from "$components/settings/VoiceVideo.svelte";
	import Notifications from "$components/settings/Notifications.svelte";
	import Premium from "$components/settings/Premium.svelte";
	import DiscordImport from "$components/settings/DiscordImport.svelte";
	import DeveloperTools from "$components/settings/DeveloperTools.svelte";
	import ChangePassword from "$components/settings/ChangePassword.svelte";
	import Support from "$components/settings/Support.svelte";
	import AppUpdater from "$components/settings/AppUpdater.svelte";
	import { isTauri } from "$lib/plugins/tauri-compat.js";

	type Section =
		| "profile"
		| "privacy"
		| "password"
		| "appearance"
		| "voice"
		| "notifications"
		| "premium"
		| "connections"
		| "family"
		| "developer"
		| "support"
		| "about";

	let activeSection: Section = "profile";
	let showDeleteConfirm = false;
	let deleting = false;
	let showDisableConfirm = false;
	let disabling = false;
	let devicesLoading = false;
	let devices: Array<{
		id: string;
		label: string;
		platform: string;
		trust_state: "trusted" | "pending_trust" | "revoked";
		last_seen_at: string | null;
	}> = [];
	let revokingDeviceId: string | null = null;
	const BASE_URL = API_URL;

	$: user = $authStore.user;
	$: currentDevice = $authStore.device;
	$: isPremium = user?.isPremium ?? false;

	const navItems: { id: Section; label: string; badge?: string }[] = [
		{ id: "profile", label: "My Profile" },
		{ id: "privacy", label: "Privacy & Safety" },
		{ id: "password", label: "Change Password" },
		{ id: "appearance", label: "Appearance" },
		{ id: "voice", label: "Voice & Video" },
		{ id: "notifications", label: "Notifications" },
		{
			id: "premium",
			label: "Yapper Premium",
			badge: isPremium ? undefined : "NEW",
		},
		{ id: "connections", label: "Connected Accounts" },
		{ id: "family", label: "Family Controls" },
		{ id: "developer", label: "For Developers" },
		{ id: "support", label: "Support" },
		...(isTauri() ? [{ id: "about" as Section, label: "About" }] : []),
	];

	async function logout() {
		try {
			await api.delete("/api/v2/auth/logout");
		} catch {
			// Best-effort server-side session revocation — always clear local auth
		}
		clearAuth();
		await goto("/login");
	}

	async function loadDevices() {
		if (!get(authStore).accessToken) return;
		devicesLoading = true;
		try {
			devices = await api.get<
				Array<{
					id: string;
					label: string;
					platform: string;
					trust_state: "trusted" | "pending_trust" | "revoked";
					last_seen_at: string | null;
				}>
			>("/api/v2/devices");
		} catch {
			devices = [];
		} finally {
			devicesLoading = false;
		}
	}

	async function revokeDevice(deviceId: string) {
		if (revokingDeviceId === deviceId) return;
		revokingDeviceId = deviceId;
		try {
			await api.delete(`/api/v2/devices/${deviceId}`);
			if (deviceId === currentDevice?.id) {
				await clearCurrentSignalStore().catch((err) => {
					console.warn("[signal] Failed to clear store on device revoke:", err);
				});
				clearAuth();
				await goto("/login");
				return;
			}
			toast.success("Device revoked.");
			await loadDevices();
		} catch (e: any) {
			toast.error(e.message ?? "Failed to revoke device");
		} finally {
			revokingDeviceId = null;
		}
	}

	async function exportData() {
		try {
			const { accessToken } = get(authStore);
			if (!accessToken) {
				throw new Error("You are not logged in.");
			}

			const res = await fetch(`${BASE_URL}/api/v2/account/data-export`, {
				method: "GET",
				headers: { Authorization: `Bearer ${accessToken}` },
				credentials: "include",
			});

			if (!res.ok) {
				const body = await res.json().catch(() => ({
					error: "Failed to export data",
				}));
				throw new Error(body.error ?? "Failed to export data");
			}

			const blob = await res.blob();
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			a.href = url;
			a.download = `yapper-data-export-${new Date().toISOString().slice(0, 10)}.zip`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);

			toast.success("Data export downloaded.");
		} catch (e: any) {
			toast.error(e.message ?? "Failed to export data");
		}
	}

	async function disableAccount() {
		disabling = true;
		try {
			await api.post("/api/v2/account/disable");
			clearAuth();
			await goto("/login");
		} catch (e: any) {
			toast.error(e.message ?? "Failed to disable account");
			disabling = false;
		}
	}

	async function deleteAccount() {
		deleting = true;
		try {
			await api.delete("/api/v2/account");
			clearAuth();
			await goto("/login");
		} catch (e: any) {
			toast.error(e.message ?? "Failed to delete account");
			deleting = false;
		}
	}

	onMount(() => {
		void loadDevices(); // fire-and-forget: errors surfaced via devices=[]
	});
</script>

<svelte:head>
	<title>Settings — Yapper</title>
</svelte:head>

<div class="settings-page">
	<!-- Left nav -->
	<nav class="settings-nav">
		<div class="nav-header">
			<span class="nav-title">Settings</span>
			<span class="nav-version">v2.4.0</span>
		</div>

		<div class="nav-items">
			{#each navItems as item}
				<button
					class="nav-item"
					class:active={activeSection === item.id}
					on:click={() => (activeSection = item.id)}
				>
					<span class="nav-label">{item.label}</span>
					{#if item.badge}
						<span class="nav-badge">{item.badge}</span>
					{/if}
				</button>
			{/each}
		</div>
	</nav>

	<!-- Main content -->
	<main class="settings-main">
		{#if activeSection === "profile"}
			<ProfileForm />
		{:else if activeSection === "privacy"}
			<PrivacySafety />
		{:else if activeSection === "password"}
			<ChangePassword />
		{:else if activeSection === "appearance"}
			<Appearance />
		{:else if activeSection === "voice"}
			<VoiceVideo />
		{:else if activeSection === "notifications"}
			<Notifications />
		{:else if activeSection === "premium"}
			<Premium />
		{:else if activeSection === "connections"}
			<DiscordImport />
		{:else if activeSection === "family"}
			<div class="family-section">
				<h2 class="section-heading">Family Controls</h2>
				<p class="section-desc">
					Manage child accounts and review their activity. You see metadata only — message content is always end-to-end encrypted.
				</p>

				{#if user?.accountType === 'parent'}
					<a class="family-btn family-btn-primary" href="/parent/dashboard">
						Open Parent Dashboard →
					</a>
					<a class="family-btn family-btn-secondary" href="/parent/children/setup">
						+ Add Another Child Account
					</a>
				{:else if user?.accountType === 'child'}
					<div class="family-info-card">
						<span class="family-info-icon">
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
								<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
							</svg>
						</span>
						<div>
							<strong>Managed Account</strong>
							<p>This account is managed by a parent. Some features may require parental approval.</p>
						</div>
					</div>
				{:else}
					<div class="family-explainer">
						<div class="family-explainer-item">
							<span>
								<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
								</svg>
							</span>
							<div>
								<strong>Approve Friend Requests</strong>
								<p>Review who your child connects with before friendships are established.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>
								<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<circle cx="12" cy="12" r="10" /><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" /><path d="M2 12h20" />
								</svg>
							</span>
							<div>
								<strong>Approve Server Joins</strong>
								<p>Your child's server join requests come to you first.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>
								<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<circle cx="12" cy="12" r="10" /><polyline points="12 6 12 12 16 14" />
								</svg>
							</span>
							<div>
								<strong>Screen Time</strong>
								<p>Set daily limits and view usage reports.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>
								<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<rect width="18" height="11" x="3" y="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" />
								</svg>
							</span>
							<div>
								<strong>E2EE Preserved</strong>
								<p>Message content is never visible — only activity metadata.</p>
							</div>
						</div>
					</div>
					<a class="family-btn family-btn-primary" href="/parent/children/setup">
						Set Up Family Controls →
					</a>
				{/if}
			</div>
		{:else if activeSection === "developer"}
			<DeveloperTools />
		{:else if activeSection === "support"}
			<Support />
		{:else if activeSection === "about"}
			<AppUpdater />
		{/if}
	</main>

	<!-- Right sidebar -->
	<aside class="settings-sidebar">
		<!-- Account actions -->
		<div class="sidebar-card">
			<h3 class="sidebar-card-title">Account</h3>
			<div class="sidebar-actions">
				<button class="action-btn" on:click={exportData}>
					<svg class="btn-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="m7.5 4.27 9 5.15" /><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" /><path d="M3.3 7 12 12l8.7-5" /><path d="M12 22V12" />
					</svg>
					Export My Data
				</button>
				<button class="action-btn" on:click={logout}>
					<svg class="btn-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><polyline points="16 17 21 12 16 7" /><line x1="21" x2="9" y1="12" y2="12" />
					</svg>
					Log Out
				</button>
			</div>
		</div>

		<div class="sidebar-card">
			<h3 class="sidebar-card-title">Devices</h3>
			<div class="device-list">
				{#if devicesLoading}
					<p class="device-empty">Loading devices...</p>
				{:else if devices.length === 0}
					<p class="device-empty">No registered devices found.</p>
				{:else}
					{#each devices as device}
						<div class="device-item">
							<div>
								<div class="device-name">
									{device.label}
									{#if device.id === currentDevice?.id}
										<span class="device-current">This device</span>
									{/if}
								</div>
								<div class="device-meta">
									{device.platform} · {device.trust_state}
									{#if device.last_seen_at}
										· Last seen {new Date(device.last_seen_at).toLocaleString()}
									{/if}
								</div>
							</div>
							{#if device.id === currentDevice?.id || currentDevice?.trustState === "trusted"}
								<button
									class="device-action"
									on:click={() => revokeDevice(device.id)}
									disabled={revokingDeviceId === device.id}
								>
									{#if revokingDeviceId === device.id}
										Working...
									{:else if device.id === currentDevice?.id}
										Forget
									{:else}
										Revoke
									{/if}
								</button>
							{/if}
						</div>
					{/each}
				{/if}
			</div>
		</div>

		<!-- Danger zone -->
		<div class="sidebar-card danger-card">
			<h3 class="sidebar-card-title danger-title">Danger Zone</h3>
			<div class="sidebar-actions">
				{#if showDisableConfirm}
					<div class="delete-confirm">
						<p class="delete-warn">
							Your account will be deactivated and you'll be signed out everywhere. Signing back in reactivates it — nothing is deleted.
						</p>
						<button
							class="delete-confirm-btn"
							on:click={disableAccount}
							disabled={disabling}
						>
							{disabling ? "Disabling…" : "Disable my account"}
						</button>
						<button
							class="cancel-link"
							on:click={() => (showDisableConfirm = false)}
						>
							Cancel
						</button>
					</div>
				{:else}
					<button class="danger-btn" on:click={() => (showDisableConfirm = true)}>
						<svg class="btn-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<rect x="14" y="4" width="4" height="16" rx="1" /><rect x="6" y="4" width="4" height="16" rx="1" />
						</svg>
						Disable Account
					</button>
				{/if}
				{#if showDeleteConfirm}
					<div class="delete-confirm">
						<p class="delete-warn">
							This is permanent and cannot be undone.
						</p>
						<button
							class="delete-confirm-btn"
							on:click={deleteAccount}
							disabled={deleting}
						>
							{deleting ? "Deleting…" : "Yes, delete my account"}
						</button>
						<button
							class="cancel-link"
							on:click={() => (showDeleteConfirm = false)}
						>
							Cancel
						</button>
					</div>
				{:else}
					<button
						class="delete-btn"
						on:click={() => (showDeleteConfirm = true)}
					>
						<svg class="btn-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M3 6h18" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" /><line x1="10" x2="10" y1="11" y2="17" /><line x1="14" x2="14" y1="11" y2="17" />
						</svg>
						Delete Account
					</button>
				{/if}
			</div>
		</div>

		<!-- GoPro promo (if not premium) -->
		{#if !isPremium}
			<div class="sidebar-card pro-card">
				<div class="pro-icon">
						<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
							<path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z" /><path d="m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z" /><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0" /><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5" />
						</svg>
					</div>
				<h3 class="pro-title">Go GoPro</h3>
				<p class="pro-desc">
					Animated avatars, 100 custom emojis, HD clips and more.
				</p>
				<button
					class="pro-btn"
					on:click={() => (activeSection = "premium")}
				>
					Learn More →
				</button>
			</div>
		{/if}
	</aside>
</div>

<style>
	.settings-page {
		display: flex;
		flex: 1;
		height: 100%;
		overflow: hidden;
		background: var(--color-bg-base);
	}

	/* Left nav */
	.settings-nav {
		width: 220px;
		background: var(--color-bg-surface);
		border-right: 1px solid var(--color-border);
		padding: 20px 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		overflow-y: auto;
		flex-shrink: 0;
	}

	.nav-header {
		display: flex;
		align-items: baseline;
		gap: 6px;
		padding: 0 16px 16px;
		border-bottom: 1px solid var(--color-border);
		margin-bottom: 8px;
	}

	.nav-title {
		font-size: 16px;
		font-weight: 800;
		color: var(--color-text-primary);
	}

	.nav-version {
		font-size: 11px;
		color: var(--color-text-muted);
	}

	.nav-items {
		padding: 0 8px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 9px 12px;
		border-radius: 8px;
		background: none;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: background 100ms;
		width: 100%;
	}

	.nav-item:hover {
		background: var(--color-bg-elevated);
	}

	.nav-item.active {
		background: rgba(124, 58, 237, 0.12);
	}

	.nav-label {
		font-size: 14px;
		color: var(--color-text-secondary);
	}

	.nav-item.active .nav-label {
		color: var(--color-brand-light);
		font-weight: 600;
	}

	.nav-badge {
		font-size: 10px;
		font-weight: 700;
		padding: 2px 7px;
		border-radius: 20px;
		background: rgba(124, 58, 237, 0.2);
		color: var(--color-brand-light);
	}

	/* Main content */
	.settings-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px;
		min-width: 0;
	}

	/* Right sidebar */
	.settings-sidebar {
		width: 280px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		overflow-y: auto;
		border-left: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.sidebar-card {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 14px;
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.sidebar-card-title {
		font-size: 12px;
		font-weight: 700;
		color: var(--color-text-secondary);
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.sidebar-actions {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.device-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.device-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 10px;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
	}

	.device-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--color-text-primary);
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.device-current {
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-brand-light);
	}

	.device-meta {
		font-size: 11px;
		color: var(--color-text-secondary);
		margin-top: 4px;
	}

	.device-action {
		padding: 8px 10px;
		border-radius: var(--radius-full);
		border: 1px solid rgba(239, 68, 68, 0.25);
		background: rgba(239, 68, 68, 0.08);
		color: var(--color-error-text);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
	}

	.device-action:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.device-empty {
		margin: 0;
		font-size: 12px;
		color: var(--color-text-secondary);
	}

	.btn-icon {
		flex-shrink: 0;
	}

	.action-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		color: var(--color-text-secondary);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: background 100ms;
	}

	.action-btn:hover {
		background: var(--color-bg-elevated);
	}

	/* Danger zone */
	.danger-card {
		border-color: rgba(239, 68, 68, 0.2);
		background: rgba(239, 68, 68, 0.03);
	}

	.danger-title {
		color: var(--color-error);
	}

	.danger-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: rgba(239, 68, 68, 0.06);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: var(--radius-full);
		color: var(--color-error);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		width: 100%;
		text-align: left;
		transition: background 100ms;
	}

	.danger-btn:hover {
		background: rgba(239, 68, 68, 0.12);
	}

	.delete-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: var(--radius-full);
		color: var(--color-error);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		width: 100%;
		text-align: left;
		transition: background 100ms;
	}

	.delete-btn:hover {
		background: rgba(239, 68, 68, 0.2);
	}

	.delete-confirm {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		background: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: 8px;
	}

	.delete-warn {
		font-size: 12px;
		color: var(--color-error);
		margin: 0;
	}

	.delete-confirm-btn {
		padding: 8px;
		background: var(--color-error);
		color: white;
		border: none;
		border-radius: var(--radius-full);
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		width: 100%;
		transition: opacity 150ms;
	}

	.delete-confirm-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.delete-confirm-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.cancel-link {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 12px;
		cursor: pointer;
		text-align: center;
		padding: 4px;
	}

	/* GoPro promo */
	.pro-card {
		background: linear-gradient(
			135deg,
			rgba(124, 58, 237, 0.14),
			rgba(124, 58, 237, 0.04)
		);
		border-color: rgba(124, 58, 237, 0.25);
		align-items: center;
		text-align: center;
	}

	.pro-icon {
		color: var(--color-brand);
		line-height: 0;
	}

	.pro-title {
		font-size: 16px;
		font-weight: 800;
		color: var(--color-text-primary);
		margin: 0;
	}

	.pro-desc {
		font-size: 13px;
		color: var(--color-text-secondary);
		margin: 0;
		line-height: 1.5;
	}

	.pro-btn {
		padding: 10px 20px;
		background: var(--color-brand);
		color: white;
		border: none;
		border-radius: var(--radius-full);
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.pro-btn:hover {
		opacity: 0.85;
	}

	/* Below 900px the three-column shell stacks into one scrollable page so
	   account / device / danger actions stay reachable (they used to be
	   display:none, stranding logout, device revoke, export and delete on
	   mobile — a primary Capacitor platform). */
	@media (max-width: 900px) {
		.settings-page {
			flex-direction: column;
			overflow-y: auto;
		}

		.settings-nav {
			width: 100%;
			flex-direction: row;
			align-items: center;
			gap: 4px;
			padding: 8px;
			overflow-x: auto;
			overflow-y: visible;
			border-right: none;
			border-bottom: 1px solid var(--color-border);
			position: sticky;
			top: 0;
			z-index: 1;
		}

		.nav-header {
			display: none;
		}

		.nav-items {
			flex-direction: row;
			flex-wrap: nowrap;
			padding: 0;
		}

		.nav-item {
			width: auto;
			white-space: nowrap;
			flex-shrink: 0;
		}

		.settings-main {
			overflow-y: visible;
			padding: 20px;
		}

		.settings-sidebar {
			width: 100%;
			flex-direction: column;
			overflow-y: visible;
			border-left: none;
			border-top: 1px solid var(--color-border);
		}
	}

	/* ── Family section ── */
	.family-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		max-width: 520px;
	}

	.section-heading {
		font-size: 1.1rem;
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
	}

	.section-desc {
		font-size: 0.875rem;
		color: var(--color-text-secondary, #9ca3af);
		margin: 0;
		line-height: 1.5;
	}

	.family-btn {
		display: inline-block;
		padding: 0.65rem 1.25rem;
		border-radius: var(--radius-full);
		font-size: 0.875rem;
		font-weight: 600;
		text-decoration: none;
		transition: opacity 150ms;
	}

	.family-btn-primary {
		background: var(--color-brand, #7c3aed);
		color: white;
	}

	.family-btn-secondary {
		background: var(--color-bg-surface);
		color: var(--color-text-primary);
		border: 1px solid var(--color-border);
	}

	.family-btn:hover {
		opacity: 0.85;
		text-decoration: none;
	}

	.family-info-card {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		padding: 1rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		font-size: 0.875rem;
		color: var(--color-text-secondary, #9ca3af);
	}

	.family-info-icon {
		font-size: 1.25rem;
		flex-shrink: 0;
	}

	.family-info-card strong {
		display: block;
		color: var(--color-text-primary);
		margin-bottom: 0.2rem;
	}

	.family-info-card p {
		margin: 0;
	}

	.family-explainer {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.family-explainer-item {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		padding: 0.875rem 1rem;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 10px;
		font-size: 0.875rem;
	}

	.family-explainer-item span {
		font-size: 1.1rem;
		flex-shrink: 0;
		margin-top: 1px;
	}

	.family-explainer-item strong {
		display: block;
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
		margin-bottom: 0.2rem;
	}

	.family-explainer-item p {
		margin: 0;
		color: var(--color-text-secondary);
		font-size: 0.8125rem;
		line-height: 1.4;
	}
</style>
