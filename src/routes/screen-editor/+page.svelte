<script lang="ts">
	import { onMount } from 'svelte';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchScreens, imgUrl, uploadImage } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Screen } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';

	let screens = $state<Screen[]>([]);

	// Selection state
	let selectedId = $state<number | null>(null);
	let isCreatingNew = $state(false);

	// Edit state
	let editId = $state<number | null>(null);
	let editName = $state('');
	let editComments = $state('');
	let editAllowPopUps = $state(true);
	let editMediaType = $state('image');
	let editMediaPath = $state<string | null>(null);

	let saving = $state(false);
	let uploading = $state(false);
	let fileInput = $state() as unknown as HTMLInputElement;

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);

	onMount(() => {
		fetchScreens().then((data) => { screens = data; });

		socket.on('screen-created', (data: any) => {
			if (data.success) {
				if (!screens.some((s) => s.id === data.screen.id)) {
					screens = [data.screen, ...screens];
				}
				addToast('success', 'Screen created.');
				if (selectedId !== data.screen.id) {
					isCreatingNew = false;
					selectedId = data.screen.id;
					editId = data.screen.id;
					editName = data.screen.graphics_name;
					editComments = data.screen.comments ?? '';
					editAllowPopUps = data.screen.allow_popups;
					editMediaType = data.screen.media_type;
					editMediaPath = data.screen.graphics_path;
				}
			}
		});

		socket.on('screen-updated', (data: any) => {
			if (data.success) {
				screens = screens.map((s) => (s.id === data.screen.id ? data.screen : s));
				addToast('success', 'Screen saved.');
				if (selectedId === data.screen.id) {
					editName = data.screen.graphics_name;
					editComments = data.screen.comments ?? '';
					editAllowPopUps = data.screen.allow_popups;
					editMediaType = data.screen.media_type;
					editMediaPath = data.screen.graphics_path;
				}
			}
		});

		socket.on('screen-deleted', (data: any) => {
			if (data.success) {
				screens = screens.filter((s) => s.id !== data.id);
				addToast('success', 'Screen deleted.');
				if (selectedId === data.id) {
					selectedId = null;
					isCreatingNew = false;
				}
			}
		});

		return () => {
			socket.off('screen-created');
			socket.off('screen-updated');
			socket.off('screen-deleted');
		};
	});

	function openNew() {
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editComments = '';
		editAllowPopUps = true;
		editMediaType = 'image';
		editMediaPath = null;
	}

	function selectScreen(s: Screen) {
		isCreatingNew = false;
		selectedId = s.id;
		editId = s.id;
		editName = s.graphics_name;
		editComments = s.comments ?? '';
		editAllowPopUps = s.allow_popups;
		editMediaType = s.media_type;
		editMediaPath = s.graphics_path;
	}

	async function deleteCurrentScreen() {
		const s = screens.find((x) => x.id === selectedId);
		if (!s) return;
		if (!confirm(`Delete screen "${s.graphics_name}"?`)) return;
		const res = await fetch(`${getBackendUrl()}/screens/${s.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	async function save() {
		if (!editName.trim()) {
			addToast('error', 'Screen name is required.');
			return;
		}
		saving = true;
		try {
			if (isNew) {
				const res = await fetch(`${getBackendUrl()}/screens`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						comments: editComments.trim(),
						allow_popups: editAllowPopUps,
						media_type: editMediaType,
					}),
				});
				const data = await res.json();
				if (data.success) {
					if (!screens.some((x) => x.id === data.screen.id)) {
						screens = [data.screen, ...screens];
					}
					isCreatingNew = false;
					selectedId = data.screen.id;
					editId = data.screen.id;
					editName = data.screen.graphics_name;
					editComments = data.screen.comments ?? '';
					editAllowPopUps = data.screen.allow_popups;
					editMediaType = data.screen.media_type;
					editMediaPath = data.screen.graphics_path;
				} else {
					addToast('error', data.error ?? 'Create failed.');
				}
			} else {
				const res = await fetch(`${getBackendUrl()}/screens/${editId}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						name: editName.trim(),
						comments: editComments.trim(),
						allow_popups: editAllowPopUps,
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

	async function uploadScreenImage(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file || editId == null) return;
		uploading = true;
		try {
			const result = await uploadImage('/screens/upload-image', file, editId);
			if (result.success) {
				editMediaPath = result.imagePath;
				fetchScreens().then((data) => { screens = data; });
				addToast('success', 'Image uploaded.');
			} else {
				addToast('error', 'Upload failed.');
			}
		} catch {
			addToast('error', 'Upload failed.');
		} finally {
			uploading = false;
		}
	}
</script>

<div class="editor-wrap">
	{#if !IS_TAURI}
		<TopNav back={{ href: '/', label: 'Studios' }} />
	{/if}
	<div class="editor-body">

		<!-- ── Sidebar ── -->
		<aside class="sidebar">
			<div class="sidebar-header">
				<span class="sidebar-title">
					Screens
					<span class="badge">{screens.length}</span>
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
							<div class="item-thumb-empty">—</div>
						</div>
						<div class="item-info">
							<span class="item-name">New Screen…</span>
						</div>
					</div>
				{/if}
				{#each screens as s (s.id)}
					<button
						class="sidebar-item"
						class:selected={selectedId === s.id}
						onclick={() => selectScreen(s)}
					>
						<div class="item-thumb-wrap">
							{#if s.graphics_path}
								<MediaPreview class="item-thumb-img" src={imgUrl(s.graphics_path)} alt={s.graphics_name} />
							{:else}
								<div class="item-thumb-empty">—</div>
							{/if}
						</div>
						<div class="item-info">
							<span class="item-name">{s.graphics_name}</span>
							<span class="item-meta">{s.media_type} · {s.allow_popups ? 'PopUps' : 'No PopUps'}</span>
						</div>
					</button>
				{:else}
					{#if !isCreatingNew}
						<div class="sidebar-empty">No screens yet.<br/>Click "New" to get started.</div>
					{/if}
				{/each}
			</div>
		</aside>

		<!-- ── Main editor area ── -->
		<main class="editor-main">
			{#if hasSelection}
				<div class="editor-panel">
					<div class="panel-header">
						<div class="panel-title-area">
							<h1 class="panel-title">{isNew ? 'New Screen' : (editName || 'Untitled')}</h1>
							{#if !isNew}
								<span class="panel-id">ID #{editId}</span>
							{/if}
						</div>
						<div class="panel-header-end">
							<div class="panel-actions">
								{#if !isNew}
									<button class="btn btn-danger btn-sm" onclick={deleteCurrentScreen}>Delete</button>
								{/if}
								<button class="btn btn-ghost btn-sm" onclick={() => { selectedId = null; isCreatingNew = false; }}>
									Cancel
								</button>
								<button class="btn btn-primary" onclick={save} disabled={saving}>
									{saving ? 'Saving…' : isNew ? 'Create Screen' : 'Save Changes'}
								</button>
							</div>
						</div>
					</div>

					<div class="form-body">
						<div class="field-group">
							<label class="field-label" for="screen-name">Screen Name</label>
							<input id="screen-name" class="form-input" type="text" bind:value={editName} placeholder="e.g. Lower Third" />
						</div>

						<div class="field-group">
							<label class="field-label" for="screen-comments">Comments</label>
							<input id="screen-comments" class="form-input" type="text" bind:value={editComments} placeholder="Optional notes" />
						</div>

						<div class="field-group">
							<label class="field-label" for="screen-media-type">Media Type</label>
							<select id="screen-media-type" class="form-select" bind:value={editMediaType}>
								<option value="image">Image</option>
								<option value="video">Video</option>
							</select>
						</div>

						<div class="field-group">
							<div class="toggle-row">
								<span class="field-label">Allow PopUps</span>
								<label class="toggle">
									<input type="checkbox" bind:checked={editAllowPopUps} aria-label="Allow pop-ups on this screen" />
									<span class="toggle-track"></span>
								</label>
							</div>
							<p class="helper-text">When active, pop-ups can appear on top of this screen.</p>
						</div>

						{#if !isNew}
							<div class="field-group">
								<span class="field-label">Media File</span>
								{#if editMediaPath}
									<div class="preview-box">
										<MediaPreview src={imgUrl(editMediaPath)} alt={editName} />
									</div>
								{/if}
								<div class="img-actions">
									<input bind:this={fileInput} type="file" accept="image/*,video/webm,video/mp4" style="display:none" onchange={uploadScreenImage} />
									<button class="btn btn-secondary btn-sm" style="flex:1" onclick={() => fileInput.click()} disabled={uploading}>
										{uploading ? 'Uploading…' : 'Upload Media'}
									</button>
									{#if editMediaPath}
										<button class="btn btn-danger btn-sm" type="button" onclick={() => { editMediaPath = null; }}>Remove</button>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{:else}
				<div class="empty-state">
					<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" class="empty-icon">
						<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>
					</svg>
					<p class="empty-title">No screen selected</p>
					<p class="empty-hint">Pick a screen from the sidebar, or create a new one.</p>
					<button class="btn btn-primary btn-sm" onclick={openNew}>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
							<path d="M12 5v14M5 12h14"/>
						</svg>
						New Screen
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
		border: 1px solid var(--border-1);
	}

	:global(.item-thumb-img) {
		width: 44px;
		height: 28px;
		object-fit: cover;
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

	.panel-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 32px;
		padding-bottom: 24px;
		border-bottom: 1px solid var(--border-1);
		flex-wrap: wrap;
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

	.panel-header-end {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-shrink: 0;
		flex-wrap: wrap;
		justify-content: flex-end;
	}

	.panel-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.form-body {
		display: flex;
		flex-direction: column;
		gap: 28px;
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

	.form-select {
		padding: 9px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 13px;
		font-family: inherit;
		outline: none;
		cursor: pointer;
		max-width: 280px;
	}

	.form-select:focus {
		border-color: var(--accent);
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		max-width: 360px;
	}

	.helper-text {
		font-size: 11px;
		color: var(--text-3);
	}

	.toggle {
		display: flex;
		align-items: center;
		cursor: pointer;
	}

	.toggle input {
		display: none;
	}

	.toggle-track {
		width: 32px;
		height: 18px;
		background: var(--surface-3);
		border: 1px solid var(--border-2);
		border-radius: 999px;
		position: relative;
		transition: all 0.15s;
	}

	.toggle-track::after {
		content: '';
		position: absolute;
		top: 2px;
		left: 2px;
		width: 12px;
		height: 12px;
		border-radius: 50%;
		background: var(--text-3);
		transition: all 0.15s;
	}

	.toggle input:checked + .toggle-track {
		background: var(--accent-dim);
		border-color: var(--accent);
	}

	.toggle input:checked + .toggle-track::after {
		background: var(--accent);
		transform: translateX(14px);
	}

	.preview-box {
		width: 100%;
		max-width: 480px;
		aspect-ratio: 16/9;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border-1);
		border-radius: var(--r);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	:global(.preview-box img), :global(.preview-box video) {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.img-actions {
		display: flex;
		gap: 8px;
		max-width: 480px;
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
</style>
