<script lang="ts">
	import { onMount } from 'svelte';
	import { llmConfig } from '$lib/stores/llm_config.svelte';
	import { t } from '$lib/i18n.svelte';
	import {
		LLM_PROVIDERS,
		LLM_PROVIDER_LABEL,
		LLM_DEFAULTS,
		isCloudProvider,
		type LlmConfig,
		type LlmProviderKind
	} from '$lib/types';

	let provider = $state<LlmProviderKind>('claude');
	let apiKey = $state('');
	let endpoint = $state('');
	let model = $state('');
	let savedFlash = $state(false);

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
		await llmConfig.load();
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
</style>
