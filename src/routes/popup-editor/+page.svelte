<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import ImageUpload from '$lib/components/ImageUpload.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchPopUps, imgUrl } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { PopUp } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl, IS_TAURI } from '$lib/bridge';
	import { showConfirm } from '$lib/confirm';

	let popups = $state<PopUp[]>([]);
	let saving = $state(false);
	let uploading = $state(false);

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
	let editMediaType = $state('image');

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);

	onMount(() => {
		fetchPopUps().then((data) => { popups = data; });

		socket.on('popup-created', (data: any) => {
			if (data.success) {
				// Deduplicate: direct HTTP update may have already added it
				if (!popups.some((p) => p.id === data.popup.id)) {
					popups = [...popups, data.popup];
				}
				addToast('success', 'PopUp created.');
				// Auto-select only if not already done by the HTTP handler
				if (selectedId !== data.popup.id) {
					isCreatingNew = false;
					selectedId = data.popup.id;
					editId = data.popup.id;
					editImagePath = data.popup.image_path;
				}
			}
		});

		socket.on('popup-updated', (data: any) => {
			if (data.success) {
				popups = popups.map((p) => (p.id === data.popup.id ? data.popup : p));
				addToast('success', 'PopUp saved.');
				if (selectedId === data.popup.id) {
					editImagePath = data.popup.image_path;
				}
			}
		});

		socket.on('popup-deleted', (data: any) => {
			if (data.success) {
				popups = popups.filter((p) => p.id !== data.id);
				addToast('success', 'PopUp deleted.');
				if (selectedId === data.id) {
					selectedId = null;
					isCreatingNew = false;
				}
			}
		});

		socket.on('update-popups', () => {
			fetchPopUps().then((data) => { popups = data; });
		});

		return () => {
			socket.off('popup-created');
			socket.off('popup-updated');
			socket.off('popup-deleted');
			socket.off('update-popups');
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
		editMediaType = 'image';
	}

	function selectPopUp(popup: PopUp) {
		isCreatingNew = false;
		selectedId = popup.id;
		editId = popup.id;
		editName = popup.name;
		editSponsor = popup.sponsor_name ?? '';
		editComments = popup.comments ?? '';
		editImagePath = popup.image_path;
		editDirection = (popup.direction ?? 'bottom') as 'top' | 'bottom' | 'left' | 'right';
		editPosition = popup.position ?? 50;
		editMediaType = popup.media_type ?? 'image';
	}

	async function deleteCurrentPopUp() {
		const popup = popups.find((p) => p.id === selectedId);
		if (!popup) return;
		if (!await showConfirm({ title: 'Delete Pop-Up', message: `Delete pop-up "${popup.name}"? This cannot be undone.`, confirmLabel: 'Delete' })) return;
		const res = await fetch(`${getBackendUrl()}/popups/${popup.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	async function save() {
		if (!editName.trim()) {
			addToast('error', 'PopUp name is required.');
			return;
		}
		saving = true;
		try {
			if (isNew) {
				const res = await fetch(`${getBackendUrl()}/popups`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						sponsor_name: editSponsor.trim(),
						comments: editComments.trim(),
						direction: editDirection,
						position: editPosition,
						media_type: editMediaType,
					}),
				});
				const data = await res.json();
				if (data.success) {
					// Update list and auto-select the new popup immediately
					popups = [...popups, data.popup];
					isCreatingNew = false;
					selectedId = data.popup.id;
					editId = data.popup.id;
					editImagePath = data.popup.image_path;
				} else {
					addToast('error', data.error ?? 'Create failed.');
				}
				// socket 'popup-created' event deduplicates if also received
			} else {
				const res = await fetch(`${getBackendUrl()}/popups/${editId}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						sponsor_name: editSponsor.trim(),
						comments: editComments.trim(),
						direction: editDirection,
						position: editPosition,
						media_type: editMediaType,
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
					PopUps
					<span class="badge">{popups.length}</span>
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
							<span class="item-name">New PopUp…</span>
						</div>
					</div>
				{/if}
				{#each popups as popup (popup.id)}
					<button
						class="sidebar-item"
						class:selected={selectedId === popup.id}
						onclick={() => selectPopUp(popup)}
					>
						<div class="item-thumb-wrap">
							{#if popup.image_path}
								<MediaPreview class="item-thumb-img" src={imgUrl(popup.image_path)} alt={popup.name} />
							{:else}
								<div class="item-thumb item-thumb-empty">—</div>
							{/if}
						</div>
						<div class="item-info">
							<span class="item-name">{popup.name}</span>
							<span class="item-meta">
								{popup.sponsor_name || 'No sponsor'}
								{#if popup.programs?.length}· {popup.programs.length} program{popup.programs.length !== 1 ? 's' : ''}{/if}
							</span>
						</div>
					</button>
				{:else}
					{#if !isCreatingNew}
						<div class="sidebar-empty">No pop-ups yet.<br/>Click "New" to get started.</div>
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
							<h1>{isNew ? 'New PopUp' : (editName || 'Untitled')}</h1>
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-actions">
							{#if !isNew}
								<button class="btn btn-danger btn-sm" onclick={deleteCurrentPopUp}>Delete</button>
							{/if}
							<button class="btn btn-ghost btn-sm" onclick={() => { selectedId = null; isCreatingNew = false; }}>
								Cancel
							</button>
							<button class="btn btn-primary" onclick={save} disabled={saving || uploading}>
								{saving ? 'Saving…' : isNew ? 'Create PopUp' : 'Save Changes'}
							</button>
						</div>
					</div>

					<div class="form-body">
						<div class="form-grid">
							<div class="field-group">
								<label class="field-label" for="popup-name">PopUp Name</label>
								<input id="popup-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Summer Campaign" />
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
								<label class="field-label" for="popup-direction">Slide-in Direction</label>
								<select id="popup-direction" class="form-input form-select" bind:value={editDirection}>
									<option value="bottom">Bottom → Up</option>
									<option value="top">Top → Down</option>
									<option value="left">Left → Right</option>
									<option value="right">Right → Left</option>
								</select>
							</div>

							<div class="field-group">
								<label class="field-label" for="popup-position">
									Position along edge <span class="field-hint">({editPosition}%)</span>
								</label>
								<div class="position-row">
									<input
										id="popup-position"
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

							<div class="field-group">
								<label class="field-label" for="popup-media-type">Media Type</label>
								<select id="popup-media-type" class="form-input form-select" bind:value={editMediaType}>
									<option value="image">Image</option>
									<option value="video">Video</option>
								</select>
							</div>
						</div>

						{#if !isNew}
							<div class="field-group">
								<span class="field-label">PopUp Image / Media</span>
								<ImageUpload
									inputId="popup-image-upload"
									endpoint="/popups/upload-image"
									id={editId!}
									currentPath={editImagePath}
									onuploaded={(path) => { editImagePath = path; }}
					onuploadingchange={(v) => { uploading = v; }}
								/>
							</div>

							{#if popups.find(p => p.id === editId)?.programs?.length}
								{@const currentPopUp = popups.find(p => p.id === editId)!}
								<div class="field-group">
									<span class="field-label">Used in Programs</span>
									<div class="prog-pills">
										{#each currentPopUp.programs as p}
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
					<svg xmlns="http://www.w3.org/2000/svg" height="64px" viewBox="0 -960 960 960" width="64px" fill="currentColor"><path d="M172.31-180Q142-180 121-201q-21-21-21-51.31v-455.38Q100-738 121-759q21-21 51.31-21h615.38Q818-780 839-759q21 21 21 51.31v455.38Q860-222 839-201q-21 21-51.31 21H172.31Zm0-60h615.38q4.62 0 8.46-3.85 3.85-3.84 3.85-8.46v-455.38q0-4.62-3.85-8.46-3.84-3.85-8.46-3.85H172.31q-4.62 0-8.46 3.85-3.85 3.84-3.85 8.46v455.38q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85ZM160-240v-480 480Zm286.15-40.77H760v-233.84H446.15v233.84Zm60-60v-113.85H700v113.85H506.15Z"/></svg>
					<h2>No pop-up selected</h2>
					<p>Pick a pop-up from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNew}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New PopUp
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
		font-size: 0.75rem;
		color: var(--text-3);
	}

	.item-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
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

	.panel-id {
		font-size: 0.8125rem;
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

	.form-input {
		width: 100%;
		padding: 9px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.form-input:focus {
		border-color: var(--accent);
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
		font-size: 0.8125rem;
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
		font-size: 0.6875rem;
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
