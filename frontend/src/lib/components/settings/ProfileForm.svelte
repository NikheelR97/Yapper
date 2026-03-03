<script lang="ts">
	import { authStore, setAuth } from "$stores/auth.js";
	import { api } from "$api/client.js";
	import { toast } from "$stores/toast.js";
	import { get } from "svelte/store";
	import { onMount } from "svelte";

	$: user = $authStore.user;

	let displayName = user?.displayName ?? "";
	let aboutMe = "";
	let selectedTheme = "#7c3aed";
	let saving = false;

	const themes = ["#7c3aed", "#ec4899", "#0891b2", "#059669", "#f59e0b"];

	let customColor = "#7c3aed";
	let showColorPicker = false;

	$: charCount = aboutMe.length;

	// Pre-populate bio and theme from the full profile (not stored in JWT)
	onMount(async () => {
		try {
			const profile = await api.get<{
				about_me: string | null;
				profile_theme_color: string | null;
			}>("/api/v1/users/me");
			if (profile.about_me) aboutMe = profile.about_me;
			if (profile.profile_theme_color) {
				selectedTheme = profile.profile_theme_color;
				customColor = profile.profile_theme_color;
			}
		} catch {
			// Non-critical — form just starts with defaults
		}
	});

	async function save() {
		if (saving) return;
		saving = true;
		try {
			await api.patch("/api/v1/users/me", {
				display_name: displayName.trim() || undefined,
				about_me: aboutMe.trim() || undefined,
				profile_theme_color: selectedTheme,
			});
			// Update local auth store so TopNav avatar name updates immediately
			const current = get(authStore);
			if (current.user && current.accessToken) {
				setAuth(
					{
						...current.user,
						displayName:
							displayName.trim() || current.user.displayName,
					},
					current.accessToken,
					current.csrfToken,
				);
			}
			toast.success("Profile saved!");
		} catch (e: any) {
			toast.error(e.message ?? "Failed to save");
		} finally {
			saving = false;
		}
	}
</script>

<div class="profile-form">
	<!-- Preview card -->
	<div class="preview-card">
		<div
			class="preview-banner"
			style="background: linear-gradient(135deg, {selectedTheme}33, {selectedTheme}66)"
		>
			<svg
				viewBox="0 0 400 120"
				xmlns="http://www.w3.org/2000/svg"
				class="wave-svg"
				preserveAspectRatio="none"
			>
				<path
					d="M0,60 C100,100 300,20 400,60 L400,120 L0,120 Z"
					fill="{selectedTheme}22"
				/>
				<path
					d="M0,80 C120,40 280,100 400,70 L400,120 L0,120 Z"
					fill="{selectedTheme}33"
				/>
			</svg>
		</div>
		<div class="preview-avatar">
			{#if user?.avatarUrl}
				<img src={user.avatarUrl} alt="Avatar" />
			{:else}
				<div
					class="avatar-placeholder"
					style="background: linear-gradient(135deg, {selectedTheme}, #2e1065)"
				>
					{user?.displayName?.[0]?.toUpperCase() ?? "U"}
				</div>
			{/if}
		</div>
		<div class="preview-body">
			<div class="preview-name">
				{displayName || user?.displayName || "Display Name"}
			</div>
			<div class="preview-username">@{user?.username ?? "username"}</div>
			{#if aboutMe}
				<div class="preview-bio">"{aboutMe}"</div>
			{/if}
		</div>
	</div>

	<!-- Display name -->
	<div class="field">
		<label class="field-label" for="displayName">Display Name</label>
		<input
			id="displayName"
			type="text"
			class="field-input"
			bind:value={displayName}
			maxlength={32}
			placeholder="Your display name"
		/>
	</div>

	<!-- Username (locked) -->
	<div class="field">
		<label class="field-label" for="username">Username</label>
		<div class="locked-input">
			<input
				id="username"
				type="text"
				class="field-input locked"
				value={user?.username ?? ""}
				disabled
			/>
			<span class="lock-icon">🔒</span>
		</div>
		<span class="field-hint">Username cannot be changed.</span>
	</div>

	<!-- About me -->
	<div class="field">
		<label class="field-label" for="aboutMe">About Me</label>
		<textarea
			id="aboutMe"
			class="field-textarea"
			bind:value={aboutMe}
			maxlength={190}
			placeholder="Tell the world about yourself…"
			rows={4}
		></textarea>
		<span class="char-counter" class:near-limit={charCount >= 170}
			>{charCount}/190</span
		>
	</div>

	<!-- Profile theme -->
	<div class="field">
		<label class="field-label">Profile Theme</label>
		<div class="theme-swatches">
			{#each themes as color}
				<button
					class="swatch"
					class:active={selectedTheme === color}
					style="background: {color}"
					on:click={() => {
						selectedTheme = color;
						showColorPicker = false;
					}}
					aria-label={color}
				></button>
			{/each}
			<button
				class="swatch custom-swatch"
				class:active={!themes.includes(selectedTheme)}
				style="background: {customColor}"
				on:click={() => (showColorPicker = !showColorPicker)}
				title="Custom color"
			>
				{#if !themes.includes(selectedTheme)}✓{:else}+{/if}
			</button>
		</div>
		{#if showColorPicker}
			<input
				type="color"
				class="color-picker"
				bind:value={customColor}
				on:input={() => (selectedTheme = customColor)}
			/>
		{/if}
	</div>

	<button class="save-btn" on:click={save} disabled={saving}>
		{saving ? "Saving…" : "Save Changes"}
	</button>
</div>

<style>
	.profile-form {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	/* Preview */
	.preview-card {
		border-radius: 16px;
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: #16171e;
	}

	.preview-banner {
		height: 100px;
		position: relative;
		overflow: hidden;
	}

	.wave-svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.preview-avatar {
		width: 72px;
		height: 72px;
		border-radius: 50%;
		border: 3px solid #16171e;
		overflow: hidden;
		margin: -36px 0 0 20px;
		position: relative;
	}

	.preview-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.avatar-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 26px;
		font-weight: 800;
		color: white;
	}

	.preview-body {
		padding: 12px 20px 20px;
	}

	.preview-name {
		font-size: 18px;
		font-weight: 800;
		color: #f9fafb;
	}

	.preview-username {
		font-size: 13px;
		color: #6b7280;
		margin-top: 2px;
	}

	.preview-bio {
		font-size: 13px;
		color: #9ca3af;
		margin-top: 8px;
		font-style: italic;
	}

	/* Fields */
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field-label {
		font-size: 12px;
		font-weight: 700;
		color: #9ca3af;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.field-input {
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
		transition: border-color 150ms;
	}

	.field-input:focus {
		border-color: #7c3aed;
	}

	.field-input.locked {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.locked-input {
		position: relative;
	}

	.lock-icon {
		position: absolute;
		right: 12px;
		top: 50%;
		transform: translateY(-50%);
		font-size: 14px;
	}

	.field-hint {
		font-size: 12px;
		color: #6b7280;
	}

	.field-textarea {
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		outline: none;
		resize: vertical;
		min-height: 100px;
		transition: border-color 150ms;
	}

	.field-textarea:focus {
		border-color: #7c3aed;
	}

	.char-counter {
		font-size: 12px;
		color: #6b7280;
		text-align: right;
	}

	.char-counter.near-limit {
		color: #f59e0b;
	}

	/* Theme */
	.theme-swatches {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.swatch {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		transition:
			transform 150ms,
			border-color 150ms;
	}

	.swatch:hover {
		transform: scale(1.15);
	}

	.swatch.active {
		border-color: white;
		box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.3);
	}

	.custom-swatch {
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		font-size: 14px;
		font-weight: 700;
	}

	.color-picker {
		width: 48px;
		height: 32px;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		padding: 0;
		background: none;
	}

	.save-btn {
		padding: 12px 24px;
		background: #7c3aed;
		color: white;
		border: none;
		border-radius: 10px;
		font-size: 15px;
		font-weight: 700;
		cursor: pointer;
		align-self: flex-start;
		transition: opacity 150ms;
	}

	.save-btn:hover:not(:disabled) {
		opacity: 0.85;
	}

	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
