<script lang="ts">
	import { onMount } from 'svelte';
	import { detectionEvents } from '$lib/stores/detection_events.svelte';
	import { gmail } from '$lib/stores/gmail.svelte';
	import { paypal } from '$lib/stores/paypal.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import MonthRangeSelect from '$lib/components/MonthRangeSelect.svelte';
	import type { DetectionSource, YearMonth } from '$lib/types';

	let scanRange = $state<YearMonth[]>([]);
	let scanFeedback = $state<string | null>(null);

	async function handleConnectGmail() {
		try {
			await gmail.connect();
		} catch {
			/* error in store */
		}
	}

	async function handleRunScan() {
		scanFeedback = null;
		if (!gmail.credentials) {
			scanFeedback = t('inbox.scan_needs_creds');
			return;
		}
		if (!gmail.connected) {
			scanFeedback = t('inbox.scan_needs_connection');
			return;
		}
		if (scanRange.length === 0) {
			scanFeedback = t('inbox.scan_needs_range');
			return;
		}
		try {
			const created = await gmail.runScan(scanRange);
			if (created === 0) {
				scanFeedback = t('inbox.scan_complete_zero');
			} else {
				scanFeedback = tn('inbox.scan_complete', created);
			}
			await detectionEvents.load();
		} catch {
			/* error in store */
		}
	}

	let pending = $derived(detectionEvents.items.filter((e) => e.status === 'pending'));
	let reviewed = $derived(
		detectionEvents.items.filter((e) => e.status !== 'pending').slice(0, 20)
	);

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

	function fromMinor(amount_minor: number, currency: string): number {
		return currency === 'JPY' ? amount_minor : amount_minor / 100;
	}

	function formatMoney(amount_minor: number | null, currency: string | null): string {
		if (amount_minor === null || !currency) return '—';
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

	function sourceLabel(s: DetectionSource): string {
		return t(`source.${s}`);
	}

	async function handleConnectPaypal() {
		try {
			await paypal.connect();
		} catch {
			/* error in store */
		}
	}

	onMount(() => {
		detectionEvents.load();
		gmail.load();
		paypal.load();
	});
</script>

<div class="container">
	<header class="title">
		<h1>{t('inbox.heading')}</h1>
		<p class="muted desc">{t('inbox.description')}</p>
	</header>

	<section class="glass section">
		<h2>{t('inbox.sources_heading')}</h2>
		<p class="muted desc-small">{t('inbox.sources_description')}</p>
		<div class="source-grid">
			<article class="source-card">
				<div>
					<h3>Gmail</h3>
					{#if gmail.connected}
						<p class="muted small connected">● {t('inbox.gmail_connected')}</p>
					{:else if !gmail.credentials}
						<p class="muted small">{t('inbox.scan_needs_creds')}</p>
					{:else}
						<p class="muted small">{t('inbox.gmail_coming_soon')}</p>
					{/if}
				</div>
				{#if gmail.connected}
					<button type="button" class="secondary" onclick={() => gmail.disconnect()}
						>{t('inbox.gmail_disconnect')}</button
					>
				{:else}
					<button
						type="button"
						onclick={handleConnectGmail}
						disabled={!gmail.credentials || gmail.connecting}
					>
						{gmail.connecting ? t('inbox.connect_gmail_loading') : t('inbox.connect_gmail')}
					</button>
				{/if}
			</article>
			<article class="source-card">
				<div>
					<h3>PayPal</h3>
					{#if paypal.connected}
						<p class="muted small connected">● {t('settings.paypal_connected')}</p>
					{:else if !paypal.credentials}
						<p class="muted small">{t('inbox.scan_needs_creds')}</p>
					{:else}
						<p class="muted small">{t('inbox.paypal_coming_soon')}</p>
					{/if}
				</div>
				{#if paypal.connected}
					<button type="button" class="secondary" onclick={() => paypal.disconnect()}
						>{t('inbox.gmail_disconnect')}</button
					>
				{:else}
					<button
						type="button"
						onclick={handleConnectPaypal}
						disabled={!paypal.credentials || paypal.connecting}
					>
						{paypal.connecting ? t('inbox.connect_gmail_loading') : t('inbox.connect_paypal')}
					</button>
				{/if}
			</article>
		</div>
		{#if gmail.error}
			<p class="error">{t('common.error')}: {gmail.error}</p>
		{/if}
	</section>

	<section class="glass section">
		<h2>{t('inbox.range_heading')}</h2>
		<p class="muted desc-small">{t('inbox.range_description')}</p>
		<MonthRangeSelect bind:value={scanRange} />
		<div class="scan-actions">
			<button
				type="button"
				class="scan-btn"
				onclick={handleRunScan}
				disabled={gmail.scanning || scanRange.length === 0}
			>
				{gmail.scanning ? t('inbox.scan_running') : t('inbox.scan_run')}
			</button>
			{#if scanFeedback}
				<span class="scan-feedback">{scanFeedback}</span>
			{/if}
		</div>
	</section>

	<section class="list-section">
		<h2>{t('inbox.pending_heading')}</h2>
		{#if detectionEvents.loading}
			<p class="muted">{t('common.loading')}</p>
		{:else if pending.length === 0}
			<article class="glass card empty">
				<p class="muted">{t('inbox.empty')}</p>
			</article>
		{:else}
			<ul class="events">
				{#each pending as ev (ev.id)}
					<li class="glass event-row">
						<div class="event-main">
							<div class="event-head">
								<h3>{ev.parsed_payload.service_name ?? '—'}</h3>
								<span class="source-tag">{sourceLabel(ev.source)}</span>
							</div>
							<p class="event-detail">
								{formatMoney(ev.parsed_payload.amount_minor, ev.parsed_payload.currency)}
								{#if ev.parsed_payload.billing_cycle}
									<span class="sep">·</span>
									<span>{t(`cycle.${ev.parsed_payload.billing_cycle}`)}</span>
								{/if}
							</p>
							{#if ev.parsed_payload.payment_method_hint}
								<p class="event-detail muted small">{ev.parsed_payload.payment_method_hint}</p>
							{/if}
							<p class="event-detail muted small">{formatTimestamp(ev.created_at)}</p>
						</div>
						<div class="event-actions">
							<button class="confirm" onclick={() => detectionEvents.confirm(ev.id)}>
								{t('inbox.confirm')}
							</button>
							<button class="reject" onclick={() => detectionEvents.reject(ev.id)}>
								{t('inbox.reject')}
							</button>
						</div>
					</li>
				{/each}
			</ul>
		{/if}

		{#if reviewed.length > 0}
			<h2 class="reviewed-heading">{t('inbox.reviewed_heading')}</h2>
			<ul class="events reviewed">
				{#each reviewed as ev (ev.id)}
					<li class="glass-subtle reviewed-row">
						<span class="rev-name">{ev.parsed_payload.service_name ?? '—'}</span>
						<span class="rev-status status-{ev.status}">{t(`status.${ev.status}`)}</span>
						<span class="rev-time muted small">{formatTimestamp(ev.created_at)}</span>
					</li>
				{/each}
			</ul>
		{/if}

		{#if detectionEvents.error}
			<p class="error">{t('common.error')}: {detectionEvents.error}</p>
		{/if}
	</section>
</div>

<style>
	.container {
		max-width: 880px;
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
	.desc-small {
		margin: 0 0 1rem;
		color: var(--kk-text-muted);
		font-size: 0.9rem;
		line-height: 1.5;
	}
	.section {
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
	}
	.section + .section {
		margin-top: 1rem;
	}
	.section h2 {
		margin: 0 0 0.4rem;
		font-size: 1rem;
		font-weight: 600;
	}
	.source-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}
	.source-card {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		padding: 1rem;
		border-radius: var(--kk-radius-sm);
		background: var(--kk-surface-2);
		border: 1px solid var(--kk-stroke);
	}
	.source-card h3 {
		margin: 0 0 0.3rem;
		font-size: 0.95rem;
		font-weight: 600;
	}
	.source-card button {
		padding: 0.55rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
	}
	.source-card button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.source-card button.secondary {
		background: transparent;
		color: var(--kk-text-muted);
	}
	.connected {
		color: var(--color-accent-matcha);
	}
	.scan-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-top: 1rem;
		flex-wrap: wrap;
	}
	.scan-btn {
		padding: 0.6rem 1.25rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
	}
	.scan-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.scan-feedback {
		font-size: 0.9rem;
		color: var(--kk-text-muted);
	}

	.list-section {
		margin-top: 2rem;
	}
	.list-section h2 {
		font-size: 1.05rem;
		font-weight: 600;
		margin: 0 0 1rem;
	}
	.reviewed-heading {
		margin-top: 1.5rem;
	}
	.events {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.event-row {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 1rem;
		padding: 1rem 1.25rem;
		border-radius: var(--kk-radius-md);
	}
	.event-main {
		min-width: 0;
	}
	.event-head {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}
	.event-head h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}
	.source-tag {
		font-size: 0.7rem;
		padding: 0.15rem 0.5rem;
		border-radius: 999px;
		background: var(--kk-surface-2);
		border: 1px solid var(--kk-stroke);
		color: var(--kk-text-muted);
	}
	.event-detail {
		margin: 0.35rem 0 0;
		font-size: 0.9rem;
	}
	.event-detail.muted {
		color: var(--kk-text-muted);
	}
	.event-detail .sep {
		opacity: 0.5;
		margin: 0 0.35rem;
	}
	.small {
		font-size: 0.8rem;
	}
	.event-actions {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-self: center;
	}
	.event-actions button {
		padding: 0.5rem 0.85rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.85rem;
		font-weight: 600;
	}
	.event-actions .confirm {
		background: var(--color-accent-matcha);
		color: oklch(0.15 0.05 155);
	}
	.event-actions .reject {
		background: transparent;
		color: var(--kk-text-muted);
	}
	.reviewed-row {
		display: grid;
		grid-template-columns: 1fr auto auto;
		gap: 0.85rem;
		align-items: center;
		padding: 0.6rem 1rem;
		border-radius: var(--kk-radius-sm);
	}
	.rev-status {
		font-size: 0.75rem;
		padding: 0.15rem 0.5rem;
		border-radius: 999px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.status-confirmed {
		background: oklch(0.82 0.13 155 / 0.2);
		color: var(--color-accent-matcha);
	}
	.status-rejected {
		background: oklch(0.82 0.13 25 / 0.2);
		color: var(--color-accent-mochi);
	}
	.status-duplicate {
		background: oklch(0.74 0.15 300 / 0.2);
		color: var(--color-accent-fuji);
	}
	.empty {
		padding: 2rem;
		text-align: center;
	}
	.error {
		color: var(--color-accent-mochi);
		margin: 1rem 0 0;
		font-size: 0.9rem;
	}
	@media (max-width: 640px) {
		.source-grid {
			grid-template-columns: 1fr;
		}
		.event-row {
			grid-template-columns: 1fr;
		}
		.event-actions {
			flex-direction: row;
			justify-self: start;
		}
	}
</style>
