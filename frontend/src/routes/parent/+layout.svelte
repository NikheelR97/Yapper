<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import DesktopVaultGate from '$lib/components/device/DesktopVaultGate.svelte';
	import {
		authStore,
		setAuth,
		storeRefreshToken,
		syncNativeRefreshTokenToSecureStorage,
	} from '$stores/auth.js';
	import {
		parentalStore,
		loadChildren,
		loadAlerts,
		selectChild,
	} from '$stores/parental.js';
	import { page } from '$app/stores';
	import { api } from '$api/client.js';
	import { normalizeServerDevice } from '$lib/device/bootstrap.js';
	import { toast } from '$stores/toast.js';
	import {
		desktopSignalVaultExists,
		desktopVaultSupported,
		isDesktopVaultUnlocked,
		unlockDesktopVault,
	} from '$lib/desktop/vault.js';
	import type { User } from '$stores/auth.js';

	let ready = false;
	let desktopVaultMode: 'setup' | 'unlock' | null = null;
	let desktopVaultBusy = false;
	let desktopVaultError: string | null = null;

	$: isSetupPage = $page.url.pathname === '/parent/children/setup';

	async function refreshSession(): Promise<boolean> {
		try {
			const res = await api.post<{
				access_token: string;
				csrf_token: string;
				refresh_token?: string;
				user: User;
				device: {
					id: string;
					signal_device_id: number;
					installation_id: string | null;
					platform: 'web' | 'tauri' | 'capacitor';
					label: string;
					trust_state: 'trusted' | 'pending_trust' | 'revoked';
					created_at: string;
					last_seen_at: string | null;
					approved_at: string | null;
					revoked_at: string | null;
				};
			}>('/api/v2/auth/refresh');
			const storageWarning = res.refresh_token
				? await storeRefreshToken(res.refresh_token)
				: null;
			setAuth(res.user, res.access_token, res.csrf_token, normalizeServerDevice(res.device));
			if (storageWarning) {
				toast.warning(storageWarning, 0);
			}
			return true;
		} catch {
			return false;
		}
	}

	onMount(async () => {
		if (!(await ensureDesktopVaultReady())) {
			return;
		}

		let state = get(authStore);
		if (!state.user) {
			const restored = await refreshSession();
			if (!restored) {
				await goto('/login');
				return;
			}
			state = get(authStore);
		}
		// Allow any logged-in user to access the setup wizard (creates their parent account)
		if (state.user?.accountType !== 'parent') {
			if (!get(page).url.pathname.startsWith('/parent/children/setup')) {
				await goto('/');
				return;
			}
			ready = true;
			return;
		}
		await loadChildren();
		await loadAlerts();
		ready = true;
	});

	async function ensureDesktopVaultReady(): Promise<boolean> {
		if (!desktopVaultSupported() || isDesktopVaultUnlocked()) {
			return true;
		}

		desktopVaultMode = (await desktopSignalVaultExists()) ? 'unlock' : 'setup';
		return false;
	}

	async function submitDesktopVaultPassphrase(passphrase: string): Promise<void> {
		desktopVaultBusy = true;
		desktopVaultError = null;
		try {
			await unlockDesktopVault(passphrase);
			await syncNativeRefreshTokenToSecureStorage();
			desktopVaultMode = null;
			let state = get(authStore);
			if (!state.user) {
				const restored = await refreshSession();
				if (!restored) {
					await goto('/login');
					return;
				}
				state = get(authStore);
			}

			if (state.user?.accountType !== 'parent' && !get(page).url.pathname.startsWith('/parent/children/setup')) {
				await goto('/');
				return;
			}

			if (state.user?.accountType === 'parent') {
				await loadChildren();
				await loadAlerts();
			}
			ready = true;
		} catch (e) {
			desktopVaultError =
				e instanceof Error && e.message.trim()
					? e.message
					: 'Unable to unlock secure vault';
		} finally {
			desktopVaultBusy = false;
		}
	}

	$: children = $parentalStore.children;
	$: selectedId = $parentalStore.selectedChildId;
	$: user = $authStore.user;

	async function handleSelectChild(id: string) {
		selectChild(id);
		await goto('/parent/dashboard');
	}
</script>

{#if desktopVaultMode}
	<DesktopVaultGate
		mode={desktopVaultMode}
		busy={desktopVaultBusy}
		error={desktopVaultError}
		onSubmit={submitDesktopVaultPassphrase}
	/>
{:else if ready}
	{#if isSetupPage}
		<slot />
	{:else}
	<div class="parent-shell">
		<!-- Top nav -->
		<nav class="top-nav">
			<div class="nav-left">
				<div class="brand">
					<div class="sphere-icon"></div>
					<span class="brand-name">Yapper</span>
				</div>
				<span class="page-label">Parent Dashboard</span>
			</div>
			<div class="nav-right">
				<div class="user-chip">
					<div class="user-avatar">
						{user?.displayName?.[0]?.toUpperCase() ?? 'P'}
					</div>
					<span class="user-name">{user?.displayName ?? 'Parent'}</span>
				</div>
			</div>
		</nav>

		<div class="parent-body">
			<!-- Left sidebar -->
			<aside class="sidebar">
				<div class="sidebar-section">
					<div class="sidebar-label">Managed Accounts</div>
					{#each children as child}
						<button
							class="child-row"
							class:active={child.id === selectedId}
							on:click={() => handleSelectChild(child.id)}
						>
							<div class="child-avatar">
								{#if child.avatar_url}
									<img src={child.avatar_url} alt={child.display_name} />
								{:else}
									<div class="avatar-placeholder">{child.display_name[0]}</div>
								{/if}
								<span class="status-dot"></span>
							</div>
							<span class="child-name">{child.display_name}</span>
						</button>
					{/each}

					<a class="add-child-btn" href="/parent/children/setup">
						+ Add Account
					</a>
				</div>

				<div class="sidebar-section">
					<div class="sidebar-label">Controls</div>
					<a
						class="nav-link"
						class:active={$page.url.pathname === '/parent/dashboard'}
						href="/parent/dashboard"
					>
						🛡 Safety Dashboard
					</a>
					<a
						class="nav-link"
						class:active={$page.url.pathname === '/parent/screen-time'}
						href="/parent/screen-time"
					>
						⏱ Screen Time
					</a>
				</div>
			</aside>

			<!-- Main content -->
			<main class="main-content">
				<slot />
			</main>
		</div>
	</div>
	{/if}
{:else}
	<div class="loading-shell">
		<div class="loader"></div>
	</div>
{/if}

<style>
	.parent-shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
		height: 100dvh;
		background: var(--color-bg-base, #0a0a0f);
		overflow: hidden;
	}

	/* Top nav */
	.top-nav {
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 20px;
		background: #0f1117;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		flex-shrink: 0;
	}

	.nav-left {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.sphere-icon {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #c4b5fd, #7c3aed 45%, #2e1065);
	}

	.brand-name {
		font-size: 16px;
		font-weight: 800;
		color: #f9fafb;
	}

	.page-label {
		font-size: 13px;
		color: #6b7280;
		padding-left: 16px;
		border-left: 1px solid rgba(255, 255, 255, 0.1);
	}

	.nav-right {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.user-chip {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.user-avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		color: white;
		font-size: 13px;
	}

	.user-name {
		font-size: 14px;
		color: #f9fafb;
	}

	/* Body layout */
	.parent-body {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	/* Sidebar */
	.sidebar {
		width: 220px;
		background: #0f1117;
		border-right: 1px solid rgba(255, 255, 255, 0.06);
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow-y: auto;
		flex-shrink: 0;
	}

	.sidebar-section {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-bottom: 16px;
	}

	.sidebar-label {
		font-size: 11px;
		font-weight: 700;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		padding: 0 8px;
		margin-bottom: 4px;
	}

	.child-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border-radius: 8px;
		background: none;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: background 100ms;
		width: 100%;
	}

	.child-row:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.child-row.active {
		background: rgba(124, 58, 237, 0.12);
	}

	.child-avatar {
		position: relative;
		width: 32px;
		height: 32px;
		flex-shrink: 0;
	}

	.child-avatar img,
	.avatar-placeholder {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		object-fit: cover;
	}

	.avatar-placeholder {
		background: linear-gradient(135deg, #7c3aed, #2e1065);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		color: white;
		font-size: 13px;
	}

	.status-dot {
		position: absolute;
		bottom: 0;
		right: 0;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #4b5563;
		border: 2px solid #0f1117;
	}

	.child-name {
		font-size: 14px;
		color: #f9fafb;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.add-child-btn {
		display: flex;
		align-items: center;
		padding: 8px;
		border-radius: 8px;
		color: #7c3aed;
		font-size: 13px;
		font-weight: 600;
		text-decoration: none;
		transition: background 100ms;
	}

	.add-child-btn:hover {
		background: rgba(124, 58, 237, 0.1);
	}

	.nav-link {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px;
		border-radius: 8px;
		color: #9ca3af;
		font-size: 14px;
		text-decoration: none;
		transition: background 100ms, color 100ms;
	}

	.nav-link:hover {
		background: rgba(255, 255, 255, 0.04);
		color: #f9fafb;
	}

	.nav-link.active {
		background: rgba(124, 58, 237, 0.12);
		color: #a78bfa;
	}

	/* Main */
	.main-content {
		flex: 1;
		overflow-y: auto;
		min-width: 0;
	}

	/* Loading */
	.loading-shell {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background: var(--color-bg-base, #0a0a0f);
	}

	.loader {
		width: 32px;
		height: 32px;
		border: 3px solid rgba(255, 255, 255, 0.1);
		border-top-color: #7c3aed;
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
