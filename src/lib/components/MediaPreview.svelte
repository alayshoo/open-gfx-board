<script lang="ts">
	let {
		src = null,
		alt = '',
		class: className = '',
		style = '',
		id = '',
		isVideo = undefined
	}: {
		src?: string | null;
		alt?: string;
		class?: string;
		style?: string;
		id?: string;
		isVideo?: boolean | undefined;
	} = $props();

	let _isVideo = $derived(
		isVideo !== undefined 
			? isVideo 
			: (src && typeof src === 'string' && (src.toLowerCase().endsWith('.webm') || src.toLowerCase().endsWith('.mp4')))
	);
</script>

{#if _isVideo}
	<!-- svelte-ignore a11y_media_has_caption -->
	<video {src} class={className} {style} {id} loop autoplay muted playsinline></video>
{:else if src}
	<img {src} {alt} class={className} {style} {id} />
{/if}
