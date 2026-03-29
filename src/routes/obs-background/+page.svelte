<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { socket } from '$lib/api/socket';
	import { imgUrl } from '$lib/api/api';

	const studioId = $derived(Number($page.url.searchParams.get('studio')));

	let bgPath = $state<string | null>(null);
	let bgType = $state<string>('image');
	let visible = $state(false);

	onMount(() => {
		if (!studioId) return;

		socket.emit('join-studio-room', { studioId });

		socket.on('program-selected', (data: any) => {
			if (data.studioId !== studioId) return;
			const rawPath: string | null = data.program?.background_graphics_path ?? null;
			bgPath = rawPath ? imgUrl(rawPath) : null;
			const ext = rawPath?.split('.').pop()?.toLowerCase() ?? '';
			bgType = ['mp4', 'webm'].includes(ext) ? 'video' : 'image';
			visible = !!bgPath;
		});

		socket.on('program-cleared', (data: any) => {
			if (data && data.studioId !== studioId) return;
			visible = false;
			bgPath = null;
		});

		return () => {
			socket.emit('leave-studio-room', { studioId });
			socket.off('program-selected');
			socket.off('program-cleared');
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

<div class="bg-root" class:visible>
	{#if bgPath}
		{#if bgType === 'video'}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video
				class="bg-media"
				src={bgPath}
				autoplay
				loop
				muted
			></video>
		{:else}
			<img class="bg-media" src={bgPath} alt="background" />
		{/if}
	{/if}
</div>

<style>
	.bg-root {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		opacity: 0;
		transition: opacity 0.5s ease;
		pointer-events: none;
	}

	.bg-root.visible {
		opacity: 1;
	}

	.bg-media {
		width: 100vw;
		height: 100vh;
		object-fit: cover;
	}
</style>
