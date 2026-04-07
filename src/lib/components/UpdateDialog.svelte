<script lang="ts">
	import { onMount } from 'svelte';

	let updateVersion = $state<string | null>(null);
	let installing = $state(false);
	let error = $state<string | null>(null);

	onMount(async () => {
		// Only run inside Tauri
		if (typeof window === 'undefined' || !('__TAURI__' in window)) return;

		const { listen } = await import('@tauri-apps/api/event');
		await listen<string>('update-available', (event) => {
			updateVersion = event.payload;
		});
	});

	async function installUpdate() {
		installing = true;
		error = null;
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			// Downloads, verifies signature, installs, then the app restarts automatically
			await invoke('install_update');
		} catch (e) {
			error = String(e);
			installing = false;
		}
	}

	function dismiss() {
		updateVersion = null;
	}
</script>

{#if updateVersion}
	<div class="update-overlay" role="dialog" aria-modal="true" aria-label="Update available">
		<div class="update-dialog">
			<div class="update-header">
				<span class="update-icon">&#8593;</span>
				<h3>Update Available</h3>
			</div>

			<p class="update-body">
				Version <strong>{updateVersion}</strong> is available. The app will restart automatically
				after the update installs.
			</p>

			{#if error}
				<p class="update-error">{error}</p>
			{/if}

			<div class="update-actions">
				{#if installing}
					<div class="update-progress">
						<div class="spinner"></div>
						<span>Downloading and installing&hellip;</span>
					</div>
				{:else}
					<button class="btn btn-secondary btn-sm" onclick={dismiss}>Later</button>
					<button class="btn btn-primary btn-sm" onclick={installUpdate}>Update Now</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.update-overlay {
		position: fixed;
		bottom: 24px;
		right: 24px;
		z-index: 9999;
		animation: slide-in 0.2s ease;
	}

	@keyframes slide-in {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.update-dialog {
		background: var(--surface-2);
		border: 1px solid var(--border-2);
		border-radius: var(--r-lg);
		padding: 16px 20px;
		min-width: 280px;
		max-width: 340px;
		box-shadow:
			0 8px 32px rgba(0, 0, 0, 0.4),
			0 2px 8px rgba(0, 0, 0, 0.2);
	}

	.update-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 8px;
	}

	.update-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		background: var(--accent-dim);
		color: var(--accent);
		border-radius: 50%;
		font-size: 0.75rem;
		font-weight: 700;
		flex-shrink: 0;
	}

	.update-header h3 {
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--text-1);
	}

	.update-body {
		font-size: 0.8125rem;
		color: var(--text-2);
		line-height: 1.5;
		margin-bottom: 14px;
	}

	.update-body strong {
		color: var(--text-1);
		font-weight: 600;
	}

	.update-error {
		font-size: 0.75rem;
		color: var(--live);
		background: var(--live-dim);
		border-radius: var(--r-sm);
		padding: 6px 10px;
		margin-bottom: 10px;
	}

	.update-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		align-items: center;
	}

	.update-progress {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.8125rem;
		color: var(--text-2);
	}

	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border-2);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
