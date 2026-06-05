<script lang="ts">
	// A small glassmorphism info popover. Renders an ⓘ trigger; the tip shows on
	// hover and on focus (keyboard / touch), so it works without a pointer.
	let { text }: { text: string } = $props();
	let open = $state(false);
</script>

<span
	class="tip-wrap"
	role="note"
	onmouseenter={() => (open = true)}
	onmouseleave={() => (open = false)}
>
	<button
		type="button"
		class="tip-trigger"
		aria-label={text}
		onfocus={() => (open = true)}
		onblur={() => (open = false)}
		onclick={() => (open = !open)}
	>
		ⓘ
	</button>
	{#if open}
		<span class="tip glass" role="tooltip">{text}</span>
	{/if}
</span>

<style>
	.tip-wrap {
		position: relative;
		display: inline-flex;
		align-items: center;
	}
	.tip-trigger {
		border: none;
		background: transparent;
		color: var(--kk-text-muted);
		cursor: help;
		font-size: 0.85rem;
		line-height: 1;
		padding: 0 0.15rem;
		font-family: inherit;
	}
	.tip-trigger:hover {
		color: var(--kk-text-primary);
	}
	.tip {
		position: absolute;
		bottom: calc(100% + 0.4rem);
		left: 50%;
		transform: translateX(-50%);
		z-index: 20;
		width: max-content;
		max-width: 18rem;
		padding: 0.6rem 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		color: var(--kk-text-primary);
		font-size: 0.78rem;
		line-height: 1.45;
		font-weight: 400;
		text-align: left;
		white-space: normal;
		box-shadow: 0 8px 24px oklch(0 0 0 / 0.18);
		pointer-events: none;
	}
</style>
