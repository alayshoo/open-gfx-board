<script lang="ts">
	let {
		title = $bindable(),
		color = $bindable(),
		key = $bindable(),
		onclick,
	}: {
		title: string;
		color: string;
		key: string;
		onclick?: () => void;
	} = $props();

	function getTextColor(bg: string): string {
		const hex = bg.replace('#', '');
		const r = parseInt(hex.substring(0, 2), 16);
		const g = parseInt(hex.substring(2, 4), 16);
		const b = parseInt(hex.substring(4, 6), 16);
		const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
		return luminance > 0.5 ? '#000000' : '#ffffff';
	}

	const textColor = $derived(getTextColor(color));
</script>

<button class="button" style="background-color: {color}" {onclick}>
	<span class="text" style="color: {textColor}">{title}</span>
	<span class="key" style="color: {textColor}; opacity: 0.7">{key}</span>
</button>

<style>
	.button {
		container-type: inline-size;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		box-sizing: border-box;
		gap: 0.4rem;
		width: 100%;
		height: 100%;
		padding: 1rem;
		border: none;
		border-radius: 10px;
		overflow: hidden;
		cursor: pointer;
		transition: filter 0.12s, transform 0.1s;
	}

	.button:hover {
		filter: brightness(1.15);
	}

	.button:active {
		transform: scale(0.96);
		filter: brightness(0.9);
	}

	.text {
		font-size: clamp(1rem, 12cqi, 3rem);
		font-weight: 700;
		text-align: center;
		white-space: normal;
		overflow-wrap: break-word;
		line-height: 1.1;
	}

	.key {
		font-size: clamp(0.7rem, 7cqi, 1.6rem);
		font-weight: 500;
	}
</style>
