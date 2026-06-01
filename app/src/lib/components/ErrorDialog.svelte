<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { t } from '$lib/i18n.svelte';

	type Props = {
		open?: boolean;
		title?: string;
		message?: string;
		details?: string | null;
		actionLabel?: string | null;
		actionUrl?: string | null;
	};

	let {
		open = $bindable(false),
		title = '',
		message = '',
		details = null,
		actionLabel = null,
		actionUrl = null
	}: Props = $props();

	let dialog: HTMLDialogElement | undefined = $state();
	let showDetails = $state(false);
	let copied = $state(false);

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			dialog.showModal();
		} else if (!open && dialog.open) {
			dialog.close();
		}
	});

	function handleClose() {
		open = false;
		showDetails = false;
		copied = false;
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === dialog) handleClose();
	}

	async function copyDetails() {
		if (!details) return;
		try {
			await navigator.clipboard.writeText(details);
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			/* clipboard blocked; user can select manually */
		}
	}

	async function openExternal(url: string) {
		try {
			await invoke('open_url', { url });
		} catch {
			/* fall back to a plain anchor */
			window.open(url, '_blank', 'noopener');
		}
	}
</script>

<dialog bind:this={dialog} onclose={handleClose} onclick={handleBackdropClick}>
	<div class="content glass-strong">
		{#if title}
			<h2>{title}</h2>
		{/if}
		{#if message}
			<p class="message">{message}</p>
		{/if}

		{#if details}
			<button
				type="button"
				class="toggle"
				onclick={() => (showDetails = !showDetails)}
			>
				{showDetails ? t('common.hide_details') : t('common.show_details')}
			</button>
			{#if showDetails}
				<pre class="details">{details}</pre>
				<button type="button" class="copy" onclick={copyDetails}>
					{copied ? t('common.copied') : t('common.copy')}
				</button>
			{/if}
		{/if}

		<div class="actions">
			{#if actionUrl && actionLabel}
				<button
					type="button"
					class="action-btn"
					onclick={() => openExternal(actionUrl!)}
				>
					{actionLabel} ↗
				</button>
			{/if}
			<button type="button" class="close-btn" onclick={handleClose}>
				{t('common.close')}
			</button>
		</div>
	</div>
</dialog>

<style>
	dialog {
		border: none;
		background: transparent;
		padding: 0;
		max-width: 36rem;
		width: calc(100% - 2rem);
		color: inherit;
	}
	dialog::backdrop {
		background: rgba(0, 0, 0, 0.45);
		backdrop-filter: blur(10px);
		-webkit-backdrop-filter: blur(10px);
	}
	.content {
		padding: 1.75rem;
		border-radius: var(--kk-radius-md);
		color: var(--kk-text-primary);
	}
	.content h2 {
		margin: 0 0 0.85rem;
		font-size: 1.05rem;
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.message {
		margin: 0 0 1rem;
		line-height: 1.55;
		color: var(--kk-text-primary);
	}
	.toggle {
		background: transparent;
		border: none;
		color: var(--kk-text-muted);
		text-decoration: underline;
		cursor: pointer;
		font-family: inherit;
		font-size: 0.85rem;
		padding: 0;
		margin-bottom: 0.45rem;
	}
	.details {
		margin: 0.4rem 0 0.6rem;
		padding: 0.85rem 1rem;
		background: var(--kk-surface-2);
		border: 1px solid var(--kk-stroke);
		border-radius: var(--kk-radius-sm);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.75rem;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-word;
		max-height: 16rem;
		overflow: auto;
		color: var(--kk-text-primary);
	}
	.copy {
		padding: 0.35rem 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-muted);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.8rem;
	}
	.copy:hover {
		color: var(--kk-text-primary);
	}
	.actions {
		display: flex;
		gap: 0.6rem;
		justify-content: flex-end;
		margin-top: 1.25rem;
		flex-wrap: wrap;
	}
	.action-btn {
		padding: 0.6rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
		font-size: 0.9rem;
	}
	.close-btn {
		padding: 0.6rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: transparent;
		color: var(--kk-text-primary);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.9rem;
	}
</style>
