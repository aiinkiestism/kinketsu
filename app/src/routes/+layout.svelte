<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';
	import BackgroundBlobs from '$lib/components/BackgroundBlobs.svelte';
	import { i18n } from '$lib/i18n.svelte';

	let { children } = $props();

	onMount(() => i18n.init());
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="shell">
	<BackgroundBlobs />
	<main>
		{@render children()}
	</main>
</div>

<style>
	.shell {
		position: relative;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}
	main {
		position: relative;
		z-index: 1;
		height: 100%;
		width: 100%;
		overflow: auto;
	}
</style>
