<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { llmConfig } from '$lib/stores/llm_config.svelte';
	import { exchangeRates } from '$lib/stores/exchange_rates.svelte';
	import { gmail } from '$lib/stores/gmail.svelte';
	import { paypal } from '$lib/stores/paypal.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import {
		LLM_PROVIDERS,
		LLM_PROVIDER_LABEL,
		LLM_DEFAULTS,
		isCloudProvider,
		type LlmConfig,
		type LlmProviderKind
	} from '$lib/types';

	function formatTimestamp(iso: string): string {
		try {
			return new Intl.DateTimeFormat(i18n.bcp47, {
				dateStyle: 'medium',
				timeStyle: 'short'
			}).format(new Date(iso));
		} catch {
			return iso;
		}
	}

	let provider = $state<LlmProviderKind>('claude');
	let apiKey = $state('');
	let endpoint = $state('');
	let model = $state('');
	let savedFlash = $state(false);

	let gmailClientId = $state('');
	let gmailClientSecret = $state('');
	let gmailSavedFlash = $state(false);

	let paypalClientId = $state('');
	let paypalClientSecret = $state('');
	let paypalSavedFlash = $state(false);

	async function handleSavePaypalCreds(e: SubmitEvent) {
		e.preventDefault();
		if (!paypalClientId.trim() || !paypalClientSecret.trim()) return;
		try {
			await paypal.saveCredentials({
				client_id: paypalClientId.trim(),
				client_secret: paypalClientSecret.trim()
			});
			paypalSavedFlash = true;
			setTimeout(() => (paypalSavedFlash = false), 2000);
		} catch {
			/* error in store */
		}
	}

	async function handleDisconnectPaypal() {
		await paypal.disconnect();
	}

	let checkingRenewals = $state(false);
	let renewalResult = $state<number | null>(null);
	let renewalError = $state<string | null>(null);

	async function handleCheckRenewals() {
		checkingRenewals = true;
		renewalResult = null;
		renewalError = null;
		try {
			renewalResult = await invoke<number>('check_renewals_now');
		} catch (e) {
			renewalError = String(e);
		} finally {
			checkingRenewals = false;
		}
	}

	async function handleSaveGmailCreds(e: SubmitEvent) {
		e.preventDefault();
		if (!gmailClientId.trim() || !gmailClientSecret.trim()) return;
		try {
			await gmail.saveCredentials({
				client_id: gmailClientId.trim(),
				client_secret: gmailClientSecret.trim()
			});
			gmailSavedFlash = true;
			setTimeout(() => (gmailSavedFlash = false), 2000);
		} catch {
			/* error in store */
		}
	}

	async function handleDisconnectGmail() {
		await gmail.disconnect();
	}

	function applyDefaultsForProvider(p: LlmProviderKind) {
		const d = LLM_DEFAULTS[p];
		if (isCloudProvider(p)) {
			endpoint = '';
		} else if (!endpoint) {
			endpoint = d.endpoint ?? '';
		}
		if (!model) model = d.model;
	}

	function onProviderChange() {
		applyDefaultsForProvider(provider);
	}

	async function handleSave(e: SubmitEvent) {
		e.preventDefault();
		const config: LlmConfig = isCloudProvider(provider)
			? { provider: provider as 'claude' | 'openai' | 'gemini', api_key: apiKey, model }
			: { provider: provider as 'ollama' | 'lmstudio', endpoint, model };
		try {
			await llmConfig.save(config);
			savedFlash = true;
			setTimeout(() => (savedFlash = false), 2000);
		} catch {
			/* error surfaced via store */
		}
	}

	onMount(async () => {
		await Promise.all([
			llmConfig.load(),
			exchangeRates.load(),
			gmail.load(),
			paypal.load()
		]);
		const cfg = llmConfig.current;
		if (cfg) {
			provider = cfg.provider;
			model = cfg.model;
			if (cfg.provider === 'claude' || cfg.provider === 'openai' || cfg.provider === 'gemini') {
				apiKey = cfg.api_key;
				endpoint = '';
			} else {
				endpoint = cfg.endpoint;
				apiKey = '';
			}
		} else {
			applyDefaultsForProvider(provider);
		}
		if (gmail.credentials) {
			gmailClientId = gmail.credentials.client_id;
			gmailClientSecret = gmail.credentials.client_secret;
		}
		if (paypal.credentials) {
			paypalClientId = paypal.credentials.client_id;
			paypalClientSecret = paypal.credentials.client_secret;
		}
	});
</script>

<div class="container">
	<header class="title">
		<h1>{t('settings.heading')}</h1>
	</header>

	<section class="glass section">
		<h2>{t('settings.llm_heading')}</h2>
		<p class="muted desc">{t('settings.llm_description')}</p>

		<form class="form" onsubmit={handleSave}>
			<label>
				<span>{t('settings.provider')}</span>
				<select bind:value={provider} onchange={onProviderChange}>
					{#each LLM_PROVIDERS as p (p)}
						<option value={p}>{LLM_PROVIDER_LABEL[p]}</option>
					{/each}
				</select>
			</label>

			{#if isCloudProvider(provider)}
				<label>
					<span>{t('settings.api_key')}</span>
					<input
						type="password"
						bind:value={apiKey}
						placeholder={LLM_DEFAULTS[provider].key_hint ?? ''}
						autocomplete="off"
						spellcheck="false"
					/>
				</label>
			{:else}
				<label>
					<span>{t('settings.endpoint')}</span>
					<input
						type="url"
						bind:value={endpoint}
						placeholder={LLM_DEFAULTS[provider].endpoint ?? ''}
					/>
				</label>
			{/if}

			<label>
				<span>{t('settings.model')}</span>
				<input
					type="text"
					bind:value={model}
					placeholder={LLM_DEFAULTS[provider].model}
					spellcheck="false"
				/>
			</label>

			<div class="actions">
				<button type="submit" class="save" disabled={llmConfig.saving}>
					{llmConfig.saving ? t('common.loading') : t('settings.save')}
				</button>
				{#if savedFlash}
					<span class="saved">{t('settings.saved')}</span>
				{/if}
			</div>

			{#if llmConfig.error}
				<p class="error">{t('common.error')}: {llmConfig.error}</p>
			{/if}
		</form>
	</section>

	<section class="glass section">
		<h2>{t('settings.rates_heading')}</h2>
		<p class="muted desc">{t('settings.rates_description')}</p>

		<div class="rates-status">
			<div>
				<span class="muted small">{t('settings.rates_last')}</span>
				<span class="rates-time">
					{exchangeRates.lastFetched
						? formatTimestamp(exchangeRates.lastFetched)
						: t('settings.rates_never')}
				</span>
			</div>
			{#if exchangeRates.items.length > 0}
				<span class="muted small"
					>{t('settings.rates_count', { count: exchangeRates.items.length })}</span
				>
			{/if}
		</div>

		<div class="actions">
			<button
				type="button"
				class="save"
				onclick={() => exchangeRates.refresh()}
				disabled={exchangeRates.refreshing}
			>
				{exchangeRates.refreshing ? t('common.loading') : t('settings.rates_refresh')}
			</button>
		</div>

		{#if exchangeRates.error}
			<p class="error">{t('common.error')}: {exchangeRates.error}</p>
		{/if}
	</section>

	<section class="glass section">
		<h2>{t('settings.gmail_heading')}</h2>
		<p class="muted desc">{t('settings.gmail_description')}</p>

		<form class="form" onsubmit={handleSaveGmailCreds}>
			<label>
				<span>{t('settings.gmail_client_id')}</span>
				<input
					type="text"
					bind:value={gmailClientId}
					autocomplete="off"
					spellcheck="false"
					placeholder="123…apps.googleusercontent.com"
				/>
			</label>
			<label>
				<span>{t('settings.gmail_client_secret')}</span>
				<input
					type="password"
					bind:value={gmailClientSecret}
					autocomplete="off"
					spellcheck="false"
				/>
			</label>

			<div class="actions">
				<button type="submit" class="save" disabled={gmail.saving}>
					{gmail.saving ? t('common.loading') : t('settings.gmail_save_creds')}
				</button>
				{#if gmailSavedFlash}
					<span class="saved">{t('settings.saved')}</span>
				{/if}
			</div>
		</form>

		<div class="rates-status" style="margin-top:1rem">
			<div>
				<span class="muted small">{t('settings.gmail_status')}</span>
				<span class="rates-time">
					{gmail.connected ? t('settings.gmail_connected') : t('settings.gmail_not_connected')}
				</span>
			</div>
			{#if gmail.connected}
				<button type="button" class="link-like" onclick={handleDisconnectGmail}>
					{t('settings.gmail_disconnect')}
				</button>
			{/if}
		</div>

		{#if gmail.error}
			<p class="error">{t('common.error')}: {gmail.error}</p>
		{/if}
	</section>

	<section class="glass section">
		<h2>{t('settings.paypal_heading')}</h2>
		<p class="muted desc">{t('settings.paypal_description')}</p>

		<form class="form" onsubmit={handleSavePaypalCreds}>
			<label>
				<span>{t('settings.gmail_client_id')}</span>
				<input type="text" bind:value={paypalClientId} autocomplete="off" spellcheck="false" />
			</label>
			<label>
				<span>{t('settings.gmail_client_secret')}</span>
				<input
					type="password"
					bind:value={paypalClientSecret}
					autocomplete="off"
					spellcheck="false"
				/>
			</label>

			<div class="actions">
				<button type="submit" class="save" disabled={paypal.saving}>
					{paypal.saving ? t('common.loading') : t('settings.gmail_save_creds')}
				</button>
				{#if paypalSavedFlash}
					<span class="saved">{t('settings.saved')}</span>
				{/if}
			</div>
		</form>

		<div class="rates-status" style="margin-top:1rem">
			<div>
				<span class="muted small">{t('settings.gmail_status')}</span>
				<span class="rates-time">
					{paypal.connected
						? t('settings.paypal_connected')
						: t('settings.paypal_not_connected')}
				</span>
			</div>
			{#if paypal.connected}
				<button type="button" class="link-like" onclick={handleDisconnectPaypal}>
					{t('settings.gmail_disconnect')}
				</button>
			{/if}
		</div>

		{#if paypal.error}
			<p class="error">{t('common.error')}: {paypal.error}</p>
		{/if}
	</section>

	<section class="glass section">
		<h2>{t('settings.notifications_heading')}</h2>
		<p class="muted desc">{t('settings.notifications_description')}</p>

		<div class="actions">
			<button type="button" class="save" onclick={handleCheckRenewals} disabled={checkingRenewals}>
				{checkingRenewals
					? t('common.loading')
					: t('settings.notifications_check_now')}
			</button>
			{#if renewalResult !== null}
				<span class="saved">
					{renewalResult === 0
						? t('settings.notifications_result_zero')
						: tn('settings.notifications_result', renewalResult)}
				</span>
			{/if}
		</div>

		{#if renewalError}
			<p class="error">{t('common.error')}: {renewalError}</p>
		{/if}
	</section>
</div>

<style>
	.container {
		max-width: 720px;
		margin: 0 auto;
		padding: 3rem 2rem 6rem;
	}
	.title h1 {
		font-size: 2rem;
		font-weight: 700;
		margin: 0 0 2rem;
		letter-spacing: -0.02em;
	}
	.section {
		padding: 1.75rem;
		border-radius: var(--kk-radius-md);
	}
	.section h2 {
		margin: 0 0 0.35rem;
		font-size: 1rem;
		font-weight: 600;
	}
	.desc {
		margin: 0 0 1.5rem;
		color: var(--kk-text-muted);
		font-size: 0.9rem;
		line-height: 1.5;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.form label {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		font-size: 0.85rem;
		color: var(--kk-text-muted);
	}
	.form input,
	.form select {
		padding: 0.65rem 0.8rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.95rem;
		font-family: inherit;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-top: 0.5rem;
	}
	.save {
		padding: 0.7rem 1.5rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
	}
	.save:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.saved {
		color: var(--color-accent-matcha);
		font-size: 0.9rem;
	}
	.error {
		color: var(--color-accent-mochi);
		margin: 0;
		font-size: 0.9rem;
	}
	.section + .section {
		margin-top: 1.25rem;
	}
	.rates-status {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.85rem 1rem;
		border-radius: var(--kk-radius-sm);
		background: var(--kk-surface-2);
		border: 1px solid var(--kk-stroke);
		margin-bottom: 1rem;
	}
	.rates-time {
		display: block;
		font-size: 0.95rem;
		font-variant-numeric: tabular-nums;
		margin-top: 0.15rem;
	}
	.small {
		font-size: 0.8rem;
	}
	.link-like {
		background: transparent;
		border: none;
		color: var(--kk-text-muted);
		text-decoration: underline;
		cursor: pointer;
		font-family: inherit;
		font-size: 0.85rem;
		padding: 0;
	}
</style>
