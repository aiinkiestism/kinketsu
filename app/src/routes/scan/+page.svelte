<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { llmConfig } from '$lib/stores/llm_config.svelte';
	import { subscriptions } from '$lib/stores/subscriptions.svelte';
	import { i18n, t } from '$lib/i18n.svelte';
	import type { BillingCycle, ParsedSubscriptionHint } from '$lib/types';

	let text = $state('');
	let scanning = $state(false);
	let error = $state<string | null>(null);
	let hint = $state<ParsedSubscriptionHint | null>(null);
	let saving = $state(false);
	let savedFlash = $state(false);

	function fromMinor(amount_minor: number, currency: string): number {
		return currency === 'JPY' ? amount_minor : amount_minor / 100;
	}

	function formatMoney(amount_minor: number, currency: string): string {
		try {
			return new Intl.NumberFormat(i18n.bcp47, {
				style: 'currency',
				currency,
				maximumFractionDigits: currency === 'JPY' ? 0 : 2
			}).format(fromMinor(amount_minor, currency));
		} catch {
			return `${amount_minor} ${currency}`;
		}
	}

	async function handleScan(e: SubmitEvent) {
		e.preventDefault();
		if (!text.trim()) return;
		scanning = true;
		error = null;
		hint = null;
		try {
			hint = await invoke<ParsedSubscriptionHint>('extract_subscription_from_text', {
				text
			});
		} catch (e) {
			error = String(e);
		} finally {
			scanning = false;
		}
	}

	async function handleSaveHint() {
		if (!hint) return;
		if (
			!hint.service_name ||
			hint.amount_minor === null ||
			!hint.currency ||
			!hint.billing_cycle
		) {
			error = t('scan.missing_fields');
			return;
		}
		saving = true;
		error = null;
		try {
			await subscriptions.create({
				name: hint.service_name,
				service_icon: null,
				plan: null,
				amount_minor: hint.amount_minor,
				currency: hint.currency,
				billing_cycle: hint.billing_cycle as BillingCycle,
				next_billing_date: null,
				started_at: hint.charged_at ?? null,
				payment_method_id: null,
				category_id: null,
				status: null,
				notes: hint.payment_method_hint ? `Payment hint: ${hint.payment_method_hint}` : null
			});
			savedFlash = true;
			hint = null;
			text = '';
			setTimeout(() => (savedFlash = false), 2000);
		} catch (e) {
			error = String(e);
		} finally {
			saving = false;
		}
	}

	onMount(() => llmConfig.load());
</script>

<div class="container">
	<header class="title">
		<h1>{t('scan.heading')}</h1>
		<p class="muted desc">{t('scan.description')}</p>
	</header>

	{#if !llmConfig.current && !llmConfig.loading}
		<article class="glass guard">
			<p>{t('scan.no_provider')}</p>
			<a href="/settings" class="link-btn">{t('scan.go_settings')}</a>
		</article>
	{:else}
		<form class="glass form" onsubmit={handleScan}>
			<textarea
				bind:value={text}
				rows="14"
				placeholder={t('scan.placeholder')}
				spellcheck="false"
			></textarea>
			<div class="actions">
				<button type="submit" class="primary-btn" disabled={scanning || !text.trim()}>
					{scanning ? t('common.loading') : t('scan.extract')}
				</button>
			</div>
		</form>

		{#if error}
			<p class="error">{t('common.error')}: {error}</p>
		{/if}

		{#if savedFlash}
			<p class="saved">{t('settings.saved')}</p>
		{/if}

		{#if hint}
			<article class="glass result">
				<h2>{t('scan.result_heading')}</h2>
				<dl class="kv">
					<dt>{t('form.name')}</dt>
					<dd>{hint.service_name ?? '—'}</dd>

					<dt>{t('form.amount')}</dt>
					<dd>
						{#if hint.amount_minor !== null && hint.currency}
							{formatMoney(hint.amount_minor, hint.currency)}
						{:else}
							—
						{/if}
					</dd>

					<dt>{t('form.currency')}</dt>
					<dd>{hint.currency ?? '—'}</dd>

					<dt>{t('form.billing_cycle')}</dt>
					<dd>{hint.billing_cycle ? t(`cycle.${hint.billing_cycle}`) : '—'}</dd>

					<dt>{t('subs.payment_method')}</dt>
					<dd>{hint.payment_method_hint ?? '—'}</dd>

					<dt>{t('scan.charged_at')}</dt>
					<dd>{hint.charged_at ?? '—'}</dd>
				</dl>
				<div class="actions">
					<button class="primary-btn" onclick={handleSaveHint} disabled={saving}>
						{saving ? t('common.loading') : t('scan.save_as_sub')}
					</button>
				</div>
			</article>
		{/if}
	{/if}
</div>

<style>
	.container {
		max-width: 760px;
		margin: 0 auto;
		padding: 3rem 2rem 6rem;
	}
	.title h1 {
		font-size: 2rem;
		font-weight: 700;
		margin: 0 0 0.5rem;
		letter-spacing: -0.02em;
	}
	.desc {
		margin: 0 0 2rem;
		color: var(--kk-text-muted);
		font-size: 0.95rem;
		line-height: 1.5;
	}
	.guard {
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}
	.guard p {
		margin: 0;
		color: var(--kk-text-muted);
	}
	.link-btn {
		display: inline-block;
		padding: 0.55rem 1rem;
		border-radius: var(--kk-radius-sm);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		text-decoration: none;
		font-weight: 600;
		font-size: 0.9rem;
	}
	.form {
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	textarea {
		width: 100%;
		padding: 0.85rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-family: ui-monospace, SFMono-Regular, monospace;
		font-size: 0.9rem;
		resize: vertical;
		min-height: 10rem;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.primary-btn {
		padding: 0.7rem 1.5rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
	}
	.primary-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.error {
		color: var(--color-accent-mochi);
		margin: 1rem 0 0;
		font-size: 0.9rem;
	}
	.saved {
		color: var(--color-accent-matcha);
		margin: 1rem 0 0;
		font-size: 0.9rem;
	}
	.result {
		margin-top: 1.25rem;
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
	}
	.result h2 {
		font-size: 1rem;
		font-weight: 600;
		margin: 0 0 1rem;
	}
	.kv {
		display: grid;
		grid-template-columns: max-content 1fr;
		column-gap: 1.5rem;
		row-gap: 0.5rem;
		margin: 0 0 1.5rem;
	}
	.kv dt {
		font-size: 0.85rem;
		color: var(--kk-text-muted);
	}
	.kv dd {
		margin: 0;
		font-size: 0.95rem;
		font-variant-numeric: tabular-nums;
	}
	@media (max-width: 640px) {
		.kv {
			grid-template-columns: 1fr;
			row-gap: 0.1rem;
		}
		.kv dd {
			margin-bottom: 0.4rem;
		}
	}
</style>
