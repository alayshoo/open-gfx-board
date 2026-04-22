<script lang="ts">
	import { onMount } from 'svelte';
	import { beforeNavigate } from '$app/navigation';
	import TopNav from '$lib/components/TitleBarWeb.svelte';
	import { socket } from '$lib/api/socket';
	import { fetchScreens, imgUrl, uploadImage } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import type { Screen } from '$lib/types';
	import MediaPreview from '$lib/components/MediaPreview.svelte';
	import { getBackendUrl } from '$lib/bridge';
	import { IS_TAURI } from '$lib/bridge';
	import { showConfirm } from '$lib/confirm';

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
	let editMediaPathVertical = $state<string | null>(null);
	let editHtmlContent = $state('');

	let saving = $state(false);
	let uploading = $state(false);
	let uploadingVertical = $state(false);
	let fileInput = $state() as unknown as HTMLInputElement;
	let fileInputVertical = $state() as unknown as HTMLInputElement;

	const isNew = $derived(isCreatingNew);
	const hasSelection = $derived(isCreatingNew || selectedId !== null);
	const selectedScreen = $derived(screens.find(s => s.id === selectedId) ?? null);
	const isPluginScreen = $derived(!isNew && selectedScreen?.plugin_id != null);

	/* ─── Dirty tracking ─────────────────────────────────── */
	let savedSnapshot = $state('');

	function makeSnapshot(): string {
		return JSON.stringify([editName, editComments, editAllowPopUps, editMediaType, editHtmlContent]);
	}

	function takeSnapshot() {
		savedSnapshot = makeSnapshot();
	}

	// Plugin screens are read-only, never dirty
	const isDirty = $derived(hasSelection && !isPluginScreen && makeSnapshot() !== savedSnapshot);

	beforeNavigate(({ cancel }) => {
		if (isDirty && !window.confirm('You have unsaved changes. Leave this page?')) {
			cancel();
		}
	});

	// Split screens into user-created and plugin-bundled
	const userScreens = $derived(screens.filter(s => s.plugin_id == null));
	const pluginScreens = $derived(screens.filter(s => s.plugin_id != null));

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
					editMediaPathVertical = data.screen.graphics_path_vertical ?? null;
					editHtmlContent = data.screen.html_content ?? '';
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
					editMediaPathVertical = data.screen.graphics_path_vertical ?? null;
					editHtmlContent = data.screen.html_content ?? '';
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

	async function openNew() {
		if (isDirty) {
			const ok = await showConfirm({ title: 'Unsaved Changes', message: 'Discard unsaved changes and create a new screen?', confirmLabel: 'Discard' });
			if (!ok) return;
		}
		isCreatingNew = true;
		selectedId = null;
		editId = null;
		editName = '';
		editComments = '';
		editAllowPopUps = true;
		editMediaType = 'image';
		editMediaPath = null;
		editMediaPathVertical = null;
		editHtmlContent = '';
		takeSnapshot();
	}

	async function selectScreen(s: Screen) {
		if (isDirty) {
			const ok = await showConfirm({ title: 'Unsaved Changes', message: `Discard unsaved changes and switch to "${s.graphics_name}"?`, confirmLabel: 'Discard' });
			if (!ok) return;
		}
		isCreatingNew = false;
		selectedId = s.id;
		editId = s.id;
		editName = s.graphics_name;
		editComments = s.comments ?? '';
		editAllowPopUps = s.allow_popups;
		editMediaType = s.media_type;
		editMediaPath = s.graphics_path;
		editMediaPathVertical = s.graphics_path_vertical ?? null;
		editHtmlContent = s.html_content ?? '';
		takeSnapshot();
	}

	async function cancelEdit() {
		if (isDirty) {
			const ok = await showConfirm({ title: 'Unsaved Changes', message: 'Discard unsaved changes?', confirmLabel: 'Discard' });
			if (!ok) return;
		}
		selectedId = null;
		isCreatingNew = false;
	}

	async function deleteCurrentScreen() {
		const s = screens.find((x) => x.id === selectedId);
		if (!s) return;
		if (!await showConfirm({ title: 'Delete Screen', message: `Delete screen "${s.graphics_name}"? This cannot be undone.`, confirmLabel: 'Delete' })) return;
		const res = await fetch(`${getBackendUrl()}/screens/${s.id}`, { method: 'DELETE' });
		const data = await res.json();
		if (!data.success) addToast('error', data.error ?? 'Delete failed.');
	}

	async function duplicateCurrentScreen() {
		const s = screens.find((x) => x.id === selectedId);
		if (!s) return;
		try {
			const res = await fetch(`${getBackendUrl()}/screens/${s.id}/duplicate`, { method: 'POST' });
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
				editMediaPathVertical = data.screen.graphics_path_vertical ?? null;
				editHtmlContent = data.screen.html_content ?? '';
				takeSnapshot();
				addToast('success', 'Screen duplicated — you can now edit your copy.');
			} else {
				addToast('error', data.error ?? 'Duplicate failed.');
			}
		} catch {
			addToast('error', 'Request failed.');
		}
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
						html_content: editMediaType === 'html' ? editHtmlContent : null,
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
					editHtmlContent = data.screen.html_content ?? '';
					takeSnapshot();
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
						html_content: editMediaType === 'html' ? editHtmlContent : null,
					}),
				});
				const data = await res.json();
				if (data.success) takeSnapshot();
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

	async function uploadScreenImageVertical(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file || editId == null) return;
		uploadingVertical = true;
		try {
			const result = await uploadImage('/screens/upload-image-vertical', file, editId);
			if (result.success) {
				editMediaPathVertical = result.imagePath;
				fetchScreens().then((data) => { screens = data; });
				addToast('success', 'Vertical alt uploaded.');
			} else {
				addToast('error', 'Upload failed.');
			}
		} catch {
			addToast('error', 'Upload failed.');
		} finally {
			uploadingVertical = false;
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

				<!-- User screens section -->
				{#if pluginScreens.length > 0}
					<div class="sidebar-section-label">Your Screens</div>
				{/if}

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

				{#each userScreens as s (s.id)}
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
					{#if !isCreatingNew && pluginScreens.length === 0}
						<div class="sidebar-empty">No screens yet.<br/>Click "New" to get started.</div>
					{/if}
				{/each}

				<!-- Plugin screens section -->
				{#if pluginScreens.length > 0}
					<div class="sidebar-section-label sidebar-section-label--plugin">
						<svg xmlns="http://www.w3.org/2000/svg" height="12px" viewBox="0 -960 960 960" width="12px" fill="currentColor"><path d="M455.38-200h49.24v-78.62L640-414v-173.69q0-4.62-3.85-8.46-3.84-3.85-8.46-3.85H332.31q-4.62 0-8.46 3.85-3.85 3.84-3.85 8.46V-414l135.38 135.38V-200Zm-59.99 60v-113.08L260-388.46v-199.23q0-29.92 21.19-51.12Q302.39-660 332.31-660h44.61l-29.99 30v-190h59.99v160h146.16v-160h59.99v190l-29.99-30h44.61q29.92 0 51.12 21.19Q700-617.61 700-587.69v199.23L564.61-253.08V-140H395.39ZM480-400Z"/></svg>
						From Plugins
						<span class="badge-sm">{pluginScreens.length}</span>
					</div>
					{#each pluginScreens as s (s.id)}
						<button
							class="sidebar-item sidebar-item--plugin"
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
								<div class="item-name-row">
									<span class="item-name">{s.graphics_name}</span>
									<span class="chip-plugin">Plugin</span>
								</div>
								<span class="item-meta">{s.media_type}</span>
							</div>
						</button>
					{/each}
				{/if}

			</div>
		</aside>

		<!-- ── Main editor area ── -->
		<main class="editor-main">
			{#if hasSelection}
				<div class="editor-panel">

					{#if isPluginScreen}
						<!-- ── Plugin screen: read-only view ── -->
						<div class="plugin-notice">
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
								<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
							</svg>
							<div>
								<strong>Plugin Screen</strong> — This screen is bundled with a plugin and cannot be edited directly.
								Duplicate it to create your own editable copy.
							</div>
						</div>

						<div class="panel-header">
							<div class="panel-title-area">
								<h1>{editName || 'Untitled'}</h1>
								<span class="panel-id">ID #{editId}</span>
								<span class="chip-plugin chip-plugin--lg">Plugin</span>
							</div>
							<div class="panel-header-end">
								<div class="panel-actions">
									<button class="btn btn-ghost btn-sm" onclick={cancelEdit}>
										Cancel
									</button>
									<button class="btn btn-primary" onclick={duplicateCurrentScreen}>
										<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
											<rect x="9" y="9" width="13" height="13" rx="2"/>
											<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
										</svg>
										Duplicate to Edit
									</button>
								</div>
							</div>
						</div>

						<div class="form-body">
							<div class="field-group">
								<span class="field-label">Screen Name</span>
								<div class="readonly-value">{editName}</div>
							</div>

							{#if editComments}
								<div class="field-group">
									<span class="field-label">Comments</span>
									<div class="readonly-value">{editComments}</div>
								</div>
							{/if}

							<div class="field-group">
								<span class="field-label">Media Type</span>
								<div class="readonly-value">{editMediaType}</div>
							</div>

							<div class="field-group">
								<span class="field-label">Allow PopUps</span>
								<div class="readonly-value">{editAllowPopUps ? 'Yes' : 'No'}</div>
							</div>

							{#if editMediaType === 'html' && editHtmlContent}
								<div class="field-group">
									<span class="field-label">HTML Content</span>
									<textarea
										class="form-input html-editor form-input--readonly"
										value={editHtmlContent}
										readonly
										spellcheck="false"
										aria-label="HTML content (read-only)"
									></textarea>
								</div>
							{/if}

							{#if !isNew}
								{#if screens.find(s => s.id === editId)?.programs?.length}
									{@const currentScreen = screens.find(s => s.id === editId)!}
									<div class="field-group">
										<span class="field-label">Used in Programs</span>
										<div class="prog-pills">
											{#each currentScreen.programs as p}
												<span class="prog-pill">{p.name}</span>
											{/each}
										</div>
									</div>
								{/if}
							{/if}
						</div>

					{:else}
						<!-- ── Normal editable screen ── -->
						<div class="panel-header">
							<div class="panel-title-area">
								<h1>{isNew ? 'New Screen' : (editName || 'Untitled')}</h1>
								{#if isDirty}
									<span class="unsaved-badge">Unsaved</span>
								{/if}
								{#if !isNew}
									<span class="panel-id">ID #{editId}</span>
								{/if}
							</div>
							<div class="panel-header-end">
								<div class="panel-actions">
									{#if !isNew}
										<button class="btn btn-danger btn-sm" onclick={deleteCurrentScreen}>Delete</button>
									{/if}
									<button class="btn btn-ghost btn-sm" onclick={cancelEdit}>
										Cancel
									</button>
									<button class="btn btn-primary" onclick={save} disabled={saving || uploading}>
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
									<option value="html">HTML</option>
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
								<p>When active, pop-ups can appear on top of this screen.</p>
							</div>

							{#if editMediaType === 'html'}
								<div class="field-group">
									<label class="field-label" for="screen-html">HTML Content</label>
									<textarea
										id="screen-html"
										class="form-input html-editor"
										bind:value={editHtmlContent}
										placeholder={'<div style="color: white; font-size: 48px;">\n  Hello World\n</div>'}
										spellcheck="false"
									></textarea>
									<p class="field-hint-block">
										Full-screen overlay. Use <code>{`{{var:program_name}}`}</code>, <code>{`{{var:current_time}}`}</code>, or <code>{`{{db:table.column:id}}`}</code> for dynamic content.
									</p>
								</div>
							{/if}

							{#if !isNew && editMediaType !== 'html'}
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

								<div class="field-group">
									<span class="field-label">Vertical Alt <span class="field-hint">(portrait / 9:16)</span></span>
									<p class="field-hint-block">Optional alternate media used by the <code>/obs-vertical</code> browser source. Leave empty to reuse the main media above.</p>
									{#if editMediaPathVertical}
										<div class="preview-box preview-box--vertical">
											<MediaPreview src={imgUrl(editMediaPathVertical)} alt="{editName} vertical" />
										</div>
									{/if}
									<div class="img-actions">
										<input bind:this={fileInputVertical} type="file" accept="image/*,video/webm,video/mp4" style="display:none" onchange={uploadScreenImageVertical} />
										<button class="btn btn-secondary btn-sm" style="flex:1" onclick={() => fileInputVertical.click()} disabled={uploadingVertical}>
											{uploadingVertical ? 'Uploading…' : editMediaPathVertical ? 'Replace Vertical' : 'Upload Vertical Alt'}
										</button>
										{#if editMediaPathVertical}
											<button class="btn btn-danger btn-sm" type="button" onclick={() => { editMediaPathVertical = null; }}>Remove</button>
										{/if}
									</div>
								</div>
							{/if}

							{#if !isNew}
								{#if screens.find(s => s.id === editId)?.programs?.length}
									{@const currentScreen = screens.find(s => s.id === editId)!}
									<div class="field-group">
										<span class="field-label">Used in Programs</span>
										<div class="prog-pills">
											{#each currentScreen.programs as p}
												<span class="prog-pill">{p.name}</span>
											{/each}
										</div>
									</div>
								{/if}
							{/if}
						</div>
					{/if}

				</div>
			{:else}
				<div class="empty-state">
					<svg xmlns="http://www.w3.org/2000/svg" height="64px" viewBox="0 -960 960 960" width="64px" fill="currentColor"><path d="M610-326.15h143.85V-470h-47.7v96.15H610v47.7ZM206.15-570h47.7v-96.15H350v-47.7H206.15V-570ZM340-140v-80H172.31Q142-220 121-241q-21-21-21-51.31v-455.38Q100-778 121-799q21-21 51.31-21h615.38Q818-820 839-799q21 21 21 51.31v455.38Q860-262 839-241q-21 21-51.31 21H620v80H340ZM172.31-280h615.38q4.62 0 8.46-3.85 3.85-3.84 3.85-8.46v-455.38q0-4.62-3.85-8.46-3.84-3.85-8.46-3.85H172.31q-4.62 0-8.46 3.85-3.85 3.84-3.85 8.46v455.38q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85ZM160-280v-480 480Z"/></svg>
					<h2>No screen selected</h2>
					<p>Pick a screen from the sidebar, or create a new one.</p>
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

	.sidebar-list {
		flex: 1;
		overflow-y: auto;
	}

	/* Section labels dividing user vs plugin screens */
	.sidebar-section-label {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 14px 4px;
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-3);
		margin-top: 4px;
	}

	.sidebar-section-label--plugin {
		margin-top: 8px;
		padding-top: 12px;
		border-top: 1px solid var(--border-1);
		color: var(--text-3);
	}

	.sidebar-section-label--plugin svg {
		opacity: 0.7;
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

	/* Plugin items get a subtly different treatment */
	.sidebar-item--plugin {
		opacity: 0.9;
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

	.item-name-row {
		display: flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
	}

	/* ── Plugin chip (sidebar + panel) ── */
	.chip-plugin {
		display: inline-flex;
		align-items: center;
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
		border-radius: 3px;
		padding: 1px 5px;
		white-space: nowrap;
		flex-shrink: 0;
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.chip-plugin--lg {
		font-size: 0.7rem;
		padding: 2px 7px;
		border-radius: 4px;
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

	/* ── Plugin notice banner ── */
	.plugin-notice {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		background: color-mix(in srgb, var(--accent) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
		border-radius: var(--r);
		padding: 12px 16px;
		font-size: 0.8125rem;
		color: var(--text-2);
		margin-bottom: 28px;
	}

	.plugin-notice svg {
		flex-shrink: 0;
		margin-top: 1px;
		color: var(--accent);
		opacity: 0.8;
	}

	.plugin-notice strong {
		color: var(--text-1);
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
		gap: 10px;
		min-width: 0;
		flex-wrap: wrap;
	}

	.panel-id {
		font-size: 0.8125rem;
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

	/* Read-only input appearance */
	.form-input--readonly {
		cursor: default;
		color: var(--text-2);
		background: var(--surface-1);
	}

	.form-input--readonly:focus {
		border-color: var(--border-1);
	}

	/* Inline read-only value (non-input display) */
	.readonly-value {
		padding: 9px 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-2);
		font-size: 0.875rem;
	}

	.form-select {
		padding: 9px 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-1);
		border-radius: var(--r-sm);
		color: var(--text-1);
		font-size: 0.875rem;
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

	.preview-box--vertical {
		aspect-ratio: 9/16;
		max-width: 160px;
	}

	.field-hint {
		font-weight: 400;
		font-size: 0.6875rem;
		color: var(--text-3);
		text-transform: none;
		letter-spacing: normal;
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

	/* ── HTML editor ── */
	.html-editor {
		min-height: 200px;
		resize: vertical;
		font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
		font-size: 0.8125rem;
		line-height: 1.5;
		tab-size: 2;
		white-space: pre;
	}

	.field-hint-block {
		font-size: 0.75rem;
		color: var(--text-3);
		line-height: 1.4;
		margin: 0;
	}

	.field-hint-block code {
		font-size: 0.6875rem;
		background: var(--surface-3);
		padding: 1px 5px;
		border-radius: 3px;
		border: 1px solid var(--border-1);
	}

	/* ── Unsaved badge ── */
	.unsaved-badge {
		display: inline-flex;
		align-items: center;
		padding: 2px 8px;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		color: var(--accent);
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
		border-radius: 20px;
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
</style>
