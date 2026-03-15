<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { authStore, clearAuth } from "$stores/auth.js";
	import { api } from "$api/client.js";
	import { toast } from "$stores/toast.js";
	import { get } from "svelte/store";
	import { clearCurrentSignalStore } from "$signal/keystore.js";
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
		| "support";

	let activeSection: Section = "profile";
	let showDeleteConfirm = false;
	let deleting = false;
	let devicesLoading = false;
	let devices: Array<{
		id: string;
		label: string;
		platform: string;
		trust_state: "trusted" | "pending_trust" | "revoked";
		last_seen_at: string | null;
	}> = [];
	let revokingDeviceId: string | null = null;
	const BASE_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";

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
				await clearCurrentSignalStore().catch(() => {});
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

			const res = await fetch(`${BASE_URL}/api/v1/account/data-export`, {
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

	async function deleteAccount() {
		deleting = true;
		try {
			await api.delete("/api/v1/account");
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
						<span class="family-info-icon">🛡</span>
						<div>
							<strong>Managed Account</strong>
							<p>This account is managed by a parent. Some features may require parental approval.</p>
						</div>
					</div>
				{:else}
					<div class="family-explainer">
						<div class="family-explainer-item">
							<span>🛡</span>
							<div>
								<strong>Approve Friend Requests</strong>
								<p>Review who your child connects with before friendships are established.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>🌐</span>
							<div>
								<strong>Approve Server Joins</strong>
								<p>Your child's server join requests come to you first.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>⏱</span>
							<div>
								<strong>Screen Time</strong>
								<p>Set daily limits and view usage reports.</p>
							</div>
						</div>
						<div class="family-explainer-item">
							<span>🔒</span>
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
		{/if}
	</main>

	<!-- Right sidebar -->
	<aside class="settings-sidebar">
		<!-- Account actions -->
		<div class="sidebar-card">
			<h3 class="sidebar-card-title">Account</h3>
			<div class="sidebar-actions">
				<button class="action-btn" on:click={exportData}>
					📦 Export My Data
				</button>
				<button class="action-btn" on:click={logout}>
					🚪 Log Out
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
				<button class="danger-btn"> ⏸ Disable Account </button>
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
						🗑 Delete Account
					</button>
				{/if}
			</div>
		</div>

		<!-- GoPro promo (if not premium) -->
		{#if !isPremium}
			<div class="sidebar-card pro-card">
				<div class="pro-icon">🚀</div>
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
		background: var(--color-bg-base, #0a0a0f);
	}

	/* Left nav */
	.settings-nav {
		width: 220px;
		background: #0f1117;
		border-right: 1px solid rgba(255, 255, 255, 0.06);
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
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		margin-bottom: 8px;
	}

	.nav-title {
		font-size: 16px;
		font-weight: 800;
		color: #f9fafb;
	}

	.nav-version {
		font-size: 11px;
		color: #4b5563;
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
		background: rgba(255, 255, 255, 0.04);
	}

	.nav-item.active {
		background: rgba(124, 58, 237, 0.12);
	}

	.nav-label {
		font-size: 14px;
		color: #d1d5db;
	}

	.nav-item.active .nav-label {
		color: #a78bfa;
		font-weight: 600;
	}

	.nav-badge {
		font-size: 10px;
		font-weight: 700;
		padding: 2px 7px;
		border-radius: 20px;
		background: rgba(124, 58, 237, 0.2);
		color: #a78bfa;
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
		border-left: 1px solid rgba(255, 255, 255, 0.06);
		flex-shrink: 0;
	}

	.sidebar-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.07);
		border-radius: 14px;
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.sidebar-card-title {
		font-size: 12px;
		font-weight: 700;
		color: #9ca3af;
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
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	.device-name {
		font-size: 13px;
		font-weight: 600;
		color: #f3f4f6;
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.device-current {
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #a78bfa;
	}

	.device-meta {
		font-size: 11px;
		color: #9ca3af;
		margin-top: 4px;
	}

	.device-action {
		padding: 8px 10px;
		border-radius: 8px;
		border: 1px solid rgba(239, 68, 68, 0.25);
		background: rgba(239, 68, 68, 0.08);
		color: #fca5a5;
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
		color: #9ca3af;
	}

	.action-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		color: #d1d5db;
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: background 100ms;
	}

	.action-btn:hover {
		background: rgba(255, 255, 255, 0.08);
	}

	/* Danger zone */
	.danger-card {
		border-color: rgba(239, 68, 68, 0.2);
		background: rgba(239, 68, 68, 0.03);
	}

	.danger-title {
		color: #ef4444;
	}

	.danger-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		background: rgba(239, 68, 68, 0.06);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: 8px;
		color: #ef4444;
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
		border-radius: 8px;
		color: #ef4444;
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
		color: #ef4444;
		margin: 0;
	}

	.delete-confirm-btn {
		padding: 8px;
		background: #ef4444;
		color: white;
		border: none;
		border-radius: 6px;
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
		color: #6b7280;
		font-size: 12px;
		cursor: pointer;
		text-align: center;
		padding: 4px;
	}

	/* GoPro promo */
	.pro-card {
		background: linear-gradient(
			135deg,
			rgba(124, 58, 237, 0.12),
			rgba(219, 39, 119, 0.05)
		);
		border-color: rgba(124, 58, 237, 0.25);
		align-items: center;
		text-align: center;
	}

	.pro-icon {
		font-size: 32px;
	}

	.pro-title {
		font-size: 16px;
		font-weight: 800;
		color: #f9fafb;
		margin: 0;
	}

	.pro-desc {
		font-size: 13px;
		color: #9ca3af;
		margin: 0;
		line-height: 1.5;
	}

	.pro-btn {
		padding: 10px 20px;
		background: linear-gradient(135deg, #7c3aed, #db2777);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.pro-btn:hover {
		opacity: 0.85;
	}

	@media (max-width: 900px) {
		.settings-sidebar {
			display: none;
		}
	}

	@media (max-width: 600px) {
		.settings-nav {
			width: 56px;
		}

		.nav-label,
		.nav-badge,
		.nav-header {
			display: none;
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
		border-radius: 8px;
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
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-text-primary);
		border: 1px solid rgba(255, 255, 255, 0.1);
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
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
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
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.07);
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
		color: var(--color-text-muted, #6b7280);
		font-size: 0.8125rem;
		line-height: 1.4;
	}
</style>

