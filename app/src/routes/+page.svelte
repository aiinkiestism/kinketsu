<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { subscriptions } from '$lib/stores/subscriptions.svelte';
	import { paymentMethods } from '$lib/stores/payment_methods.svelte';
	import { categories } from '$lib/stores/categories.svelte';
	import { exchangeRates } from '$lib/stores/exchange_rates.svelte';
	import { detectionEvents } from '$lib/stores/detection_events.svelte';
	import { gmail } from '$lib/stores/gmail.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import { CURRENCIES, PAYMENT_METHOD_KINDS, SUBSCRIPTION_STATUSES } from '$lib/constants';
	import type {
		BillingCycle,
		Category,
		PaymentMethod,
		PaymentMethodKind,
		Subscription,
		SubscriptionStatus
	} from '$lib/bindings';

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

	// Subscription form (used for both create and edit)
	let showForm = $state(false);
	let editingSub = $state<Subscription | null>(null);
	let formName = $state('');
	let formAmount = $state<number>(0);
	let formCurrency = $state(exchangeRates.base || 'JPY');
	let formCycle = $state<BillingCycle>('monthly');
	let formStatus = $state<SubscriptionStatus>('active');
	let formPlan = $state('');
	let formNextBilling = $state('');
	let formStartedAt = $state('');
	let formNotes = $state('');
	let formPaymentMethodId = $state('');
	let formCategoryId = $state('');

	function resetForm() {
		editingSub = null;
		formName = '';
		formAmount = 0;
		formCurrency = exchangeRates.base || 'JPY';
		formCycle = 'monthly';
		formStatus = 'active';
		formPlan = '';
		formNextBilling = '';
		formStartedAt = '';
		formNotes = '';
		formPaymentMethodId = '';
		formCategoryId = '';
	}

	function openCreate() {
		resetForm();
		showForm = true;
	}

	function openEdit(sub: Subscription) {
		editingSub = sub;
		formName = sub.name;
		formAmount = fromMinor(sub.amount_minor, sub.currency);
		formCurrency = sub.currency;
		formCycle = sub.billing_cycle;
		formStatus = sub.status;
		formPlan = sub.plan ?? '';
		formNextBilling = sub.next_billing_date ?? '';
		formStartedAt = sub.started_at ?? '';
		formNotes = sub.notes ?? '';
		formPaymentMethodId = sub.payment_method_id ?? '';
		formCategoryId = sub.category_id ?? '';
		showForm = true;
		// Scroll the form into view
		setTimeout(() => {
			document.querySelector('.form')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
		}, 0);
	}

	function toMinor(amount: number, currency: string): number {
		return currency === 'JPY' ? Math.round(amount) : Math.round(amount * 100);
	}

	function fromMinor(amount_minor: number, currency: string): number {
		return currency === 'JPY' ? amount_minor : amount_minor / 100;
	}

	function baseFractionDigits(): number {
		return exchangeRates.base === 'JPY' ? 0 : 2;
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

	let monthlyTotalBase = $derived(
		subscriptions.items
			.filter((s) => s.status === 'active')
			.reduce((sum, s) => {
				const monthlyMinor = monthlyEquivalentMinor(s.amount_minor, s.billing_cycle);
				const converted = exchangeRates.toBaseMinor(monthlyMinor, s.currency);
				return converted !== null ? sum + converted : sum;
			}, 0)
	);

	let unconvertibleCount = $derived(
		subscriptions.items.filter(
			(s) =>
				s.status === 'active' &&
				s.currency !== exchangeRates.base &&
				exchangeRates.toBaseMinor(s.amount_minor, s.currency) === null
		).length
	);

	let activeCount = $derived(subscriptions.items.filter((s) => s.status === 'active').length);

	let pmById = $derived(new Map(paymentMethods.items.map((p) => [p.id, p])));
	let catById = $derived(new Map(categories.items.map((c) => [c.id, c])));

	async function handleSubmitSubscription(e: SubmitEvent) {
		e.preventDefault();
		if (!formName.trim()) return;
		const amount_minor = toMinor(formAmount, formCurrency);
		const plan = formPlan.trim() || null;
		const notes = formNotes.trim() || null;
		const next_billing_date = formNextBilling.trim() || null;
		const started_at = formStartedAt.trim() || null;
		const payment_method_id = formPaymentMethodId || null;
		const category_id = formCategoryId || null;
		try {
			if (editingSub) {
				await subscriptions.update({
					...editingSub,
					name: formName.trim(),
					plan,
					amount_minor,
					currency: formCurrency,
					billing_cycle: formCycle,
					status: formStatus,
					next_billing_date,
					started_at,
					payment_method_id,
					category_id,
					notes
				});
			} else {
				await subscriptions.create({
					name: formName.trim(),
					service_icon: null,
					plan,
					amount_minor,
					currency: formCurrency,
					billing_cycle: formCycle,
					next_billing_date,
					started_at,
					payment_method_id,
					category_id,
					status: formStatus,
					notes
				});
			}
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

	let editingPmId = $state<string | null>(null);
	let editPmName = $state('');
	let editPmKind = $state<PaymentMethodKind>('credit_card');

	function startEditPm(pm: PaymentMethod) {
		editingPmId = pm.id;
		editPmName = pm.name;
		editPmKind = pm.kind;
	}

	function cancelEditPm() {
		editingPmId = null;
	}

	async function saveEditPm(pm: PaymentMethod) {
		if (!editPmName.trim()) return;
		try {
			await paymentMethods.update({
				...pm,
				name: editPmName.trim(),
				kind: editPmKind
			});
			editingPmId = null;
		} catch {
			/* error in store */
		}
	}

	let editingCatId = $state<string | null>(null);
	let editCatName = $state('');

	function startEditCat(cat: Category) {
		editingCatId = cat.id;
		editCatName = cat.name;
	}

	function cancelEditCat() {
		editingCatId = null;
	}

	async function saveEditCat(cat: Category) {
		if (!editCatName.trim()) return;
		try {
			await categories.update({
				...cat,
				name: editCatName.trim()
			});
			editingCatId = null;
		} catch {
			/* error in store */
		}
	}

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

	let pendingReview = $derived(detectionEvents.items.filter((e) => e.status === 'pending'));

	function detMoney(amount_minor: number | null, currency: string | null): string {
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

	onMount(() => {
		subscriptions.load();
		paymentMethods.load();
		categories.load();
		detectionEvents.load();
		gmail.loadSummary();
		exchangeRates.init(i18n.locale).then(() => {
			if (!editingSub) {
				formCurrency = exchangeRates.base;
			}
		});
	});
</script>

<div class="container">
	<header class="title">
		<h1>kinketsu</h1>
		<p class="tagline">{t('tagline')}</p>
	</header>

	<section class="grid">
		<article class="glass card">
			<h2>{t('dashboard.monthly_total', { currency: exchangeRates.base })}</h2>
			<p class="big">{formatMoney(monthlyTotalBase, exchangeRates.base)}</p>
			<p class="muted">{tn('dashboard.active', activeCount)}</p>
			{#if unconvertibleCount > 0}
				<p class="muted small warn">{tn('dashboard.unconvertible', unconvertibleCount)}</p>
			{/if}
		</article>
		<article class="glass card">
			<h2>{t('dashboard.last_scan_heading')}</h2>
			{#if gmail.summary}
				<p class="big">{gmail.summary.created}</p>
				<p class="muted">
					{t('dashboard.last_scan_summary', {
						matched: gmail.summary.matched_estimate,
						llm: gmail.summary.llm_calls,
						created: gmail.summary.created
					})}
				</p>
				<p class="muted small">
					{gmail.summary.mode === 'deep'
						? t('dashboard.last_scan_mode_deep')
						: t('dashboard.last_scan_mode_fast')} · {formatTimestamp(gmail.summary.ran_at)}
				</p>
			{:else}
				<p class="muted">{t('dashboard.last_scan_never')}</p>
			{/if}
		</article>
	</section>

	{#if pendingReview.length > 0}
		<section class="review-section">
			<div class="list-header">
				<h2>
					{t('dashboard.review_heading')}
					<span class="review-badge">{pendingReview.length}</span>
				</h2>
				<a class="glass-subtle pill-btn" href="/inbox">{t('dashboard.review_view_all')}</a>
			</div>
			<ul class="review-list">
				{#each pendingReview.slice(0, 5) as ev (ev.id)}
					<li class="glass review-row">
						<div class="review-main">
							<span class="review-name">
								{ev.parsed_payload.service_name ?? '—'}
								{#if ev.parsed_payload.recurring && ev.parsed_payload.months_seen}
									<span class="recur-pill">{tn('inbox.recurring_pill', ev.parsed_payload.months_seen)}</span>
								{/if}
							</span>
							<span class="review-amt">
								{detMoney(ev.parsed_payload.amount_minor, ev.parsed_payload.currency)}
								{#if ev.parsed_payload.billing_cycle}
									<span class="sep">·</span>{t(`cycle.${ev.parsed_payload.billing_cycle}`)}
								{/if}
							</span>
						</div>
						<div class="review-actions">
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
		</section>
	{/if}

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
				<button
					class="glass-subtle pill-btn"
					onclick={() => {
						if (showForm) {
							showForm = false;
							resetForm();
						} else {
							openCreate();
						}
					}}
				>
					{showForm ? t('subs.cancel') : t('subs.add')}
				</button>
			</div>
		</div>

		{#if showForm}
			<form class="glass form" onsubmit={handleSubmitSubscription}>
				{#if editingSub}
					<p class="form-heading">{t('form.edit_heading')}</p>
				{/if}
				<label>
					<span>{t('form.name')}</span>
					<input bind:value={formName} placeholder={t('form.name_placeholder')} required />
				</label>
				<label>
					<span>{t('form.amount')}</span>
					<input
						type="number"
						bind:value={formAmount}
						min="0"
						step={formCurrency === 'JPY' ? '1' : '0.01'}
						required
					/>
				</label>
				<label>
					<span>{t('form.currency')}</span>
					<select bind:value={formCurrency}>
						{#each CURRENCIES as cur (cur)}
							<option value={cur}>{cur}</option>
						{/each}
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
						<option value="custom">{t('cycle.custom')}</option>
					</select>
				</label>
				<label>
					<span>{t('form.status')}</span>
					<select bind:value={formStatus}>
						{#each SUBSCRIPTION_STATUSES as s (s)}
							<option value={s}>{t(`sub_status.${s}`)}</option>
						{/each}
					</select>
				</label>
				<label>
					<span>{t('form.plan')}</span>
					<input bind:value={formPlan} placeholder="e.g. Premium" />
				</label>
				<label>
					<span>{t('form.next_billing_date')}</span>
					<input type="date" bind:value={formNextBilling} />
				</label>
				<label>
					<span>{t('form.started_at')}</span>
					<input type="date" bind:value={formStartedAt} />
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
				<label class="span-2">
					<span>{t('form.notes')}</span>
					<textarea bind:value={formNotes} rows="2"></textarea>
				</label>
				<button type="submit" class="submit">
					{editingSub ? t('form.update') : t('form.submit')}
				</button>
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
							class="edit"
							onclick={() => openEdit(sub)}
							aria-label={t('common.edit')}
							title={t('common.edit')}
						>✎</button>
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
								{#if editingPmId === pm.id}
									<li class="manage-edit">
										<input bind:value={editPmName} placeholder={t('form.name')} />
										<select bind:value={editPmKind}>
											{#each PAYMENT_METHOD_KINDS as k (k)}
												<option value={k}>{t(`kind.${k}`)}</option>
											{/each}
										</select>
										<button class="ok-small" onclick={() => saveEditPm(pm)} aria-label={t('common.edit')}>✓</button>
										<button class="del-small" onclick={cancelEditPm} aria-label={t('subs.cancel')}>×</button>
									</li>
								{:else}
									<li>
										<span class="m-name">{pm.name}</span>
										<span class="muted small">{t(`kind.${pm.kind}`)}</span>
										<button
											class="edit-small"
											onclick={() => startEditPm(pm)}
											aria-label={t('common.edit')}
										>✎</button>
										<button
											class="del-small"
											onclick={() => paymentMethods.remove(pm.id)}
											aria-label={t('common.delete')}
										>×</button>
									</li>
								{/if}
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
								{#if editingCatId === cat.id}
									<li class="manage-edit cat-edit">
										<input bind:value={editCatName} placeholder={t('form.name')} />
										<button class="ok-small" onclick={() => saveEditCat(cat)} aria-label={t('common.edit')}>✓</button>
										<button class="del-small" onclick={cancelEditCat} aria-label={t('subs.cancel')}>×</button>
									</li>
								{:else}
								<li>
									<span class="m-name">{cat.name}</span>
									<button
										class="edit-small"
										onclick={() => startEditCat(cat)}
										aria-label={t('common.edit')}
									>✎</button>
									<button
										class="del-small"
										onclick={() => categories.remove(cat.id)}
										aria-label={t('common.delete')}
									>×</button>
								</li>
								{/if}
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
		grid-template-columns: 1fr auto auto auto auto;
		gap: 0.85rem;
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
	.del,
	.edit {
		border: none;
		background: transparent;
		color: var(--kk-text-muted);
		cursor: pointer;
		font-size: 1.1rem;
		line-height: 1;
		padding: 0;
		width: 1.5rem;
		height: 1.5rem;
	}
	.del {
		font-size: 1.5rem;
	}
	.del:hover {
		color: var(--color-accent-mochi);
	}
	.edit:hover {
		color: var(--color-accent-sora);
	}
	.warn {
		color: var(--color-accent-yuzu);
	}

	/* Needs-review section */
	.review-section {
		margin-bottom: 2.5rem;
	}
	.review-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.4rem;
		height: 1.4rem;
		padding: 0 0.4rem;
		margin-left: 0.4rem;
		border-radius: 999px;
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		font-size: 0.8rem;
		font-weight: 700;
		vertical-align: middle;
	}
	.review-list {
		list-style: none;
		padding: 0;
		margin: 1rem 0 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.review-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.85rem 1.25rem;
		border-radius: var(--kk-radius-sm);
	}
	.review-main {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.review-name {
		font-weight: 600;
	}
	.recur-pill {
		font-size: 0.7rem;
		padding: 0.1rem 0.45rem;
		margin-left: 0.4rem;
		border-radius: 999px;
		background: oklch(0.82 0.13 155 / 0.18);
		color: var(--color-accent-matcha);
		font-weight: 600;
		white-space: nowrap;
	}
	.review-amt {
		font-size: 0.85rem;
		color: var(--kk-text-muted);
		font-variant-numeric: tabular-nums;
	}
	.review-amt .sep {
		opacity: 0.5;
		margin: 0 0.35rem;
	}
	.review-actions {
		display: flex;
		gap: 0.4rem;
		flex-shrink: 0;
	}
	.review-actions button {
		padding: 0.5rem 0.85rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.85rem;
		font-weight: 600;
	}
	.review-actions .confirm {
		background: var(--color-accent-matcha);
		color: oklch(0.15 0.05 155);
	}
	.review-actions .reject {
		background: transparent;
		color: var(--kk-text-muted);
	}

	.form-heading {
		grid-column: span 2;
		margin: 0;
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--kk-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.form textarea {
		padding: 0.6rem 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.95rem;
		font-family: inherit;
		resize: vertical;
	}
	.form .span-2 {
		grid-column: span 2;
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
	.del-small,
	.edit-small,
	.ok-small {
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
	.edit-small:hover {
		color: var(--color-accent-sora);
	}
	.ok-small {
		color: var(--color-accent-matcha);
	}
	.manage-edit {
		display: grid !important;
		grid-template-columns: 1fr 1fr auto auto;
		gap: 0.4rem;
		align-items: center;
	}
	.manage-edit.cat-edit {
		grid-template-columns: 1fr auto auto;
	}
	.manage-edit input,
	.manage-edit select {
		padding: 0.4rem 0.55rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.85rem;
		font-family: inherit;
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
			grid-template-columns: 1fr auto auto auto;
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
