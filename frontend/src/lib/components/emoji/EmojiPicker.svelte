<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";
	import { serversStore } from "$stores/servers.js";

	export let serverId: string | null = null;

	const dispatch = createEventDispatcher<{ select: string }>();

	let search = "";
	let activeTab: "recent" | "server" | string = "recent";
	let recentEmojis: string[] = [];

	// Standard emoji categories (subset for MVP)
	const standardEmojis: Record<string, string[]> = {
		"😀 Smileys": [
			"😀",
			"😂",
			"🥹",
			"😊",
			"😇",
			"🥰",
			"😍",
			"😘",
			"🤩",
			"😎",
			"🤔",
			"🤭",
			"🫠",
			"😑",
			"😒",
			"😞",
			"😢",
			"😤",
			"😡",
			"🤯",
			"😱",
			"😨",
			"🤗",
			"🫡",
			"🤫",
			"🫤",
			"😐",
			"😶",
			"🙄",
			"😬",
			"😮",
		],
		"👍 Gestures": [
			"👍",
			"👎",
			"👋",
			"🤙",
			"✌",
			"🤞",
			"🫰",
			"🤟",
			"🤘",
			"👌",
			"🤌",
			"🫵",
			"🫶",
			"❤",
			"🧡",
			"💛",
			"💚",
			"💙",
			"💜",
			"🖤",
			"🤍",
			"🤎",
			"💔",
			"💯",
			"✨",
			"⚡",
			"🔥",
			"🎉",
			"🎊",
			"🚀",
			"⭐",
		],
		"🐶 Animals": [
			"🐶",
			"🐱",
			"🐭",
			"🐹",
			"🐰",
			"🦊",
			"🐻",
			"🐼",
			"🐨",
			"🐯",
			"🦁",
			"🐮",
			"🐷",
			"🐸",
			"🐵",
			"🐔",
			"🐧",
			"🐦",
			"🦆",
			"🦅",
			"🦋",
			"🐛",
			"🐝",
			"🪲",
			"🐢",
			"🦎",
			"🐍",
		],
		"🍕 Food": [
			"🍕",
			"🍔",
			"🍟",
			"🌭",
			"🍿",
			"🧆",
			"🌮",
			"🌯",
			"🫔",
			"🥙",
			"🧇",
			"🥞",
			"🧈",
			"🍳",
			"🥚",
			"🧀",
			"🥗",
			"🥘",
			"🫕",
			"🍜",
			"🍝",
			"🍛",
			"🍣",
			"🍱",
		],
	};

	// Custom server emojis
	$: serverData = $serversStore.servers.find((s) => s.id === serverId);
	$: customEmojis = (serverData as any)?.customEmojis ?? [];

	$: filteredStandard = search
		? (Object.fromEntries(
				Object.entries(standardEmojis)
					.map(([cat, emojis]) => [
						cat,
						emojis.filter((e) => e.includes(search)),
					])
					.filter(([, arr]) => (arr as string[]).length > 0),
			) as Record<string, string[]>)
		: standardEmojis;

	function select(emoji: string) {
		dispatch("select", emoji);
		// Add to recent
		recentEmojis = [
			emoji,
			...recentEmojis.filter((e) => e !== emoji),
		].slice(0, 24);
		try {
			localStorage.setItem(
				"yapper_recent_emojis",
				JSON.stringify(recentEmojis),
			);
		} catch {}
	}

	onMount(() => {
		try {
			const saved = localStorage.getItem("yapper_recent_emojis");
			if (saved) recentEmojis = JSON.parse(saved);
		} catch {}
	});
</script>

<div class="emoji-picker" role="dialog" aria-label="Emoji picker">
	<!-- Search -->
	<div class="search-bar">
		<span class="search-icon">🔍</span>
		<input
			type="text"
			class="search-input"
			placeholder="Search emoji…"
			bind:value={search}
			autocomplete="off"
		/>
	</div>

	<!-- Tabs -->
	<div class="tabs" role="tablist">
		<button
			class="tab"
			class:active={activeTab === "recent"}
			role="tab"
			on:click={() => (activeTab = "recent")}>🕐</button
		>

		{#if customEmojis.length > 0}
			<button
				class="tab"
				class:active={activeTab === "server"}
				role="tab"
				on:click={() => (activeTab = "server")}>🌐</button
			>
		{/if}

		{#each Object.keys(standardEmojis) as cat}
			<button
				class="tab"
				class:active={activeTab === cat}
				role="tab"
				title={cat}
				on:click={() => (activeTab = cat)}>{cat.split(" ")[0]}</button
			>
		{/each}
	</div>

	<!-- Emoji grid -->
	<div class="emoji-scroll">
		{#if activeTab === "recent"}
			{#if recentEmojis.length === 0}
				<div class="empty-tab">No recent emojis yet.</div>
			{:else}
				<div class="category-label">Recently Used</div>
				<div class="emoji-grid">
					{#each recentEmojis as emoji}
						<button
							class="emoji-btn"
							title={emoji}
							on:click={() => select(emoji)}
						>
							{emoji}
						</button>
					{/each}
				</div>
			{/if}
		{:else if activeTab === "server"}
			<div class="category-label">Server Emojis</div>
			<div class="emoji-grid custom-grid">
				{#each customEmojis as emoji}
					<button
						class="emoji-btn custom-emoji-btn"
						title=":{emoji.name}:"
						on:click={() => select(`:${emoji.name}:`)}
					>
						<img
							src={emoji.url}
							alt={emoji.name}
							class="custom-emoji-img"
						/>
					</button>
				{/each}
			</div>
		{:else if search}
			{#each Object.entries(filteredStandard) as [cat, emojis]}
				<div class="category-label">{cat}</div>
				<div class="emoji-grid">
					{#each emojis as emoji}
						<button
							class="emoji-btn"
							title={emoji}
							on:click={() => select(emoji)}
						>
							{emoji}
						</button>
					{/each}
				</div>
			{/each}
			{#if Object.keys(filteredStandard).length === 0}
				<div class="empty-tab">No results for "{search}"</div>
			{/if}
		{:else}
			{#each Object.entries(standardEmojis) as [cat, emojis]}
				{#if activeTab === cat || !Object.keys(standardEmojis).includes(activeTab)}
					{#if activeTab === cat}
						<div class="category-label">{cat}</div>
						<div class="emoji-grid">
							{#each emojis as emoji}
								<button
									class="emoji-btn"
									title={emoji}
									on:click={() => select(emoji)}
								>
									{emoji}
								</button>
							{/each}
						</div>
					{/if}
				{/if}
			{/each}
		{/if}
	</div>
</div>

<style>
	.emoji-picker {
		width: 360px;
		background: #1c1d26;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 14px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
	}

	/* Search */
	.search-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.search-icon {
		font-size: 14px;
		flex-shrink: 0;
	}

	.search-input {
		flex: 1;
		background: none;
		border: none;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
	}

	.search-input::placeholder {
		color: #6b7280;
	}

	/* Tabs */
	.tabs {
		display: flex;
		overflow-x: auto;
		padding: 6px 8px;
		gap: 2px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		scrollbar-width: none;
	}

	.tabs::-webkit-scrollbar {
		display: none;
	}

	.tab {
		width: 32px;
		height: 30px;
		background: none;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-size: 16px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: background 100ms;
	}

	.tab:hover {
		background: rgba(255, 255, 255, 0.06);
	}

	.tab.active {
		background: rgba(124, 58, 237, 0.2);
	}

	/* Emoji scroll area */
	.emoji-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 8px;
		max-height: 340px;
	}

	.category-label {
		font-size: 11px;
		font-weight: 700;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 8px 4px 4px;
	}

	.emoji-grid {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: 2px;
	}

	.emoji-btn {
		width: 36px;
		height: 36px;
		background: none;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-size: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 100ms,
			transform 80ms;
	}

	.emoji-btn:hover {
		background: rgba(255, 255, 255, 0.08);
		transform: scale(1.2);
	}

	.custom-emoji-btn {
		padding: 4px;
	}

	.custom-emoji-img {
		width: 24px;
		height: 24px;
		object-fit: contain;
	}

	.empty-tab {
		font-size: 13px;
		color: #6b7280;
		text-align: center;
		padding: 24px;
	}
</style>
