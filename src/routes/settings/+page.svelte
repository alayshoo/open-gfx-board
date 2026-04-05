<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { hasData } from '$lib/api/api';
	import { BACKEND_URL } from '$lib/api/socket';
	import { addToast } from '$lib/toasts';
	import { IS_TAURI } from '$lib/bridge';

	type Tab = 'import-export' | 'about';
	let activeTab = $state<Tab>('import-export');

	let appVersion = $state('');
	let canExport = $state(false);
	let importing = $state(false);
	let exporting = $state(false);
	let fileInput: HTMLInputElement;

	onMount(async () => {
		canExport = await hasData();
		if (IS_TAURI) {
			const { getVersion } = await import('@tauri-apps/api/app');
			appVersion = await getVersion();
		}
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

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/', label: 'Control' }} />
	{/if}
	<div class="editor-body">

		<!-- ── Sidebar ── -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<span class="sidebar-title">Settings</span>
			</div>
			<nav class="sidebar-list">
				<button
					class="sidebar-item"
					class:selected={activeTab === 'import-export'}
					onclick={() => (activeTab = 'import-export')}
				>
					<div class="item-icon">
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
							<path d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1M8 12l4-4m0 0l4 4m-4-4v12"/>
						</svg>
					</div>
					<span class="item-label">Import &amp; Export</span>
				</button>

				<button
					class="sidebar-item"
					class:selected={activeTab === 'about'}
					onclick={() => (activeTab = 'about')}
				>
					<div class="item-icon">
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
							<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>
						</svg>
					</div>
					<span class="item-label">About</span>
				</button>
			</nav>
		</aside>

		<!-- ── Main area ── -->
		<main class="editor-main">

			<!-- Import & Export tab -->
			{#if activeTab === 'import-export'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1 class="panel-title">Import &amp; Export</h1>
						</div>
					</div>

					<div class="form-body">
						<p class="section-desc">
							Export your programs, screens, pop-ups, and studio settings as a ZIP archive.
							Import a previous backup to fully restore your data.
						</p>

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

			<!-- About tab -->
			{:else if activeTab === 'about'}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<img src="/icon.png" alt="Open GFX Board" class="about-icon" />
							<div class="about-app-info">
								<h1 class="panel-title">Open GFX Board</h1>
								{#if appVersion}
									<span class="about-version">v{appVersion}</span>
								{/if}
							</div>
						</div>
					</div>

					<div class="form-body">
						<p class="section-desc">
							A graphics and overlay management tool for live broadcasting. Manage programs, screens,
							pop-up overlays, and studio presets — all in one place.
						</p>
						<p class="about-credit">
							Developed by <a href="https://github.com/alayshoo" target="_blank" rel="noopener noreferrer" class="about-link">Alayshoo</a>.
						</p>
					</div>
				</div>
			{/if}

		</main>
	</div>
</div>

<input
	bind:this={fileInput}
	type="file"
	accept=".zip"
	onchange={onFileChange}
	style="display:none"
/>

<style>
	/* ── Layout ── */
	.editor-wrap {
		height: 100vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.editor-body {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	/* ── Sidebar ── */
	.sidebar {
		width: 220px;
		flex-shrink: 0;
		background: var(--surface-1);
		border-right: 1px solid var(--border-1);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.sidebar-header {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-1);
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}

	.sidebar-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
	}

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
		padding: 6px 0;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 14px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-left: 3px solid transparent;
		cursor: pointer;
		transition: background 0.1s;
		font-family: inherit;
	}

	.sidebar-item:hover {
		background: var(--surface-2);
	}

	.sidebar-item.selected {
		background: var(--accent-dim);
		border-left-color: var(--accent);
	}

	.item-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-3);
		flex-shrink: 0;
	}

	.sidebar-item.selected .item-icon {
		color: var(--accent);
	}

	.item-label {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-2);
	}

	.sidebar-item.selected .item-label {
		color: var(--text-1);
	}

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px 40px;
		background: var(--bg);
	}

	.editor-panel {
		max-width: 600px;
	}

	.panel-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 32px;
		padding-bottom: 24px;
		border-bottom: 1px solid var(--border-1);
	}

	.panel-title-area {
		display: flex;
		align-items: center;
		gap: 24px;
	}

	.panel-title {
		font-size: 20px;
		font-weight: 700;
		color: var(--text-1);
	}

	.form-body {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.section-desc {
		font-size: 13px;
		color: var(--text-3);
		line-height: 1.7;
		margin: 0;
	}

	/* ── Import/Export actions ── */
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
		margin: 0;
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

	/* ── About tab ── */
	.about-icon {
		width: 96px;
		height: 96px;
		border-radius: var(--r-lg);
		flex-shrink: 0;
		filter: drop-shadow(0 0 4px rgba(248, 248, 248, 0.18));
	}

	.about-app-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.about-app-info .panel-title {
		font-size: 32px;
	}

	.about-version {
		font-size: 13px;
		color: var(--text-3);
	}

	.about-credit {
		font-size: 13px;
		color: var(--text-3);
		margin: 0;
	}

	.about-link {
		color: var(--accent);
		text-decoration: none;
	}

	.about-link:hover {
		text-decoration: underline;
	}
</style>
