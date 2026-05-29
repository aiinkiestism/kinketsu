<script lang="ts">
	import { onMount } from 'svelte';
	import { subscriptions } from '$lib/stores/subscriptions.svelte';
	import type { BillingCycle } from '$lib/types';

	let showForm = $state(false);
	let formName = $state('');
	let formAmount = $state<number>(0);
	let formCurrency = $state('JPY');
	let formCycle = $state<BillingCycle>('monthly');

	function resetForm() {
		formName = '';
		formAmount = 0;
		formCurrency = 'JPY';
		formCycle = 'monthly';
	}

	function toMinor(amount: number, currency: string): number {
		// JPY has no fractional unit; everything else assumes 1/100 minor units.
		return currency === 'JPY' ? Math.round(amount) : Math.round(amount * 100);
	}

	function fromMinor(amount_minor: number, currency: string): number {
		return currency === 'JPY' ? amount_minor : amount_minor / 100;
	}

	function formatMoney(amount_minor: number, currency: string): string {
		try {
			return new Intl.NumberFormat('ja-JP', {
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

	const CYCLE_LABEL: Record<BillingCycle, string> = {
		weekly: '週次',
		monthly: '月次',
		quarterly: '四半期',
		semi_annual: '半年',
		annual: '年次',
		custom: 'カスタム'
	};

	function monthlyEquivalentMinor(amount_minor: number, cycle: BillingCycle): number {
		return amount_minor / CYCLE_MONTHS[cycle];
	}

	let monthlyTotalJpy = $derived(
		subscriptions.items
			.filter((s) => s.status === 'active' && s.currency === 'JPY')
			.reduce((sum, s) => sum + monthlyEquivalentMinor(s.amount_minor, s.billing_cycle), 0)
	);

	let activeCount = $derived(subscriptions.items.filter((s) => s.status === 'active').length);

	async function handleAdd(e: SubmitEvent) {
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
				payment_method_id: null,
				category_id: null,
				status: null,
				notes: null
			});
			showForm = false;
			resetForm();
		} catch {
			// store.error is already set; surfaced by the error banner below.
		}
	}

	onMount(() => subscriptions.load());
</script>

<div class="container">
	<header class="title">
		<h1>金欠<span class="kana">きんけつ</span></h1>
		<p class="tagline">サブスクの全貌、メールから。</p>
	</header>

	<section class="grid">
		<article class="glass card">
			<h2>月額換算合計 (JPY)</h2>
			<p class="big">
				{new Intl.NumberFormat('ja-JP', {
					style: 'currency',
					currency: 'JPY',
					maximumFractionDigits: 0
				}).format(monthlyTotalJpy)}
			</p>
			<p class="muted">{activeCount} active subscription{activeCount === 1 ? '' : 's'}</p>
		</article>
		<article class="glass card">
			<h2>このあと請求</h2>
			<p class="muted">no upcoming charges scheduled</p>
		</article>
	</section>

	<section class="list-section">
		<div class="list-header">
			<h2>登録済みサブスク</h2>
			<button class="glass-subtle add-btn" onclick={() => (showForm = !showForm)}>
				{showForm ? 'キャンセル' : '+ 追加'}
			</button>
		</div>

		{#if showForm}
			<form class="glass form" onsubmit={handleAdd}>
				<label>
					<span>名前</span>
					<input bind:value={formName} placeholder="e.g. Netflix" required />
				</label>
				<label>
					<span>金額</span>
					<input type="number" bind:value={formAmount} min="0" step="1" required />
				</label>
				<label>
					<span>通貨</span>
					<select bind:value={formCurrency}>
						<option value="JPY">JPY</option>
						<option value="USD">USD</option>
						<option value="EUR">EUR</option>
						<option value="GBP">GBP</option>
					</select>
				</label>
				<label>
					<span>請求サイクル</span>
					<select bind:value={formCycle}>
						<option value="weekly">週次</option>
						<option value="monthly">月次</option>
						<option value="quarterly">四半期</option>
						<option value="semi_annual">半年</option>
						<option value="annual">年次</option>
					</select>
				</label>
				<button type="submit" class="submit">登録</button>
			</form>
		{/if}

		{#if subscriptions.error}
			<p class="error">エラー: {subscriptions.error}</p>
		{/if}

		{#if subscriptions.loading}
			<p class="muted">読み込み中...</p>
		{:else if subscriptions.items.length === 0}
			<article class="glass card empty">
				<p class="muted">まだサブスクが登録されていません。「+ 追加」から最初の1件を。</p>
			</article>
		{:else}
			<ul class="sub-list">
				{#each subscriptions.items as sub (sub.id)}
					<li class="glass sub-row">
						<div class="sub-main">
							<span class="sub-name">{sub.name}</span>
							{#if sub.plan}<span class="sub-plan">{sub.plan}</span>{/if}
						</div>
						<span class="sub-cycle">{CYCLE_LABEL[sub.billing_cycle]}</span>
						<span class="sub-amount">{formatMoney(sub.amount_minor, sub.currency)}</span>
						<button
							class="del"
							onclick={() => subscriptions.remove(sub.id)}
							aria-label="削除"
						>×</button>
					</li>
				{/each}
			</ul>
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
		letter-spacing: -0.02em;
	}
	.title .kana {
		font-size: 0.4em;
		font-weight: 500;
		margin-left: 0.5em;
		opacity: 0.5;
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
	.list-section h2 {
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
	.add-btn {
		border: none;
		padding: 0.5rem 1rem;
		border-radius: var(--kk-radius-sm);
		color: var(--kk-text-primary);
		cursor: pointer;
		font-size: 0.9rem;
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
	}
	.sub-name {
		font-weight: 600;
	}
	.sub-plan {
		font-size: 0.8rem;
		color: var(--kk-text-muted);
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
	@media (max-width: 640px) {
		.grid {
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
	}
</style>
