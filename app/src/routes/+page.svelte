<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { subscriptions } from '$lib/stores/subscriptions.svelte';
	import { paymentMethods } from '$lib/stores/payment_methods.svelte';
	import { categories } from '$lib/stores/categories.svelte';
	import { exchangeRates } from '$lib/stores/exchange_rates.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import { PAYMENT_METHOD_KINDS, type BillingCycle, type PaymentMethodKind } from '$lib/types';

	let exporting = $state(false);

	async function handleExportIcs() {
		exporting = true;
		try {
			const text = await invoke<string>('export_subscriptions_ics');
			const blob = new Blob([text], { type: 'text/calendar' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `kinketsu-${new Date().toISOString().slice(0, 10)}.ics`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			URL.revokeObjectURL(url);
		} catch (e) {
			console.error(e);
		} finally {
			exporting = false;
		}
	}

	// Subscription form
	let showForm = $state(false);
	let formName = $state('');
	let formAmount = $state<number>(0);
	let formCurrency = $state('JPY');
	let formCycle = $state<BillingCycle>('monthly');
	let formPaymentMethodId = $state('');
	let formCategoryId = $state('');

	function resetForm() {
		formName = '';
		formAmount = 0;
		formCurrency = 'JPY';
		formCycle = 'monthly';
		formPaymentMethodId = '';
		formCategoryId = '';
	}

	function toMinor(amount: number, currency: string): number {
		return currency === 'JPY' ? Math.round(amount) : Math.round(amount * 100);
	}

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

	const CYCLE_MONTHS: Record<BillingCycle, number> = {
		weekly: 1 / 4.333,
		monthly: 1,
		quarterly: 3,
		semi_annual: 6,
		annual: 12,
		custom: 1
	};

	function monthlyEquivalentMinor(amount_minor: number, cycle: BillingCycle): number {
		return amount_minor / CYCLE_MONTHS[cycle];
	}

	let monthlyTotalJpy = $derived(
		subscriptions.items
			.filter((s) => s.status === 'active')
			.reduce((sum, s) => {
				const monthlyMinor = monthlyEquivalentMinor(s.amount_minor, s.billing_cycle);
				if (s.currency === 'JPY') return sum + monthlyMinor;
				const converted = exchangeRates.toJpyMinor(monthlyMinor, s.currency);
				return converted !== null ? sum + converted : sum;
			}, 0)
	);

	let unconvertibleCount = $derived(
		subscriptions.items.filter(
			(s) =>
				s.status === 'active' &&
				s.currency !== 'JPY' &&
				exchangeRates.toJpyMinor(s.amount_minor, s.currency) === null
		).length
	);

	let activeCount = $derived(subscriptions.items.filter((s) => s.status === 'active').length);

	let pmById = $derived(new Map(paymentMethods.items.map((p) => [p.id, p])));
	let catById = $derived(new Map(categories.items.map((c) => [c.id, c])));

	async function handleAddSubscription(e: SubmitEvent) {
		e.preventDefault();
		if (!formName.trim()) return;
		try {
			await subscriptions.create({
				name: formName.trim(),
				service_icon: null,
				plan: null,
				amount_minor: toMinor(formAmount, formCurrency),
				currency: formCurrency,
				billing_cycle: formCycle,
				next_billing_date: null,
				started_at: null,
				payment_method_id: formPaymentMethodId || null,
				category_id: formCategoryId || null,
				status: null,
				notes: null
			});
			showForm = false;
			resetForm();
		} catch {
			/* store.error surfaced by banner */
		}
	}

	// Manage section
	let showManage = $state(false);
	let pmName = $state('');
	let pmKind = $state<PaymentMethodKind>('credit_card');
	let catName = $state('');

	async function handleAddPaymentMethod(e: SubmitEvent) {
		e.preventDefault();
		if (!pmName.trim()) return;
		try {
			await paymentMethods.create({
				name: pmName.trim(),
				kind: pmKind,
				last4: null,
				color: null,
				icon: null
			});
			pmName = '';
			pmKind = 'credit_card';
		} catch {
			/* error surfaced via store */
		}
	}

	async function handleAddCategory(e: SubmitEvent) {
		e.preventDefault();
		if (!catName.trim()) return;
		try {
			await categories.create({
				name: catName.trim(),
				icon: null,
				color: null
			});
			catName = '';
		} catch {
			/* error surfaced via store */
		}
	}

	onMount(() => {
		subscriptions.load();
		paymentMethods.load();
		categories.load();
		exchangeRates.load();
	});
</script>

<div class="container">
	<header class="title">
		<h1>kinketsu</h1>
		<p class="tagline">{t('tagline')}</p>
	</header>

	<section class="grid">
		<article class="glass card">
			<h2>{t('dashboard.monthly_total')}</h2>
			<p class="big">{formatMoney(monthlyTotalJpy, 'JPY')}</p>
			<p class="muted">{tn('dashboard.active', activeCount)}</p>
		</article>
		<article class="glass card">
			<h2>{t('dashboard.upcoming')}</h2>
			<p class="muted">{t('dashboard.upcoming_empty')}</p>
		</article>
	</section>

	<section class="list-section">
		<div class="list-header">
			<h2>{t('subs.heading')}</h2>
			<div class="header-actions">
				<button
					class="glass-subtle pill-btn"
					onclick={handleExportIcs}
					disabled={exporting || subscriptions.items.length === 0}
				>
					{exporting ? t('common.loading') : t('dashboard.export_ics')}
				</button>
				<button class="glass-subtle pill-btn" onclick={() => (showForm = !showForm)}>
					{showForm ? t('subs.cancel') : t('subs.add')}
				</button>
			</div>
		</div>

		{#if showForm}
			<form class="glass form" onsubmit={handleAddSubscription}>
				<label>
					<span>{t('form.name')}</span>
					<input bind:value={formName} placeholder={t('form.name_placeholder')} required />
				</label>
				<label>
					<span>{t('form.amount')}</span>
					<input type="number" bind:value={formAmount} min="0" step="1" required />
				</label>
				<label>
					<span>{t('form.currency')}</span>
					<select bind:value={formCurrency}>
						<option value="JPY">JPY</option>
						<option value="USD">USD</option>
						<option value="EUR">EUR</option>
						<option value="GBP">GBP</option>
					</select>
				</label>
				<label>
					<span>{t('form.billing_cycle')}</span>
					<select bind:value={formCycle}>
						<option value="weekly">{t('cycle.weekly')}</option>
						<option value="monthly">{t('cycle.monthly')}</option>
						<option value="quarterly">{t('cycle.quarterly')}</option>
						<option value="semi_annual">{t('cycle.semi_annual')}</option>
						<option value="annual">{t('cycle.annual')}</option>
					</select>
				</label>
				<label>
					<span>{t('subs.payment_method')}</span>
					<select bind:value={formPaymentMethodId}>
						<option value="">{t('subs.none')}</option>
						{#each paymentMethods.items as pm (pm.id)}
							<option value={pm.id}>{pm.name}</option>
						{/each}
					</select>
				</label>
				<label>
					<span>{t('subs.category')}</span>
					<select bind:value={formCategoryId}>
						<option value="">{t('subs.none')}</option>
						{#each categories.items as cat (cat.id)}
							<option value={cat.id}>{cat.name}</option>
						{/each}
					</select>
				</label>
				<button type="submit" class="submit">{t('form.submit')}</button>
			</form>
		{/if}

		{#if subscriptions.error}
			<p class="error">{t('common.error')}: {subscriptions.error}</p>
		{/if}

		{#if subscriptions.loading}
			<p class="muted">{t('common.loading')}</p>
		{:else if subscriptions.items.length === 0}
			<article class="glass card empty">
				<p class="muted">{t('subs.empty')}</p>
			</article>
		{:else}
			<ul class="sub-list">
				{#each subscriptions.items as sub (sub.id)}
					{@const pm = sub.payment_method_id ? pmById.get(sub.payment_method_id) : undefined}
					{@const cat = sub.category_id ? catById.get(sub.category_id) : undefined}
					<li class="glass sub-row">
						<div class="sub-main">
							<span class="sub-name">{sub.name}</span>
							{#if pm || cat || sub.plan}
								<span class="sub-meta">
									{#if pm}<span>{pm.name}</span>{/if}
									{#if pm && (cat || sub.plan)}<span class="sep">·</span>{/if}
									{#if cat}<span>{cat.name}</span>{/if}
									{#if cat && sub.plan}<span class="sep">·</span>{/if}
									{#if sub.plan}<span>{sub.plan}</span>{/if}
								</span>
							{/if}
						</div>
						<span class="sub-cycle">{t(`cycle.${sub.billing_cycle}`)}</span>
						<span class="sub-amount">{formatMoney(sub.amount_minor, sub.currency)}</span>
						<button
							class="del"
							onclick={() => subscriptions.remove(sub.id)}
							aria-label={t('common.delete')}
						>×</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section class="manage-section">
		<button class="glass-subtle pill-btn manage-toggle" onclick={() => (showManage = !showManage)}>
			{t('manage.heading')} <span class="caret">{showManage ? '▾' : '▸'}</span>
		</button>

		{#if showManage}
			<div class="manage-grid">
				<article class="glass manage-block">
					<h3>{t('manage.payment_methods')}</h3>
					{#if paymentMethods.items.length === 0}
						<p class="muted small">{t('manage.empty_pm')}</p>
					{:else}
						<ul class="manage-list">
							{#each paymentMethods.items as pm (pm.id)}
								<li>
									<span class="m-name">{pm.name}</span>
									<span class="muted small">{t(`kind.${pm.kind}`)}</span>
									<button
										class="del-small"
										onclick={() => paymentMethods.remove(pm.id)}
										aria-label={t('common.delete')}
									>×</button>
								</li>
							{/each}
						</ul>
					{/if}
					<form class="manage-add" onsubmit={handleAddPaymentMethod}>
						<input bind:value={pmName} placeholder={t('form.name')} required />
						<select bind:value={pmKind}>
							{#each PAYMENT_METHOD_KINDS as k (k)}
								<option value={k}>{t(`kind.${k}`)}</option>
							{/each}
						</select>
						<button type="submit" class="add-mini">+</button>
					</form>
				</article>

				<article class="glass manage-block">
					<h3>{t('manage.categories')}</h3>
					{#if categories.items.length === 0}
						<p class="muted small">{t('manage.empty_cat')}</p>
					{:else}
						<ul class="manage-list">
							{#each categories.items as cat (cat.id)}
								<li>
									<span class="m-name">{cat.name}</span>
									<button
										class="del-small"
										onclick={() => categories.remove(cat.id)}
										aria-label={t('common.delete')}
									>×</button>
								</li>
							{/each}
						</ul>
					{/if}
					<form class="manage-add" onsubmit={handleAddCategory}>
						<input bind:value={catName} placeholder={t('form.name')} required />
						<button type="submit" class="add-mini">+</button>
					</form>
				</article>
			</div>
		{/if}
	</section>
</div>

<style>
	.container {
		max-width: 1024px;
		margin: 0 auto;
		padding: 4rem 2rem 6rem;
	}
	.title h1 {
		font-size: clamp(2.5rem, 4vw, 4rem);
		font-weight: 800;
		margin: 0;
		letter-spacing: -0.03em;
		font-family:
			ui-sans-serif,
			system-ui,
			-apple-system,
			'Inter',
			sans-serif;
	}
	.title .tagline {
		margin: 0.5rem 0 3rem;
		color: var(--kk-text-muted);
		font-size: 1.1rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		margin-bottom: 2.5rem;
	}
	.card {
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
	}
	.card h2 {
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--kk-text-muted);
		font-weight: 500;
	}
	.card .big {
		margin: 0;
		font-size: 2rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		font-variant-numeric: tabular-nums;
	}
	.card .muted {
		margin: 0.25rem 0 0;
		color: var(--kk-text-muted);
		font-size: 0.9rem;
	}
	.list-section h2,
	.manage-section h3 {
		font-size: 1.05rem;
		font-weight: 600;
		margin: 0;
	}
	.list-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}
	.pill-btn {
		border: none;
		padding: 0.5rem 1rem;
		border-radius: var(--kk-radius-sm);
		color: var(--kk-text-primary);
		cursor: pointer;
		font-size: 0.9rem;
		font-family: inherit;
	}
	.pill-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.header-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.form {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
		margin-bottom: 1rem;
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
		padding: 0.6rem 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.95rem;
		font-family: inherit;
	}
	.form .submit {
		grid-column: span 2;
		padding: 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-weight: 600;
		cursor: pointer;
	}
	.sub-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.sub-row {
		display: grid;
		grid-template-columns: 1fr auto auto auto;
		gap: 1rem;
		align-items: center;
		padding: 0.85rem 1.25rem;
		border-radius: var(--kk-radius-sm);
	}
	.sub-main {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.sub-name {
		font-weight: 600;
	}
	.sub-meta {
		display: inline-flex;
		gap: 0.4rem;
		flex-wrap: wrap;
		font-size: 0.8rem;
		color: var(--kk-text-muted);
	}
	.sub-meta .sep {
		opacity: 0.5;
	}
	.sub-cycle {
		font-size: 0.8rem;
		color: var(--kk-text-muted);
	}
	.sub-amount {
		font-variant-numeric: tabular-nums;
		font-weight: 600;
	}
	.del {
		border: none;
		background: transparent;
		color: var(--kk-text-muted);
		cursor: pointer;
		font-size: 1.5rem;
		line-height: 1;
		padding: 0;
		width: 1.5rem;
		height: 1.5rem;
	}
	.del:hover {
		color: var(--color-accent-mochi);
	}
	.empty {
		padding: 2rem;
		text-align: center;
	}
	.error {
		color: var(--color-accent-mochi);
		margin: 0 0 1rem;
		font-size: 0.9rem;
	}

	/* Manage section */
	.manage-section {
		margin-top: 2.5rem;
	}
	.manage-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}
	.manage-toggle .caret {
		font-size: 0.75rem;
		opacity: 0.7;
	}
	.manage-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		margin-top: 1rem;
	}
	.manage-block {
		padding: 1.25rem;
		border-radius: var(--kk-radius-md);
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.manage-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.manage-list li {
		display: grid;
		grid-template-columns: 1fr auto auto;
		gap: 0.75rem;
		align-items: center;
		padding: 0.45rem 0.65rem;
		background: var(--kk-surface-2);
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
	}
	.m-name {
		font-size: 0.95rem;
	}
	.small {
		font-size: 0.8rem;
	}
	.del-small {
		border: none;
		background: transparent;
		color: var(--kk-text-muted);
		cursor: pointer;
		font-size: 1.1rem;
		line-height: 1;
		padding: 0;
		width: 1.25rem;
		height: 1.25rem;
	}
	.del-small:hover {
		color: var(--color-accent-mochi);
	}
	.manage-add {
		display: grid;
		grid-template-columns: 1fr auto auto;
		gap: 0.5rem;
	}
	.manage-add input,
	.manage-add select {
		padding: 0.5rem 0.65rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.9rem;
		font-family: inherit;
	}
	.add-mini {
		border: 1px solid var(--kk-stroke);
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		border-radius: var(--kk-radius-sm);
		font-weight: 700;
		cursor: pointer;
		width: 2rem;
	}

	@media (max-width: 640px) {
		.grid,
		.manage-grid {
			grid-template-columns: 1fr;
		}
		.form {
			grid-template-columns: 1fr;
		}
		.form .submit {
			grid-column: span 1;
		}
		.sub-row {
			grid-template-columns: 1fr auto auto;
		}
		.sub-cycle {
			display: none;
		}
		.manage-add {
			grid-template-columns: 1fr auto;
		}
		.manage-add select {
			grid-column: span 2;
		}
	}
</style>
