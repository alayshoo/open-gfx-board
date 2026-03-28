<script lang="ts">
	import '@fontsource/figtree/400.css';
	import '@fontsource/figtree/500.css';
	import '@fontsource/figtree/600.css';
	import '@fontsource/figtree/700.css';
	import '@fontsource/figtree/800.css';

	let { children } = $props();
</script>

{@render children()}

<style>
	:global(html, body) {
		font-family: 'Figtree', system-ui, -apple-system, 'Segoe UI', sans-serif;
	}
</style>
