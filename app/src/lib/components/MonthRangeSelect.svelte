<script lang="ts">
	import { t, tn } from '$lib/i18n.svelte';

	type YearMonth = { year: number; month: number };

	let {
		value = $bindable<YearMonth[]>([]),
		years = 3
	}: {
		value?: YearMonth[];
		years?: number;
	} = $props();

	const today = new Date();
	const currentYear = today.getFullYear();
	const currentMonth = today.getMonth() + 1;

	const yearList = $derived(Array.from({ length: years }, (_, i) => currentYear - i));

	const monthAbbr = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

	function key(y: number, m: number) {
		return `${y}-${m}`;
	}

	function isSelected(y: number, m: number): boolean {
		return value.some((v) => v.year === y && v.month === m);
	}

	function isFuture(y: number, m: number): boolean {
		if (y > currentYear) return true;
		if (y === currentYear && m > currentMonth) return true;
		return false;
	}

	function toggle(y: number, m: number) {
		if (isFuture(y, m)) return;
		if (isSelected(y, m)) {
			value = value.filter((v) => !(v.year === y && v.month === m));
		} else {
			value = [...value, { year: y, month: m }];
		}
	}

	function selectAllYear(y: number) {
		const existing = new Set(value.map((v) => key(v.year, v.month)));
		for (let m = 1; m <= 12; m++) {
			if (!isFuture(y, m)) existing.add(key(y, m));
		}
		value = Array.from(existing)
			.map((k) => {
				const [yy, mm] = k.split('-').map(Number);
				return { year: yy, month: mm };
			})
			.sort((a, b) => (a.year - b.year) * 100 + (a.month - b.month));
	}

	function clearYear(y: number) {
		value = value.filter((v) => v.year !== y);
	}

	function clearAll() {
		value = [];
	}

	function selectLast12Months() {
		const out: YearMonth[] = [];
		let y = currentYear;
		let m = currentMonth;
		for (let i = 0; i < 12; i++) {
			out.push({ year: y, month: m });
			m -= 1;
			if (m === 0) {
				m = 12;
				y -= 1;
			}
		}
		value = out.sort((a, b) => (a.year - b.year) * 100 + (a.month - b.month));
	}

	function selectYear(year: number) {
		const out: YearMonth[] = [];
		for (let m = 1; m <= 12; m++) {
			if (!isFuture(year, m)) out.push({ year, month: m });
		}
		value = out;
	}
</script>

<div class="month-range">
	<div class="presets">
		<button type="button" onclick={selectLast12Months}>{t('range.last_12_months')}</button>
		<button type="button" onclick={() => selectYear(currentYear)}>{t('range.this_year')}</button>
		<button type="button" onclick={() => selectYear(currentYear - 1)}>{t('range.last_year')}</button>
	</div>
	{#each yearList as year (year)}
		<div class="year-row">
			<div class="year-head">
				<span class="year">{year}</span>
				<div class="row-actions">
					<button type="button" onclick={() => selectAllYear(year)}>{t('range.all')}</button>
					<button type="button" onclick={() => clearYear(year)}>{t('range.none')}</button>
				</div>
			</div>
			<div class="months">
				{#each monthAbbr as ma, i}
					{@const m = i + 1}
					<button
						type="button"
						class:selected={isSelected(year, m)}
						class:future={isFuture(year, m)}
						disabled={isFuture(year, m)}
						onclick={() => toggle(year, m)}
					>
						{ma}
					</button>
				{/each}
			</div>
		</div>
	{/each}
	{#if value.length > 0}
		<div class="footer">
			<span class="count">{tn('range.selected', value.length)}</span>
			<button type="button" class="clear-all" onclick={clearAll}>{t('range.clear_all')}</button>
		</div>
	{/if}
</div>

<style>
	.month-range {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.presets {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
		margin-bottom: 0.25rem;
	}
	.presets button {
		padding: 0.35rem 0.8rem;
		border-radius: 999px;
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-muted);
		font-size: 0.8rem;
		cursor: pointer;
		font-family: inherit;
	}
	.presets button:hover {
		color: var(--kk-text-primary);
	}
	.year-row {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.year-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.year {
		font-weight: 600;
		font-size: 0.95rem;
	}
	.row-actions {
		display: flex;
		gap: 0.6rem;
	}
	.row-actions button {
		font-size: 0.75rem;
		background: transparent;
		border: none;
		color: var(--kk-text-muted);
		cursor: pointer;
		text-decoration: underline;
		padding: 0;
		font-family: inherit;
	}
	.months {
		display: grid;
		grid-template-columns: repeat(12, 1fr);
		gap: 0.3rem;
	}
	.months button {
		padding: 0.45rem 0.2rem;
		border-radius: var(--kk-radius-sm);
		border: 1px solid var(--kk-stroke);
		background: var(--kk-surface-2);
		color: var(--kk-text-primary);
		font-size: 0.75rem;
		cursor: pointer;
		font-family: inherit;
		transition: background 80ms ease, border-color 80ms ease;
	}
	.months button:hover:not(:disabled) {
		background: var(--kk-surface-1);
	}
	.months button.selected {
		background: var(--color-accent-sora);
		color: oklch(0.15 0.05 245);
		border-color: var(--color-accent-sora);
		font-weight: 600;
	}
	.months button.future {
		opacity: 0.3;
		cursor: not-allowed;
	}
	.footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 0.25rem;
		font-size: 0.85rem;
		color: var(--kk-text-muted);
	}
	.clear-all {
		background: transparent;
		border: none;
		color: var(--kk-text-muted);
		cursor: pointer;
		text-decoration: underline;
		font-size: 0.85rem;
		padding: 0;
		font-family: inherit;
	}
	@media (max-width: 640px) {
		.months {
			grid-template-columns: repeat(6, 1fr);
		}
	}
</style>
