<script lang="ts">
	let { children } = $props();
</script>

{@render children()}

<style>
	:global(html, body) {
		height: 100%;
		overflow: hidden;
		margin: 0;
	}
</style>
