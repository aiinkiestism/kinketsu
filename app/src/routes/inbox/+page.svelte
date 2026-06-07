<script lang="ts">
	import { onMount } from 'svelte';
	import { gmail } from '$lib/stores/gmail.svelte';
	import { paypal } from '$lib/stores/paypal.svelte';
	import { llmConfig } from '$lib/stores/llm_config.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import MonthRangeSelect from '$lib/components/MonthRangeSelect.svelte';
	import ErrorDialog from '$lib/components/ErrorDialog.svelte';
	import Tooltip from '$lib/components/Tooltip.svelte';
	import type { ScanOptsDto, YearMonthDto } from '$lib/bindings';

	type ScanErrorInfo = {
		title: string;
		message: string;
		details: string;
		actionUrl?: string;
		actionLabel?: string;
	};

	let errorDialogOpen = $state(false);
	let errorDialog = $state<ScanErrorInfo>({ title: '', message: '', details: '' });

	function openErrorDialog(info: ScanErrorInfo) {
		errorDialog = info;
		errorDialogOpen = true;
	}

	function parseScanError(raw: string): ScanErrorInfo {
		if (raw.includes('Gmail API has not been used') || raw.includes('SERVICE_DISABLED')) {
			const m = raw.match(
				/https:\/\/console\.developers\.google\.com\/apis\/api\/gmail\.googleapis\.com\/overview\?project=\d+/
			);
			return {
				title: t('inbox.error_gmail_disabled_title'),
				message: t('inbox.error_gmail_disabled_msg'),
				details: raw,
				actionUrl: m?.[0] ?? 'https://console.cloud.google.com/apis/library/gmail.googleapis.com',
				actionLabel: t('inbox.error_open_google_console')
			};
		}
		if (raw.includes('access_denied') || raw.includes('Access blocked')) {
			return {
				title: t('inbox.error_oauth_blocked_title'),
				message: t('inbox.error_oauth_blocked_msg'),
				details: raw,
				actionUrl: 'https://console.cloud.google.com/apis/credentials/consent',
				actionLabel: t('inbox.error_open_consent_screen')
			};
		}
		return {
			title: t('common.error'),
			message: raw.length > 240 ? raw.slice(0, 240) + '…' : raw,
			details: raw
		};
	}


	let scanRange = $state<YearMonthDto[]>([]);
	let scanFeedback = $state<string | null>(null);

	// Scan limits & filters, set before scanning.
	let maxFetch = $state(1000);
	let maxLlm = $state(250);
	let usePurchases = $state(false);
	function currentOpts(): ScanOptsDto {
		return { max_fetch: maxFetch, max_llm: maxLlm, use_purchases: usePurchases };
	}

	function scanPreconditions(): boolean {
		scanFeedback = null;
		if (!gmail.credentials) {
			scanFeedback = t('inbox.scan_needs_creds');
			return false;
		}
		if (!gmail.connected) {
			scanFeedback = t('inbox.scan_needs_connection');
			return false;
		}
		if (!llmConfig.current) {
			scanFeedback = t('inbox.scan_needs_llm');
			return false;
		}
		if (scanRange.length === 0) {
			scanFeedback = t('inbox.scan_needs_range');
			return false;
		}
		return true;
	}

	async function handlePreview() {
		if (!scanPreconditions()) return;
		try {
			await gmail.preview(scanRange, currentOpts());
		} catch (e) {
			const raw = gmail.error ?? String(e);
			if (raw.includes('scan cancelled')) {
				scanFeedback = t('inbox.scan_cancelled');
			} else {
				openErrorDialog(parseScanError(raw));
			}
		}
	}

	function fmtUsd(v: number): string {
		try {
			return new Intl.NumberFormat(i18n.bcp47, {
				style: 'currency',
				currency: 'USD',
				maximumFractionDigits: v > 0 && v < 1 ? 4 : 2
			}).format(v);
		} catch {
			return `$${v.toFixed(2)}`;
		}
	}

	async function handleConnectGmail() {
		try {
			await gmail.connect();
		} catch {
			const raw = gmail.error ?? '';
			if (raw && !raw.includes('oauth cancelled')) {
				openErrorDialog(parseScanError(raw));
			}
		}
	}

	async function handleRunScan() {
		if (!scanPreconditions()) return;
		try {
			const created = await gmail.runScan(scanRange, currentOpts());
			const updated = gmail.summary?.updated ?? 0;
			if (created === 0 && updated === 0) {
				scanFeedback = t('inbox.scan_complete_zero');
			} else if (created === 0) {
				scanFeedback = tn('dashboard.last_scan_updated', updated);
			} else {
				scanFeedback =
					tn('inbox.scan_complete', created) +
					(updated > 0 ? ` · ${tn('dashboard.last_scan_updated', updated)}` : '');
			}
		} catch (e) {
			const raw = gmail.error ?? String(e);
			// Short, friendly errors stay inline; everything else gets the modal.
			if (raw.includes('scan cancelled')) {
				scanFeedback = t('inbox.scan_cancelled');
			} else if (raw.includes('no LLM provider configured')) {
				scanFeedback = t('inbox.scan_needs_llm');
			} else if (raw.includes('Gmail not connected')) {
				scanFeedback = t('inbox.scan_needs_connection');
			} else if (raw.includes('Gmail OAuth credentials not configured')) {
				scanFeedback = t('inbox.scan_needs_creds');
			} else {
				openErrorDialog(parseScanError(raw));
			}
		}
	}

	async function handleCancelScan() {
		await gmail.cancelScan();
	}


	async function handleConnectPaypal() {
		try {
			await paypal.connect();
		} catch {
			const raw = paypal.error ?? '';
			if (raw && !raw.includes('oauth cancelled')) {
				openErrorDialog(parseScanError(raw));
			}
		}
	}

	onMount(() => {
		gmail.load();
		paypal.load();
		llmConfig.load();
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
				{:else if gmail.connecting}
					<button type="button" class="secondary" onclick={() => gmail.cancel()}>
						{t('subs.cancel')}
					</button>
				{:else}
					<button
						type="button"
						onclick={handleConnectGmail}
						disabled={!gmail.credentials}
					>
						{t('inbox.connect_gmail')}
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
				{:else if paypal.connecting}
					<button type="button" class="secondary" onclick={() => paypal.cancel()}>
						{t('subs.cancel')}
					</button>
				{:else}
					<button
						type="button"
						onclick={handleConnectPaypal}
						disabled={!paypal.credentials}
					>
						{t('inbox.connect_paypal')}
					</button>
				{/if}
			</article>
		</div>
	</section>

	<section class="glass section">
		<h2>{t('inbox.range_heading')}</h2>
		<p class="muted desc-small">{t('inbox.range_description')}</p>
		<MonthRangeSelect bind:value={scanRange} />

		<div class="opts-grid">
			<label class="opt">
				<span class="opt-label"
					>{t('inbox.opts_max_fetch')}<Tooltip text={t('inbox.opts_max_fetch_hint')} /></span
				>
				<input type="number" min="1" step="50" bind:value={maxFetch} />
			</label>
			<label class="opt">
				<span class="opt-label"
					>{t('inbox.opts_max_llm')}<Tooltip text={t('inbox.opts_max_llm_hint')} /></span
				>
				<input type="number" min="1" step="10" bind:value={maxLlm} />
			</label>
			<label class="opt opt-check">
				<input type="checkbox" bind:checked={usePurchases} />
				<span class="opt-label"
					>{t('inbox.opts_purchases')}<Tooltip text={t('inbox.opts_purchases_hint')} /></span
				>
			</label>
		</div>

		<div class="scan-actions">
			{#if gmail.scanning || gmail.previewing}
				<button type="button" class="scan-btn running" disabled>
					{#if gmail.progress}
						{#if gmail.progress.phase === 'indexing'}
							{t('inbox.scan_progress_indexing', {
								processed: gmail.progress.processed,
								total: gmail.progress.total
							})}
						{:else}
							{t('inbox.scan_progress', {
								processed: gmail.progress.processed,
								total: gmail.progress.total,
								created: gmail.progress.created
							})}
						{/if}
					{:else}
						{gmail.previewing ? t('inbox.previewing') : t('inbox.scan_running')}
					{/if}
				</button>
				<button type="button" class="scan-cancel" onclick={handleCancelScan}>
					{t('subs.cancel')}
				</button>
			{:else}
				<div class="scan-group">
					<button
						type="button"
						class="scan-preview"
						onclick={handlePreview}
						disabled={scanRange.length === 0}
					>
						{t('inbox.preview')}
					</button>
					<button
						type="button"
						class="scan-btn"
						onclick={handleRunScan}
						disabled={scanRange.length === 0}
					>
						{t('inbox.scan_run')}
					</button>
					<Tooltip text={t('inbox.scan_run_hint')} />
				</div>
			{/if}
			{#if scanFeedback}
				<span class="scan-feedback">{scanFeedback}</span>
			{/if}
		</div>

		{#if (gmail.scanning || gmail.previewing) && gmail.progress}
			<p class="scan-detail muted small">
				{t('inbox.scan_progress_detail', {
					classified: gmail.progress.skippedClassified,
					seen: gmail.progress.skippedSeen
				})}
			</p>
		{/if}

		{#if gmail.estimate && !gmail.scanning && !gmail.previewing}
			{@const est = gmail.estimate}
			<div class="estimate glass-subtle">
				<div class="est-head">
					<strong>{t('inbox.preview_matched', { count: est.matched_estimate })}</strong>
				</div>
				<p class="est-targets">{tn('inbox.preview_llm', est.llm_targets)}</p>
				{#if est.notification_hits > 0}
					<p class="muted small">
						{t('inbox.preview_free_notifications', { count: est.notification_hits })}
					</p>
				{/if}
				<p class="muted small">
					{t('inbox.preview_excluded', {
						seen: est.skipped_seen,
						blocked: est.skipped_blocked,
						noamount: est.skipped_no_amount,
						recurrence:
							est.skipped_recurrence > 0
								? t('inbox.preview_recurrence_suffix', { count: est.skipped_recurrence })
								: ''
					})}
				</p>
				{#if est.truncated_by_max_llm}
					<p class="small warn">{t('inbox.preview_truncated')}</p>
				{/if}
				<p class="est-cost">
					{#if est.is_local}
						{t('inbox.preview_cost_local')}
					{:else}
						{t('inbox.preview_cost', {
							low: fmtUsd(est.cost_low_usd),
							high: fmtUsd(est.cost_high_usd)
						})}
					{/if}
				</p>
				<p class="muted small">
					{t('inbox.preview_tokens', { input: est.input_tokens.toLocaleString() })} · {est.provider}/{est.model}
				</p>
				{#if !est.is_local}
					<p class="muted small">{t('inbox.preview_cost_note')}</p>
				{/if}
			</div>
		{/if}
	</section>


	<ErrorDialog
		bind:open={errorDialogOpen}
		title={errorDialog.title}
		message={errorDialog.message}
		details={errorDialog.details}
		actionLabel={errorDialog.actionLabel ?? null}
		actionUrl={errorDialog.actionUrl ?? null}
	/>
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
	.scan-btn.running {
		opacity: 0.8;
	}
	.scan-cancel {
		padding: 0.6rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: transparent;
		color: var(--kk-text-muted);
		cursor: pointer;
		font-family: inherit;
	}
	.scan-cancel:hover {
		color: var(--kk-text-primary);
	}
	.scan-feedback {
		font-size: 0.9rem;
		color: var(--kk-text-muted);
	}
	.scan-detail {
		margin: 0.5rem 0 0;
	}

	.opts-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.85rem 1.25rem;
		margin-top: 1rem;
	}
	.opt {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.85rem;
		color: var(--kk-text-muted);
	}
	.opt-label {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
	}
	.opt input[type='number'] {
		padding: 0.45rem 0.6rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.9rem;
		font-family: inherit;
		width: 8rem;
	}
	.opt-check {
		flex-direction: row;
		align-items: center;
		gap: 0.5rem;
		align-self: end;
	}
	.opt-check input {
		width: 1rem;
		height: 1rem;
		cursor: pointer;
	}
	.scan-group {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding-right: 0.6rem;
	}
	.scan-preview {
		padding: 0.6rem 1rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
	}
	.scan-preview:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.scan-preview:hover:not(:disabled) {
		background: var(--kk-surface-3, var(--kk-surface-2));
		border-color: var(--color-accent-sora);
	}

	.estimate {
		margin-top: 1rem;
		padding: 1rem 1.25rem;
		border-radius: var(--kk-radius-md);
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.est-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}
	.est-head strong {
		font-size: 1.05rem;
	}
	.est-targets {
		margin: 0;
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--color-accent-sora);
	}
	.estimate p {
		margin: 0;
	}
	.est-cost {
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.warn {
		color: var(--color-accent-yuzu);
	}

	.small {
		font-size: 0.8rem;
	}
	@media (max-width: 640px) {
		.source-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
