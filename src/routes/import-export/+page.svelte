<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TopNav.svelte';
	import { hasData } from '$lib/api';
	import { BACKEND_URL } from '$lib/socket';
	import { addToast } from '$lib/stores/toasts';

	let canExport = $state(false);
	let importing = $state(false);
	let exporting = $state(false);
	let fileInput: HTMLInputElement;

	onMount(async () => {
		canExport = await hasData();
	});

	async function doExport() {
		exporting = true;
		try {
			const res = await fetch(`${BACKEND_URL}/export`);
			if (!res.ok) throw new Error('Export failed');
			const blob = await res.blob();
			const now = new Date();
			const stamp = now.toISOString().slice(0, 19).replace(/[T:]/g, '-');
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `obs-manager-backup-${stamp}.zip`;
			a.click();
			URL.revokeObjectURL(url);
			addToast('success', 'Database exported successfully.');
		} catch {
			addToast('error', 'Export failed.');
		} finally {
			exporting = false;
		}
	}

	function triggerImport() {
		fileInput.click();
	}

	async function onFileChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;

		if (!file.name.endsWith('.zip')) {
			addToast('error', 'Please select a .zip backup file.');
			return;
		}

		const confirmed = confirm(
			'Warning: Importing will replace ALL current data. This cannot be undone. Continue?'
		);
		if (!confirmed) return;

		importing = true;
		try {
			const formData = new FormData();
			formData.append('file', file);
			const res = await fetch(`${BACKEND_URL}/import`, {
				method: 'POST',
				body: formData,
			});
			const data = await res.json();
			if (data.success) {
				addToast('success', 'Database imported. Reload to see changes.');
				canExport = true;
			} else {
				addToast('error', data.error ?? 'Import failed.');
			}
		} catch {
			addToast('error', 'Import failed.');
		} finally {
			importing = false;
			fileInput.value = '';
		}
	}
</script>

<div class="page-wrap">
	<TopNav back={{ href: '/studio-selector', label: 'Studios' }} />
	<main class="page-content">
		<div class="center-content">
			<div class="card-panel">
				<div class="panel-icon">
					<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1M16 12l-4 4m0 0l-4-4m4 4V4"/>
					</svg>
				</div>
				<div class="panel-text">
					<h2>Database Backup</h2>
					<p>Export your programs, ads, and studio settings as a ZIP archive. Import a previous backup to restore your data.</p>
				</div>

				<div class="action-row">
					<button
						class="action-btn"
						class:disabled={!canExport}
						onclick={doExport}
						disabled={!canExport || exporting}
					>
						<div class="action-icon export-icon">
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
								<path d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1M16 12l-4 4m0 0l-4-4m4 4V4"/>
							</svg>
						</div>
						<div class="action-text">
							<span class="action-title">{exporting ? 'Exporting…' : 'Export Database'}</span>
							<span class="action-sub">Download a .zip backup file</span>
						</div>
					</button>

					<button class="action-btn" onclick={triggerImport} disabled={importing}>
						<div class="action-icon import-icon">
							<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
								<path d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1M8 12l4-4m0 0l4 4m-4-4v12"/>
							</svg>
						</div>
						<div class="action-text">
							<span class="action-title">{importing ? 'Importing…' : 'Import Database'}</span>
							<span class="action-sub">Restore from a .zip backup file</span>
						</div>
					</button>
				</div>

				{#if !canExport}
					<p class="warn-note">
						<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>
						</svg>
						Export is disabled — no data in database yet.
					</p>
				{/if}

				<div class="danger-note">
					<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/>
					</svg>
					Importing will permanently replace all existing data.
				</div>
			</div>
		</div>
	</main>
</div>

<input
	bind:this={fileInput}
	type="file"
	accept=".zip"
	onchange={onFileChange}
	style="display:none"
/>

<style>
	.center-content {
		display: flex;
		justify-content: center;
		padding-top: 40px;
	}

	.card-panel {
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-xl);
		padding: 36px;
		max-width: 520px;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.panel-icon {
		width: 52px;
		height: 52px;
		background: var(--accent-dim);
		border: 1px solid rgba(56, 189, 248, 0.2);
		border-radius: var(--r-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--accent);
	}

	.panel-text h2 {
		font-size: 18px;
		font-weight: 700;
		color: var(--text-1);
		margin-bottom: 6px;
	}

	.panel-text p {
		font-size: 13px;
		color: var(--text-3);
		line-height: 1.6;
	}

	.action-row {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.action-btn {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 14px 18px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-lg);
		cursor: pointer;
		font-family: inherit;
		text-align: left;
		transition: all 0.15s;
	}

	.action-btn:hover:not(:disabled) {
		border-color: var(--border-2);
		background: var(--surface-3);
	}

	.action-btn:disabled,
	.action-btn.disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.action-icon {
		width: 40px;
		height: 40px;
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.export-icon {
		background: var(--accent-dim);
		color: var(--accent);
		border: 1px solid rgba(56, 189, 248, 0.2);
	}

	.import-icon {
		background: var(--go-dim);
		color: var(--go);
		border: 1px solid rgba(34, 197, 94, 0.2);
	}

	.action-text {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.action-title {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-1);
	}

	.action-sub {
		font-size: 11px;
		color: var(--text-3);
	}

	.warn-note {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--warn);
		background: var(--warn-dim);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: var(--r-sm);
		padding: 8px 12px;
	}

	.danger-note {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--text-3);
		background: var(--live-dim);
		border: 1px solid rgba(239, 68, 68, 0.15);
		border-radius: var(--r-sm);
		padding: 8px 12px;
	}
</style>
