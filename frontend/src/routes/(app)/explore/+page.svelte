<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		exploreStore,
		loadTrendingTags,
		loadLiveServers,
		loadCommunities,
		search,
		joinServer,
	} from '$stores/explore.js';
	import TrendingTags from '$lib/components/explore/TrendingTags.svelte';
	import LiveServerCard from '$lib/components/explore/LiveServerCard.svelte';
	import CommunityCard from '$lib/components/explore/CommunityCard.svelte';
	import { fetchServers } from '$stores/servers.js';
	import { api } from '$api/client.js';
	import { toast } from '$stores/toast.js';

	let searchInput = '';
	let searchTimer: ReturnType<typeof setTimeout> | null = null;
	let activeTag: string | null = null;
	let viewMode: 'grid' | 'list' = 'grid';
	let joinError = '';
	let joiningId = '';
	let sentRequests = new Set<string>();

	$: state = $exploreStore;
	$: isSearching = searchInput.trim().length > 0;

	// Filter communities + live servers by active tag
	$: filteredCommunities = activeTag
		? state.communities.filter((c) => c.tags.includes(activeTag!))
		: state.communities;

	$: filteredLive = activeTag
		? state.liveServers.filter((s) => s.tags.includes(activeTag!))
		: state.liveServers;

	onMount(() => {
		loadTrendingTags();
		loadLiveServers();
		loadCommunities();
	});

	function handleSearchInput() {
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			search(searchInput);
		}, 350);
	}

	function handleTagSelect(e: CustomEvent<string | null>) {
		activeTag = e.detail;
		searchInput = '';
	}

	async function handleAddFriend(username: string) {
		try {
			await api.post(`/api/v2/users/by/${encodeURIComponent(username)}/friend-request`, {});
			sentRequests = new Set([...sentRequests, username]);
			toast.success('Friend request sent!');
		} catch (e: any) {
			toast.error(e.message ?? 'Failed to send friend request');
		}
	}

	async function handleJoin(id: string) {
		joinError = '';
		joiningId = id;
		try {
			await joinServer(id);
			await fetchServers();
			goto(`/servers/${id}/channels`);
		} catch (e) {
			const err = e as { status?: number };
			if (err.status === 409) {
				// Already a member — navigate directly since we have the id
				goto(`/servers/${id}/channels`);
			} else {
				joinError = 'Could not join server. Try again.';
			}
		} finally {
			joiningId = '';
		}
	}
</script>

<svelte:head>
	<title>Explore — Yapper</title>
</svelte:head>

<div class="explore-page">
	<!-- Sidebar with server list is rendered by parent layout -->

	<main class="explore-main">
		<!-- Search bar -->
		<header class="explore-header">
			<div class="search-wrap">
				<svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<circle cx="11" cy="11" r="8"/>
					<line x1="21" y1="21" x2="16.65" y2="16.65"/>
				</svg>
				<input
					class="search-input"
					type="search"
					placeholder="Search servers and people…"
					bind:value={searchInput}
					on:input={handleSearchInput}
					aria-label="Search"
				/>
				{#if state.searching}
					<span class="search-spinner" aria-label="Searching…"></span>
				{/if}
			</div>

			<div class="view-toggle" role="group" aria-label="View mode">
				<button
					class="toggle-btn"
					class:active={viewMode === 'grid'}
					on:click={() => (viewMode = 'grid')}
					title="Grid view"
					aria-pressed={viewMode === 'grid'}
				>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/>
						<rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>
					</svg>
				</button>
				<button
					class="toggle-btn"
					class:active={viewMode === 'list'}
					on:click={() => (viewMode = 'list')}
					title="List view"
					aria-pressed={viewMode === 'list'}
				>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
						<line x1="8" y1="6" x2="21" y2="6"/>
						<line x1="8" y1="12" x2="21" y2="12"/>
						<line x1="8" y1="18" x2="21" y2="18"/>
						<line x1="3" y1="6" x2="3.01" y2="6"/>
						<line x1="3" y1="12" x2="3.01" y2="12"/>
						<line x1="3" y1="18" x2="3.01" y2="18"/>
					</svg>
				</button>
			</div>
		</header>

		<div class="explore-body">
			{#if joinError}
				<div class="join-error">{joinError}</div>
			{/if}

			<!-- Search results -->
			{#if isSearching}
				<section class="results-section" data-testid="search-results">
					{#if state.searchServers.length > 0}
						<h2 class="section-title">Servers</h2>
						<div class="cards-grid" class:list-mode={viewMode === 'list'}>
							{#each state.searchServers as s}
								<CommunityCard
									server={s}
									on:join={(e) => handleJoin(e.detail)}
								/>
							{/each}
						</div>
					{/if}

					{#if state.searchUsers.length > 0}
						<h2 class="section-title">People</h2>
						<div class="user-list">
							{#each state.searchUsers as u}
								<div class="user-row">
									<button class="user-avatar-btn" on:click={() => goto(`/profile/${u.username}`)}>
										<div class="user-avatar">
											{#if u.avatar_url}
												<img src={u.avatar_url} alt={u.username} />
											{:else}
												<span>{u.username.charAt(0).toUpperCase()}</span>
											{/if}
										</div>
									</button>
									<div class="user-info">
										<button class="user-name-btn" on:click={() => goto(`/profile/${u.username}`)}>
											<span class="display-name">{u.display_name ?? u.username}</span>
										</button>
										<span class="username">@{u.username}</span>
									</div>
									<div class="user-actions">
										{#if u.is_friend}
											<span class="friend-badge">Friends</span>
										{:else if sentRequests.has(u.username) || u.request_sent}
											<span class="pending-badge">Request Sent</span>
										{:else if u.friend_request_permission !== 'nobody'}
											<button class="btn-add-friend" on:click={() => handleAddFriend(u.username)}>
												+ Add
											</button>
										{/if}
									</div>
								</div>
							{/each}
						</div>
					{/if}

					{#if !state.searching && state.searchServers.length === 0 && state.searchUsers.length === 0}
						<p class="no-results">No results for "{searchInput}"</p>
					{/if}
				</section>

			<!-- Default explore content -->
			{:else}
				<!-- Trending tags -->
				{#if state.tags.length > 0}
					<section class="section">
						<h2 class="section-title">Trending Tags</h2>
						<TrendingTags
							tags={state.tags}
							{activeTag}
							on:select={handleTagSelect}
						/>
					</section>
				{/if}

				<!-- Live servers -->
				{#if filteredLive.length > 0}
					<section class="section">
						<h2 class="section-title">
							<span class="live-indicator" aria-hidden="true"></span>
							Live Now
						</h2>
						<div class="cards-grid live-grid" class:list-mode={viewMode === 'list'}>
							{#each filteredLive as s}
								<LiveServerCard
									server={s}
									on:join={(e) => handleJoin(e.detail)}
								/>
							{/each}
						</div>
					</section>
				{/if}

				<!-- Communities -->
				<section class="section">
					<h2 class="section-title">Communities</h2>
					{#if state.loadingCommunities}
						<div class="loading-row">
							{#each [1, 2, 3, 4, 5, 6] as _}
								<div class="skeleton-card"></div>
							{/each}
						</div>
					{:else if filteredCommunities.length === 0}
						<p class="empty-msg">
							{activeTag ? `No public servers tagged #${activeTag}.` : 'No public servers yet. Be the first!'}
						</p>
					{:else}
						<div class="cards-grid" class:list-mode={viewMode === 'list'}>
							{#each filteredCommunities as c}
								<CommunityCard
									server={c}
									on:join={(e) => handleJoin(e.detail)}
								/>
							{/each}
						</div>
					{/if}
				</section>
			{/if}
		</div>
	</main>
</div>

<style>
	.explore-page {
		display: flex;
		height: 100%;
		overflow: hidden;
		background: var(--color-bg-base);
	}

	.explore-main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	/* ── Header ── */
	.explore-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1.25rem;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg-elevated);
		flex-shrink: 0;
	}

	.search-wrap {
		flex: 1;
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 0.625rem;
		color: var(--color-text-muted);
		pointer-events: none;
	}

	.search-input {
		width: 100%;
		padding: 0.45rem 2rem 0.45rem 2.125rem;
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border);
		background: var(--color-bg-surface);
		color: var(--color-text-primary);
		font-size: 0.875rem;
		outline: none;
		transition: border-color var(--transition-fast);
	}

	.search-input:focus {
		border-color: var(--color-brand);
	}

	.search-input::-webkit-search-cancel-button { display: none; }

	.search-spinner {
		position: absolute;
		right: 0.625rem;
		width: 14px;
		height: 14px;
		border: 2px solid var(--color-border);
		border-top-color: var(--color-brand);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }

	.view-toggle {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.toggle-btn {
		padding: 0.4rem 0.6rem;
		border: none;
		background: transparent;
		color: var(--color-text-muted);
		cursor: pointer;
		transition: background var(--transition-fast), color var(--transition-fast);
		display: flex;
		align-items: center;
	}

	.toggle-btn.active {
		background: var(--color-brand);
		color: #fff;
	}

	/* ── Body ── */
	.explore-body {
		flex: 1;
		overflow-y: auto;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1.75rem;
		scrollbar-width: thin;
		scrollbar-color: var(--color-border) transparent;
	}

	.join-error {
		padding: 0.6rem 1rem;
		border-radius: var(--radius-md);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--color-error-text);
		font-size: 0.8125rem;
	}

	.section, .results-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.section-title {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-text-muted);
		margin: 0;
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.live-indicator {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-error);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50%       { opacity: 0.3; }
	}

	/* ── Grid / List ── */
	.cards-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 0.75rem;
	}

	.cards-grid.list-mode {
		grid-template-columns: 1fr;
	}

	.live-grid {
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
	}

	/* ── Skeletons ── */
	.loading-row {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 0.75rem;
	}

	.skeleton-card {
		height: 140px;
		border-radius: var(--radius-lg);
		background: var(--color-bg-elevated);
		animation: shimmer 1.4s ease-in-out infinite;
	}

	@keyframes shimmer {
		0%   { opacity: 0.5; }
		50%  { opacity: 1; }
		100% { opacity: 0.5; }
	}

	/* ── User search results ── */
	.user-list {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.user-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.625rem 0.875rem;
		border-radius: var(--radius-md);
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
	}

	.user-avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		overflow: hidden;
		background: rgba(124, 58, 237, 0.2);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		font-weight: 700;
		color: var(--color-brand-light);
		flex-shrink: 0;
	}

	.user-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.user-avatar-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		flex-shrink: 0;
	}

	.user-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.user-name-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		text-align: left;
	}

	.user-name-btn:hover .display-name {
		text-decoration: underline;
	}

	.display-name {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.username {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.user-actions {
		flex-shrink: 0;
		display: flex;
		align-items: center;
	}

	.btn-add-friend {
		background: var(--color-brand, #7c3aed);
		color: white;
		border: none;
		border-radius: 6px;
		padding: 0.3rem 0.75rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 150ms;
	}

	.btn-add-friend:hover {
		opacity: 0.85;
	}

	.friend-badge,
	.pending-badge {
		font-size: 0.75rem;
		font-weight: 600;
		padding: 0.3rem 0.65rem;
		border-radius: 6px;
	}

	.friend-badge {
		background: rgba(124, 58, 237, 0.15);
		color: var(--color-brand-light, #a78bfa);
	}

	.pending-badge {
		background: var(--color-bg-surface);
		color: var(--color-text-secondary);
	}

	.empty-msg, .no-results {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		text-align: center;
		padding: 2rem 0;
	}
</style>
