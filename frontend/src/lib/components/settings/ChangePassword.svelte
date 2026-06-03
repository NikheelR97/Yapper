<script lang="ts">
    import { api } from "$api/client.js";
    import { toast } from "$stores/toast.js";
    import { goto } from "$app/navigation";
    import { clearAuth } from "$stores/auth.js";

    let currentPassword = "";
    let newPassword = "";
    let confirmPassword = "";
    let saving = false;

    $: passwordsMatch = newPassword === confirmPassword;
    $: canSubmit =
        currentPassword.length > 0 && newPassword.length >= 8 && passwordsMatch;

    async function save() {
        if (!canSubmit || saving) return;
        saving = true;
        try {
            await api.post("/api/v2/auth/change-password", {
                current_password: currentPassword,
                new_password: newPassword,
            });
            toast.success("Password changed! Please sign in again.");
            // Backend revokes all sessions — clear local state and redirect to login
            clearAuth();
            await goto("/login");
        } catch (e: unknown) {
            toast.error(e instanceof Error ? e.message : "Failed to change password");
        } finally {
            saving = false;
        }
    }
</script>

<div class="password-section">
    <h2 class="section-title">Change Password</h2>

    <div class="info-banner">
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <circle cx="12" cy="12" r="10" /><line
                x1="12"
                y1="16"
                x2="12"
                y2="12"
            /><line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        Changing your password will sign you out of all devices.
    </div>

    <form on:submit|preventDefault={save} class="password-form">
        <div class="field">
            <label class="field-label" for="currentPw">Current Password</label>
            <input
                id="currentPw"
                type="password"
                class="field-input"
                bind:value={currentPassword}
                placeholder="Enter your current password"
                autocomplete="current-password"
            />
        </div>

        <div class="field">
            <label class="field-label" for="newPw">New Password</label>
            <input
                id="newPw"
                type="password"
                class="field-input"
                bind:value={newPassword}
                placeholder="At least 8 characters"
                autocomplete="new-password"
                minlength={8}
            />
            {#if newPassword.length > 0 && newPassword.length < 8}
                <span class="field-error"
                    >Password must be at least 8 characters</span
                >
            {/if}
        </div>

        <div class="field">
            <label class="field-label" for="confirmPw"
                >Confirm New Password</label
            >
            <input
                id="confirmPw"
                type="password"
                class="field-input"
                class:error={confirmPassword.length > 0 && !passwordsMatch}
                bind:value={confirmPassword}
                placeholder="Repeat your new password"
                autocomplete="new-password"
            />
            {#if confirmPassword.length > 0 && !passwordsMatch}
                <span class="field-error">Passwords do not match</span>
            {/if}
        </div>

        <button type="submit" class="save-btn" disabled={!canSubmit || saving}>
            {saving ? "Changing…" : "Change Password"}
        </button>
    </form>
</div>

<style>
    .password-section {
        display: flex;
        flex-direction: column;
        gap: 24px;
    }

    .section-title {
        font-size: 20px;
        font-weight: 800;
        color: var(--color-text-primary);
        margin: 0;
    }

    .info-banner {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 12px 16px;
        background: rgba(59, 130, 246, 0.08);
        border: 1px solid rgba(59, 130, 246, 0.2);
        border-radius: 10px;
        font-size: 13px;
        color: var(--color-info-text);
    }

    .password-form {
        display: flex;
        flex-direction: column;
        gap: 20px;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .field-label {
        font-size: 12px;
        font-weight: 700;
        color: var(--color-text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .field-input {
        padding: 10px 14px;
        background: var(--color-bg-elevated);
        border: 1px solid var(--color-border);
        border-radius: 8px;
        color: var(--color-text-primary);
        font-size: 14px;
        font-family: inherit;
        outline: none;
        transition: border-color 150ms;
    }

    .field-input:focus {
        border-color: var(--color-brand);
    }

    .field-input.error {
        border-color: var(--color-error);
    }

    .field-error {
        font-size: 12px;
        color: var(--color-error-text);
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
