<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import BackgroundBlobs from '$lib/components/BackgroundBlobs.svelte';

	let { children } = $props();
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
