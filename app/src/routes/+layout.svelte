<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import favicon from '$lib/assets/favicon.svg';
	import BackgroundBlobs from '$lib/components/BackgroundBlobs.svelte';
	import { i18n, t } from '$lib/i18n.svelte';

	let { children } = $props();

	onMount(() => i18n.init());
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="shell">
	<BackgroundBlobs />
	<nav class="topnav glass-subtle">
		<a href="/" class="brand">kinketsu</a>
		<div class="nav-links">
			<a href="/" class:active={page.url.pathname === '/'}>{t('nav.dashboard')}</a>
			<a href="/inbox" class:active={page.url.pathname === '/inbox'}>{t('nav.inbox')}</a>
			<a href="/scan" class:active={page.url.pathname === '/scan'}>{t('nav.scan')}</a>
			<a href="/settings" class:active={page.url.pathname === '/settings'}
				>{t('nav.settings')}</a
			>
		</div>
	</nav>
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
		display: flex;
		flex-direction: column;
	}
	main {
		position: relative;
		z-index: 1;
		flex: 1 1 auto;
		min-height: 0;
		overflow: auto;
	}
	.topnav {
		position: relative;
		z-index: 2;
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.85rem 2rem;
		border-bottom: 1px solid var(--kk-stroke);
	}
	.topnav .brand {
		font-weight: 700;
		font-size: 1.05rem;
		color: var(--kk-text-primary);
		text-decoration: none;
		letter-spacing: -0.015em;
	}
	.topnav .nav-links {
		display: flex;
		gap: 0.25rem;
	}
	.topnav .nav-links a {
		color: var(--kk-text-muted);
		text-decoration: none;
		padding: 0.4rem 0.85rem;
		border-radius: var(--kk-radius-sm);
		font-size: 0.9rem;
		transition: color 120ms ease;
	}
	.topnav .nav-links a:hover {
		color: var(--kk-text-primary);
	}
	.topnav .nav-links a.active {
		color: var(--kk-text-primary);
		background: var(--kk-surface-2);
		border: 1px solid var(--kk-stroke);
	}
	@media (max-width: 640px) {
		.topnav {
			padding: 0.75rem 1.25rem;
		}
	}
</style>
