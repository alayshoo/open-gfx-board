<script lang="ts">
	import { uploadImage, imgUrl } from '$lib/api/api';
	import { addToast } from '$lib/toasts';
	import MediaPreview from './MediaPreview.svelte';

	let {
		endpoint,
		id,
		currentPath = null,
		onuploaded,
		inputId,
	}: {
		endpoint: string;
		id: number;
		currentPath?: string | null;
		onuploaded?: (path: string) => void;
		inputId?: string;
	} = $props();

	let dragover = $state(false);
	let uploading = $state(false);
	let preview = $state<string | null>(null);
	let isVideoFile = $state<boolean | undefined>(undefined);

	$effect(() => {
		preview = imgUrl(currentPath);
	});

	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragover = false;
		const file = e.dataTransfer?.files[0];
		if (file) doUpload(file);
	}

	function onChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (file) doUpload(file);
	}

	async function doUpload(file: File) {
		if (!file.type.startsWith('image/') && !file.type.startsWith('video/')) {
			addToast('error', 'Please select an image or video file.');
			return;
		}
		isVideoFile = file.type.startsWith('video/');
		uploading = true;
		// Show local preview immediately
		preview = URL.createObjectURL(file);
		try {
			const result = await uploadImage(endpoint, file, id);
			if (result.success) {
				onuploaded?.(result.imagePath);
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

	let inputEl: HTMLInputElement;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="upload-zone"
	class:dragover
	class:has-preview={preview}
	ondragover={(e) => { e.preventDefault(); dragover = true; }}
	ondragleave={() => { dragover = false; }}
	ondrop={onDrop}
	onclick={() => inputEl.click()}
	role="button"
	tabindex="0"
	onkeydown={(e) => e.key === 'Enter' && inputEl.click()}
>
	{#if preview}
		<MediaPreview class="preview" src={preview} alt="Preview" isVideo={isVideoFile} />
		<div class="preview-overlay">
			{#if uploading}
				<span class="uploading-label">Uploading…</span>
			{:else}
				<span class="change-label">Change media</span>
			{/if}
		</div>
	{:else}
		<div class="empty">
			<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12"/>
			</svg>
			{#if uploading}
				<span>Uploading…</span>
			{:else}
				<span>Upload media</span>
				<span class="hint">Click or drag and drop</span>
			{/if}
		</div>
	{/if}
	<input
		id={inputId}
		bind:this={inputEl}
		type="file"
		accept="image/*,video/mp4,video/webm"
		onchange={onChange}
		style="display:none"
	/>
</div>

<style>
	.upload-zone {
		position: relative;
		width: 100%;
		height: 120px;
		border: 1.5px dashed var(--border-2);
		border-radius: var(--r);
		overflow: hidden;
		cursor: pointer;
		transition: border-color 0.15s, background 0.15s;
		background: var(--surface-2);
	}

	.upload-zone:hover,
	.upload-zone.dragover {
		border-color: var(--accent);
		background: var(--accent-dim);
	}

	.empty {
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		color: var(--text-3);
		font-size: 12px;
	}

	.hint {
		font-size: 11px;
		color: var(--text-3);
		opacity: 0.6;
	}

	:global(.preview) {
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: var(--surface-2);
	}

	.preview-overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.upload-zone:hover .preview-overlay {
		opacity: 1;
	}

	.change-label,
	.uploading-label {
		font-size: 12px;
		font-weight: 600;
		color: #fff;
	}
</style>
