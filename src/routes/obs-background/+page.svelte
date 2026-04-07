<script lang="ts">
	import { onMount } from 'svelte';
	import { socket } from '$lib/api/socket';
	import { imgUrl } from '$lib/api/api';
	import type { StudioState } from '$lib/types';

	let bgPath = $state<string | null>(null);
	let bgType = $state<string>('image');
	let visible = $state(false);

	function applyProgram(program: any) {
		const rawPath: string | null = program?.background_graphics_path ?? null;
		bgPath = rawPath ? imgUrl(rawPath) : null;
		const ext = rawPath?.split('.').pop()?.toLowerCase() ?? '';
		bgType = ['mp4', 'webm'].includes(ext) ? 'video' : 'image';
		visible = !!bgPath;
	}

	onMount(() => {
		socket.emit('join-studio-room', {});
		socket.emit('get-studio-state', {});

		function onStudioState(data: StudioState) {
			applyProgram(data.program);
		}

		function onProgramSelected(data: any) {
			applyProgram(data.program);
		}

		function onProgramCleared(_data: any) {
			visible = false;
			bgPath = null;
		}

		socket.on('studio-state', onStudioState);
		socket.on('program-selected', onProgramSelected);
		socket.on('program-cleared', onProgramCleared);

		return () => {
			socket.emit('leave-studio-room', {});
			socket.off('studio-state', onStudioState);
			socket.off('program-selected', onProgramSelected);
			socket.off('program-cleared', onProgramCleared);
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
