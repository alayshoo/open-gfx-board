<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { socket } from '$lib/api/socket';
	import { imgUrl } from '$lib/api/api';

	const studioId = $derived(Number($page.url.searchParams.get('studio')));

	let activePath = $state<string | null>(null);
	let activeType = $state<string>('image');
	let visible = $state(false);

	onMount(() => {
		if (!studioId) return;

		socket.emit('join-studio-room', { studioId });

		function onOverlayActivated(data: any) {
			if (data.studioId !== studioId) return;
			const rawPath: string | null = data.graphicPath ?? null;
			activePath = rawPath ? imgUrl(rawPath) : null;
			// Detect media type from path extension
			const ext = rawPath?.split('.').pop()?.toLowerCase() ?? '';
			activeType = ['mp4', 'webm'].includes(ext) ? 'video' : 'image';
			visible = true;
		}

		function onOverlayDeactivated(data: any) {
			if (data && data.studioId !== studioId) return;
			visible = false;
			activePath = null;
		}

		socket.on('overlay-activated', onOverlayActivated);
		socket.on('overlay-deactivated', onOverlayDeactivated);

		return () => {
			socket.emit('leave-studio-room', { studioId });
			socket.off('overlay-activated', onOverlayActivated);
			socket.off('overlay-deactivated', onOverlayDeactivated);
		};
	});
</script>

<svelte:head>
	<style>
		html, body {
			margin: 0;
			padding: 0;
			background: transparent !important;
			overflow: hidden;
		}
	</style>
</svelte:head>

<div class="overlay-root" class:visible>
	{#if activePath}
		{#if activeType === 'video'}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video
				class="overlay-media"
				src={activePath}
				autoplay
				loop
				muted
			></video>
		{:else}
			<img class="overlay-media" src={activePath} alt="overlay" />
		{/if}
	{/if}
</div>

<style>
	.overlay-root {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		opacity: 0;
		transition: opacity 0.3s ease;
		pointer-events: none;
	}

	.overlay-root.visible {
		opacity: 1;
	}

	.overlay-media {
		max-width: 100vw;
		max-height: 100vh;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
