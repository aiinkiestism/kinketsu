<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { detectionEvents } from '$lib/stores/detection_events.svelte';
	import { i18n, t, tn } from '$lib/i18n.svelte';
	import { CURRENCIES } from '$lib/constants';
	import type { BillingCycle, DetectionEvent } from '$lib/bindings';

	type SortKey =
		| 'service'
		| 'amount'
		| 'cycle'
		| 'recurring'
		| 'source'
		| 'status'
		| 'last_charged'
		| 'detected';
	type StatusFilter = 'all' | 'pending' | 'confirmed' | 'rejected';
	type SourceFilter = 'all' | 'merchant_receipt' | 'processor_notification' | 'card_notification';

	let statusFilter = $state<StatusFilter>('pending');
	let sourceFilter = $state<SourceFilter>('all');
	let query = $state('');
	let sortKey = $state<SortKey>('recurring');
	let sortDir = $state<'asc' | 'desc'>('desc');

	function toggleSort(key: SortKey) {
		if (sortKey === key) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDir = key === 'service' ? 'asc' : 'desc';
		}
	}

	function fromMinor(amount_minor: number, currency: string): number {
		return currency === 'JPY' ? amount_minor : amount_minor / 100;
	}
	function toMinor(amount: number, currency: string): number {
		return currency === 'JPY' ? Math.round(amount) : Math.round(amount * 100);
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
	function formatTimestamp(iso: string): string {
		try {
			return new Intl.DateTimeFormat(i18n.bcp47, { dateStyle: 'medium' }).format(new Date(iso));
		} catch {
			return iso;
		}
	}

	function cmp(a: DetectionEvent, b: DetectionEvent): number {
		const dir = sortDir === 'asc' ? 1 : -1;
		const pa = a.parsed_payload;
		const pb = b.parsed_payload;
		let x: number;
		switch (sortKey) {
			case 'service':
				x = (pa.service_name ?? '').localeCompare(pb.service_name ?? '');
				break;
			case 'amount':
				x = (pa.amount_minor ?? -1) - (pb.amount_minor ?? -1);
				break;
			case 'cycle':
				x = (pa.billing_cycle ?? '').localeCompare(pb.billing_cycle ?? '');
				break;
			case 'recurring':
				x = (pa.months_seen ?? 0) - (pb.months_seen ?? 0);
				break;
			case 'source':
				x = (pa.source_kind ?? '').localeCompare(pb.source_kind ?? '');
				break;
			case 'status':
				x = a.status.localeCompare(b.status);
				break;
			case 'last_charged':
				x = (pa.last_charged_at ?? '').localeCompare(pb.last_charged_at ?? '');
				break;
			case 'detected':
				x = a.created_at.localeCompare(b.created_at);
				break;
		}
		return x * dir;
	}

	let rows = $derived(
		detectionEvents.items
			.filter((e) => statusFilter === 'all' || e.status === statusFilter)
			.filter((e) => sourceFilter === 'all' || e.parsed_payload.source_kind === sourceFilter)
			.filter((e) => {
				const q = query.trim().toLowerCase();
				if (!q) return true;
				return (e.parsed_payload.service_name ?? '').toLowerCase().includes(q);
			})
			.slice()
			.sort(cmp)
	);

	// Bulk select (pending only)
	let selectedIds = $state(new SvelteSet<string>());
	let pendingRows = $derived(rows.filter((r) => r.status === 'pending'));
	let allSelected = $derived(
		pendingRows.length > 0 && pendingRows.every((r) => selectedIds.has(r.id))
	);
	function toggleSelectAll() {
		if (allSelected) selectedIds.clear();
		else for (const r of pendingRows) selectedIds.add(r.id);
	}
	async function handleBulkReject() {
		const ids = Array.from(selectedIds);
		if (ids.length === 0) return;
		await detectionEvents.bulkReject(ids);
		selectedIds.clear();
	}

	// Inline edit-and-confirm
	let editingId = $state<string | null>(null);
	let editName = $state('');
	let editAmount = $state(0);
	let editCurrency = $state('JPY');
	let editCycle = $state<BillingCycle>('monthly');
	let editNextBilling = $state('');

	function startEdit(ev: DetectionEvent) {
		editingId = ev.id;
		editName = ev.parsed_payload.service_name ?? '';
		editCurrency = ev.parsed_payload.currency ?? 'JPY';
		editAmount =
			ev.parsed_payload.amount_minor !== null
				? fromMinor(ev.parsed_payload.amount_minor, editCurrency)
				: 0;
		editCycle = ev.parsed_payload.billing_cycle ?? 'monthly';
		editNextBilling = '';
	}
	async function saveEdit(ev: DetectionEvent) {
		if (!editName.trim()) return;
		await detectionEvents.confirmWithOverrides(ev.id, {
			name: editName.trim(),
			service_icon: null,
			plan: null,
			amount_minor: toMinor(editAmount, editCurrency),
			currency: editCurrency,
			billing_cycle: editCycle,
			next_billing_date: editNextBilling || null,
			started_at: ev.parsed_payload.charged_at,
			payment_method_id: null,
			category_id: null,
			status: null,
			notes: null
		});
		editingId = null;
	}

	function sortArrow(key: SortKey): string {
		if (sortKey !== key) return '';
		return sortDir === 'asc' ? ' ▲' : ' ▼';
	}

	onMount(() => detectionEvents.load());
</script>

<div class="container">
	<header class="title">
		<h1>{t('review.heading')}</h1>
		<p class="muted desc">{t('review.description')}</p>
	</header>

	<div class="toolbar glass-subtle">
		<input class="search" type="search" bind:value={query} placeholder={t('review.search_placeholder')} />
		<label class="filter">
			<span>{t('review.filter_status')}</span>
			<select bind:value={statusFilter}>
				<option value="all">{t('review.all')}</option>
				<option value="pending">{t('status.pending')}</option>
				<option value="confirmed">{t('status.confirmed')}</option>
				<option value="rejected">{t('status.rejected')}</option>
			</select>
		</label>
		<label class="filter">
			<span>{t('review.filter_source')}</span>
			<select bind:value={sourceFilter}>
				<option value="all">{t('review.all')}</option>
				<option value="merchant_receipt">{t('source_kind.merchant_receipt')}</option>
				<option value="processor_notification">{t('source_kind.processor_notification')}</option>
				<option value="card_notification">{t('source_kind.card_notification')}</option>
			</select>
		</label>
		<span class="count muted small">{tn('review.count', rows.length)}</span>
		{#if selectedIds.size > 0}
			<button class="bulk-reject" onclick={handleBulkReject}>
				{t('inbox.bulk_reject')} ({selectedIds.size})
			</button>
		{/if}
	</div>

	{#if detectionEvents.loading}
		<p class="muted">{t('common.loading')}</p>
	{:else if rows.length === 0}
		<article class="glass card empty"><p class="muted">{t('review.empty')}</p></article>
	{:else}
		<div class="table-wrap glass">
			<table>
				<thead>
					<tr>
						<th class="sel">
							<input
								type="checkbox"
								checked={allSelected}
								indeterminate={selectedIds.size > 0 && !allSelected}
								onchange={toggleSelectAll}
								disabled={pendingRows.length === 0}
								aria-label={t('inbox.bulk_select_all')}
							/>
						</th>
						<th class="sortable" onclick={() => toggleSort('service')}>{t('review.col_service')}{sortArrow('service')}</th>
						<th class="sortable num" onclick={() => toggleSort('amount')}>{t('review.col_amount')}{sortArrow('amount')}</th>
						<th class="sortable" onclick={() => toggleSort('cycle')}>{t('review.col_cycle')}{sortArrow('cycle')}</th>
						<th class="sortable num" onclick={() => toggleSort('recurring')}>{t('review.col_recurring')}{sortArrow('recurring')}</th>
						<th class="sortable" onclick={() => toggleSort('source')}>{t('review.col_source')}{sortArrow('source')}</th>
						<th class="sortable" onclick={() => toggleSort('status')}>{t('review.col_status')}{sortArrow('status')}</th>
						<th class="sortable" onclick={() => toggleSort('last_charged')}>{t('review.col_last_charged')}{sortArrow('last_charged')}</th>
						<th class="sortable" onclick={() => toggleSort('detected')}>{t('review.col_detected')}{sortArrow('detected')}</th>
						<th class="actions-col"></th>
					</tr>
				</thead>
				<tbody>
					{#each rows as ev (ev.id)}
						<tr class:editing={editingId === ev.id}>
							<td class="sel">
								{#if ev.status === 'pending'}
									<input
										type="checkbox"
										checked={selectedIds.has(ev.id)}
										onchange={(e) =>
											(e.currentTarget as HTMLInputElement).checked
												? selectedIds.add(ev.id)
												: selectedIds.delete(ev.id)}
										aria-label={t('inbox.bulk_select_row')}
									/>
								{/if}
							</td>
							<td class="svc">{ev.parsed_payload.service_name ?? '—'}</td>
							<td class="num">{formatMoney(ev.parsed_payload.amount_minor, ev.parsed_payload.currency)}</td>
							<td>{ev.parsed_payload.billing_cycle ? t(`cycle.${ev.parsed_payload.billing_cycle}`) : '—'}</td>
							<td class="num">
								{#if ev.parsed_payload.recurring && ev.parsed_payload.months_seen}
									<span class="recur-pill">{tn('review.months', ev.parsed_payload.months_seen)}</span>
								{:else}
									<span class="muted">—</span>
								{/if}
							</td>
							<td class="muted small">
								{ev.parsed_payload.source_kind ? t(`source_kind.${ev.parsed_payload.source_kind}`) : '—'}
							</td>
							<td><span class="status-pill status-{ev.status}">{t(`status.${ev.status}`)}</span></td>
							<td class="muted small">
								{ev.parsed_payload.last_charged_at
									? formatTimestamp(ev.parsed_payload.last_charged_at)
									: '—'}
							</td>
							<td class="muted small">{formatTimestamp(ev.created_at)}</td>
							<td class="actions">
								{#if ev.status === 'pending'}
									<button class="act confirm" onclick={() => detectionEvents.confirm(ev.id)}>{t('inbox.confirm')}</button>
									<button class="act" onclick={() => startEdit(ev)}>{t('inbox.edit_confirm')}</button>
									<button class="act reject" onclick={() => detectionEvents.reject(ev.id)}>{t('inbox.reject')}</button>
								{:else if ev.status === 'rejected'}
									<button class="act restore" onclick={() => detectionEvents.restore(ev.id)}>{t('review.restore')}</button>
								{/if}
							</td>
						</tr>
						{#if editingId === ev.id}
							<tr class="edit-row">
								<td colspan="10">
									<div class="edit-grid">
										<label><span>{t('form.name')}</span><input bind:value={editName} required /></label>
										<label><span>{t('form.amount')}</span>
											<input type="number" bind:value={editAmount} min="0" step={editCurrency === 'JPY' ? '1' : '0.01'} />
										</label>
										<label><span>{t('form.currency')}</span>
											<select bind:value={editCurrency}>
												{#each CURRENCIES as c (c)}<option value={c}>{c}</option>{/each}
											</select>
										</label>
										<label><span>{t('form.billing_cycle')}</span>
											<select bind:value={editCycle}>
												<option value="weekly">{t('cycle.weekly')}</option>
												<option value="monthly">{t('cycle.monthly')}</option>
												<option value="quarterly">{t('cycle.quarterly')}</option>
												<option value="semi_annual">{t('cycle.semi_annual')}</option>
												<option value="annual">{t('cycle.annual')}</option>
												<option value="custom">{t('cycle.custom')}</option>
											</select>
										</label>
										<label><span>{t('form.next_billing_date')}</span><input type="date" bind:value={editNextBilling} /></label>
										<div class="edit-actions">
											<button class="act confirm" onclick={() => saveEdit(ev)}>{t('review.save')}</button>
											<button class="act" onclick={() => (editingId = null)}>{t('subs.cancel')}</button>
										</div>
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if detectionEvents.error}
		<p class="error">{t('common.error')}: {detectionEvents.error}</p>
	{/if}
</div>

<style>
	.container {
		max-width: 1100px;
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
		margin: 0 0 1.5rem;
		color: var(--kk-text-muted);
		font-size: 0.95rem;
		line-height: 1.5;
	}
	.toolbar {
		display: flex;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
		padding: 0.75rem 1rem;
		border-radius: var(--kk-radius-md);
		margin-bottom: 1rem;
	}
	.search {
		flex: 1 1 12rem;
		min-width: 8rem;
		padding: 0.5rem 0.75rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-family: inherit;
		font-size: 0.9rem;
	}
	.filter {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		color: var(--kk-text-muted);
	}
	.filter select {
		padding: 0.45rem 0.6rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-family: inherit;
		font-size: 0.85rem;
	}
	.count {
		margin-left: auto;
	}
	.bulk-reject {
		padding: 0.45rem 0.85rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: oklch(0.82 0.13 25 / 0.18);
		color: var(--color-accent-mochi);
		font-weight: 600;
		font-size: 0.85rem;
		cursor: pointer;
		font-family: inherit;
	}
	.table-wrap {
		border-radius: var(--kk-radius-md);
		overflow-x: auto;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.88rem;
	}
	thead th {
		text-align: left;
		font-weight: 600;
		color: var(--kk-text-muted);
		padding: 0.7rem 0.75rem;
		border-bottom: 1px solid var(--kk-stroke);
		white-space: nowrap;
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	th.sortable {
		cursor: pointer;
		user-select: none;
	}
	th.sortable:hover {
		color: var(--kk-text-primary);
	}
	th.num,
	td.num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	tbody td {
		padding: 0.65rem 0.75rem;
		border-bottom: 1px solid var(--kk-stroke);
		vertical-align: middle;
	}
	tbody tr:last-child td {
		border-bottom: none;
	}
	tr.editing td {
		border-bottom: none;
	}
	.svc {
		font-weight: 600;
		max-width: 18rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sel {
		width: 1.5rem;
		text-align: center;
	}
	.recur-pill {
		font-size: 0.72rem;
		padding: 0.1rem 0.45rem;
		border-radius: 999px;
		background: oklch(0.82 0.13 155 / 0.18);
		color: var(--color-accent-matcha);
		font-weight: 600;
		white-space: nowrap;
	}
	.status-pill {
		font-size: 0.72rem;
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.status-pending {
		background: var(--kk-surface-2);
		color: var(--kk-text-muted);
		border: 1px solid var(--kk-stroke);
	}
	.status-confirmed {
		background: oklch(0.82 0.13 155 / 0.2);
		color: var(--color-accent-matcha);
	}
	.status-rejected {
		background: oklch(0.82 0.13 25 / 0.2);
		color: var(--color-accent-mochi);
	}
	.actions {
		text-align: right;
		white-space: nowrap;
	}
	.act {
		padding: 0.35rem 0.6rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: transparent;
		color: var(--kk-text-muted);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		margin-left: 0.3rem;
	}
	.act.confirm {
		background: var(--color-accent-matcha);
		color: oklch(0.15 0.05 155);
		border-color: transparent;
	}
	.act.reject:hover {
		color: var(--color-accent-mochi);
	}
	.act.restore {
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		border-color: transparent;
	}
	.edit-row td {
		background: var(--kk-surface-2);
		padding: 1rem 1.25rem;
	}
	.edit-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
		gap: 0.6rem;
		align-items: end;
	}
	.edit-grid label {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.78rem;
		color: var(--kk-text-muted);
	}
	.edit-grid input,
	.edit-grid select {
		padding: 0.4rem 0.55rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface);
		color: var(--kk-text-primary);
		font-size: 0.85rem;
		font-family: inherit;
	}
	.edit-actions {
		display: flex;
		gap: 0.4rem;
	}
	.empty {
		padding: 2rem;
		text-align: center;
	}
	.small {
		font-size: 0.8rem;
	}
	.error {
		color: var(--color-accent-mochi);
		margin: 1rem 0 0;
		font-size: 0.9rem;
	}
</style>
