<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import ImageUpload from '$lib/components/ImageUpload.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchAdvertisements, imgUrl } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Advertisement } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl, IS_TAURI } from '$lib/bridge';

	let ads = $state<Advertisement[]>([]);
	let saving = $state(false);

	// Selection state
	let selectedId = $state<number | null>(null);
	let isCreatingNew = $state(false);

	// Edit state
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editSponsor = $state('');
	let editComments = $state('');
	let editImagePath = $state<string | null>(null);
	let editDirection = $state<'top' | 'bottom' | 'left' | 'right'>('bottom');
	let editPosition = $state(50);

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);

	onMount(() => {
		fetchAdvertisements().then((data) => { ads = data; });

		socket.on('ad-created', (data: any) => {
			if (data.success) {
				// Deduplicate: direct HTTP update may have already added it
				if (!ads.some((a) => a.id === data.ad.id)) {
					ads = [...ads, data.ad];
				}
				addToast('success', 'Ad created.');
				// Auto-select only if not already done by the HTTP handler
				if (selectedId !== data.ad.id) {
					isCreatingNew = false;
					selectedId = data.ad.id;
					editId = data.ad.id;
					editImagePath = data.ad.image_path;
				}
			}
		});

		socket.on('ad-updated', (data: any) => {
			if (data.success) {
				ads = ads.map((a) => (a.id === data.ad.id ? data.ad : a));
				addToast('success', 'Ad saved.');
				if (selectedId === data.ad.id) {
					editImagePath = data.ad.image_path;
				}
			}
		});

		socket.on('ad-deleted', (data: any) => {
			if (data.success) {
				ads = ads.filter((a) => a.id !== data.id);
				addToast('success', 'Ad deleted.');
				if (selectedId === data.id) {
					selectedId = null;
					isCreatingNew = false;
				}
			}
		});

		socket.on('update-ads', () => {
			fetchAdvertisements().then((data) => { ads = data; });
		});

		return () => {
			socket.off('ad-created');
			socket.off('ad-updated');
			socket.off('ad-deleted');
			socket.off('update-ads');
		};
	});

	function openNew() {
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editSponsor = '';
		editComments = '';
		editImagePath = null;
		editDirection = 'bottom';
		editPosition = 50;
	}

	function selectAd(ad: Advertisement) {
		isCreatingNew = false;
		selectedId = ad.id;
		editId = ad.id;
		editName = ad.name;
		editSponsor = ad.sponsor_name ?? '';
		editComments = ad.comments ?? '';
		editImagePath = ad.image_path;
		editDirection = (ad.direction ?? 'bottom') as 'top' | 'bottom' | 'left' | 'right';
		editPosition = ad.position ?? 50;
	}

	async function deleteCurrentAd() {
		const ad = ads.find((a) => a.id === selectedId);
		if (!ad) return;
		if (!confirm(`Delete ad "${ad.name}"?`)) return;
		const res = await fetch(`${getBackendUrl()}/advertisements/${ad.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	async function save() {
		if (!editName.trim()) {
			addToast('error', 'Ad name is required.');
			return;
		}
		saving = true;
		try {
			if (isNew) {
				const res = await fetch(`${getBackendUrl()}/advertisements`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						sponsor_name: editSponsor.trim(),
						comments: editComments.trim(),
						direction: editDirection,
						position: editPosition,
					}),
				});
				const data = await res.json();
				if (data.success) {
					// Update list and auto-select the new ad immediately
					ads = [...ads, data.ad];
					isCreatingNew = false;
					selectedId = data.ad.id;
					editId = data.ad.id;
					editImagePath = data.ad.image_path;
				} else {
					addToast('error', data.error ?? 'Create failed.');
				}
				// socket 'ad-created' event deduplicates if also received
			} else {
				const res = await fetch(`${getBackendUrl()}/advertisements/${editId}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						sponsor_name: editSponsor.trim(),
						comments: editComments.trim(),
						direction: editDirection,
						position: editPosition,
					}),
				});
				const data = await res.json();
				if (!data.success) addToast('error', data.error ?? 'Save failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		} finally {
			saving = false;
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
				<span class="sidebar-title">
					Advertisements
					<span class="badge">{ads.length}</span>
				</span>
				<button class="btn btn-primary btn-sm" onclick={openNew}>
					<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
						<path d="M12 5v14M5 12h14"/>
					</svg>
					New
				</button>
			</div>
			<div class="sidebar-list">
				{#if isCreatingNew}
					<div class="sidebar-item selected">
						<div class="item-thumb-wrap">
							<div class="item-thumb item-thumb-empty">—</div>
						</div>
						<div class="item-info">
							<span class="item-name">New Ad…</span>
						</div>
					</div>
				{/if}
				{#each ads as ad (ad.id)}
					<button
						class="sidebar-item"
						class:selected={selectedId === ad.id}
						onclick={() => selectAd(ad)}
					>
						<div class="item-thumb-wrap">
							{#if ad.image_path}
								<MediaPreview class="item-thumb-img" src={imgUrl(ad.image_path)} alt={ad.name} />
							{:else}
								<div class="item-thumb item-thumb-empty">—</div>
							{/if}
						</div>
						<div class="item-info">
							<span class="item-name">{ad.name}</span>
							<span class="item-meta">
								{ad.sponsor_name || 'No sponsor'}
								{#if ad.programs?.length}· {ad.programs.length} program{ad.programs.length !== 1 ? 's' : ''}{/if}
							</span>
						</div>
					</button>
				{:else}
					{#if !isCreatingNew}
						<div class="sidebar-empty">No ads yet.<br/>Click "New" to get started.</div>
					{/if}
				{/each}
			</div>
		</aside>

		<!-- ── Main Editor Area ── -->
		<main class="editor-main">
			{#if hasSelection}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1 class="panel-title">{isNew ? 'New Advertisement' : (editName || 'Untitled')}</h1>
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-actions">
							{#if !isNew}
								<button class="btn btn-danger btn-sm" onclick={deleteCurrentAd}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={() => { selectedId = null; isCreatingNew = false; }}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={save} disabled={saving}>
								{saving ? 'Saving…' : isNew ? 'Create Ad' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="form-body">
						<div class="form-grid">
							<div class="field-group">
								<label class="field-label" for="ad-name">Ad Name</label>
								<input id="ad-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Summer Campaign" />
							</div>

							<div class="field-group">
								<label class="field-label" for="sponsor-name">Sponsor Name</label>
								<input id="sponsor-name" class="form-input" type="text" bind:value={editSponsor} placeholder="e.g. Acme Corp" />
							</div>

							<div class="field-group span-2">
								<label class="field-label" for="comments">Comments</label>
								<input id="comments" class="form-input" type="text" bind:value={editComments} placeholder="Optional notes" />
							</div>

							<div class="field-group">
								<label class="field-label" for="ad-direction">Slide-in Direction</label>
								<select id="ad-direction" class="form-input form-select" bind:value={editDirection}>
									<option value="bottom">Bottom → Up</option>
									<option value="top">Top → Down</option>
									<option value="left">Left → Right</option>
									<option value="right">Right → Left</option>
								</select>
							</div>

							<div class="field-group">
								<label class="field-label" for="ad-position">
									Position along edge <span class="field-hint">({editPosition}%)</span>
								</label>
								<div class="position-row">
									<input
										id="ad-position"
										class="form-input position-number"
										type="number"
										min="0"
										max="100"
										bind:value={editPosition}
									/>
									<input
										class="position-slider"
										type="range"
										min="0"
										max="100"
										bind:value={editPosition}
									/>
								</div>
							</div>
						</div>

						{#if !isNew}
							<div class="field-group">
								<span class="field-label">Ad Image / Media</span>
								<ImageUpload
									inputId="ad-image-upload"
									endpoint="/advertisements/upload-image"
									id={editId!}
									currentPath={editImagePath}
									onuploaded={(path) => { editImagePath = path; }}
								/>
							</div>

							{#if editImagePath}
								<div class="field-group">
									<span class="field-label">Preview</span>
									<div class="image-preview-box">
										<MediaPreview src={imgUrl(editImagePath)} alt={editName} />
									</div>
								</div>
							{/if}

							{#if ads.find(a => a.id === editId)?.programs?.length}
								{@const currentAd = ads.find(a => a.id === editId)!}
								<div class="field-group">
									<span class="field-label">Used in Programs</span>
									<div class="prog-pills">
										{#each currentAd.programs as p}
											<span class="prog-pill">{p.name}</span>
										{/each}
									</div>
								</div>
							{/if}
						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
						<rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 7V5a2 2 0 00-4 0v2"/>
					</svg>
					<p class="empty-title">No ad selected</p>
					<p class="empty-hint">Pick an ad from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNew}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New Ad
					</button>
				</div>
			{/if}
		</main>
	</div>
</div>

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
		width: 260px;
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
		justify-content: space-between;
		flex-shrink: 0;
	}

	.sidebar-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.badge {
		background: var(--surface-3);
		color: var(--text-2);
		font-size: 11px;
		font-weight: 700;
		border-radius: 999px;
		padding: 1px 7px;
		text-transform: none;
		letter-spacing: normal;
	}

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 14px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-left: 3px solid transparent;
		cursor: pointer;
		transition: background 0.1s;
	}

	.sidebar-item:hover {
		background: var(--surface-2);
	}

	.sidebar-item.selected {
		background: var(--accent-dim);
		border-left-color: var(--accent);
	}

	.item-thumb-wrap {
		flex-shrink: 0;
		width: 44px;
		height: 28px;
		border-radius: var(--r-sm);
		overflow: hidden;
		background: var(--surface-3);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	:global(.item-thumb-img) {
		width: 44px;
		height: 28px;
		object-fit: cover;
	}

	.item-thumb {
		width: 44px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.item-thumb-empty {
		font-size: 11px;
		color: var(--text-3);
	}

	.item-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.item-name {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.item-meta {
		font-size: 11px;
		color: var(--text-3);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sidebar-empty {
		padding: 28px 16px;
		font-size: 12px;
		color: var(--text-3);
		text-align: center;
		line-height: 1.7;
	}

	/* ── Main editor area ── */
	.editor-main {
		flex: 1;
		overflow-y: auto;
		padding: 32px 40px;
		background: var(--bg);
	}

	.editor-panel {
		max-width: 760px;
	}

	/* ── Panel header ── */
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
		align-items: baseline;
		gap: 12px;
		min-width: 0;
	}

	.panel-title {
		font-size: 20px;
		font-weight: 700;
		color: var(--text-1);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.panel-id {
		font-size: 12px;
		color: var(--text-3);
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.panel-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	/* ── Form body ── */
	.form-body {
		display: flex;
		flex-direction: column;
		gap: 28px;
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 20px;
	}

	.span-2 {
		grid-column: span 2;
	}

	.field-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.field-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--text-3);
	}

	.form-input {
		width: 100%;
		padding: 9px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 13px;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.form-input:focus {
		border-color: var(--accent);
	}

	/* ── Image preview ── */
	.image-preview-box {
		width: 100%;
		max-width: 480px;
		aspect-ratio: 16/9;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	:global(.image-preview-box img),
	:global(.image-preview-box video) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	/* ── Program pills ── */
	.prog-pills {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.prog-pill {
		font-size: 12px;
		font-weight: 500;
		background: var(--surface-3);
		color: var(--text-2);
		border-radius: 4px;
		padding: 3px 8px;
		border: 1px solid var(--border-1);
	}

	/* ── Empty state ── */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		height: calc(100vh - 48px - 64px);
		color: var(--text-3);
		text-align: center;
	}

	.empty-icon {
		color: var(--text-3);
		opacity: 0.35;
		margin-bottom: 4px;
	}

	.empty-title {
		font-size: 15px;
		font-weight: 600;
		color: var(--text-2);
	}

	.empty-hint {
		font-size: 13px;
		color: var(--text-3);
		margin-bottom: 6px;
	}

	/* ── Position / Direction fields ── */
	.form-select {
		appearance: none;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
		background-repeat: no-repeat;
		background-position: right 10px center;
		padding-right: 30px;
		cursor: pointer;
	}

	.field-hint {
		font-weight: 400;
		font-size: 10px;
		color: var(--text-3);
		text-transform: none;
		letter-spacing: normal;
	}

	.position-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.position-number {
		width: 72px;
		flex-shrink: 0;
	}

	.position-slider {
		flex: 1;
		accent-color: var(--accent);
		cursor: pointer;
	}
</style>
