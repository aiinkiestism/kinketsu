import { invoke } from '@tauri-apps/api/core';
import type { ExchangeRate } from '$lib/types';

class ExchangeRatesStore {
	base = $state('JPY');
	items = $state<ExchangeRate[]>([]);
	loading = $state(false);
	refreshing = $state(false);
	error = $state<string | null>(null);

	byQuote = $derived(new Map(this.items.map((r) => [r.quote, r.rate])));
	lastFetched = $derived(this.items.length > 0 ? this.items[0].fetched_at : null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.items = await invoke<ExchangeRate[]>('list_exchange_rates', { base: this.base });
		} catch (e) {
			this.error = String(e);
			this.items = [];
		} finally {
			this.loading = false;
		}
	}

	async refresh() {
		this.refreshing = true;
		this.error = null;
		try {
			await invoke<number>('refresh_exchange_rates', { base: this.base });
			await this.load();
		} catch (e) {
			this.error = String(e);
		} finally {
			this.refreshing = false;
		}
	}

	/**
	 * Convert an `amount_minor` value in `currency` into JPY minor units (= yen)
	 * using the cached rates. Returns null when no rate is available.
	 *
	 * Conventions: JPY has no fractional unit so 1 minor == 1 yen. Other ISO 4217
	 * currencies assume 1 major == 100 minor (cents). Rates are stored with
	 * base = "JPY", so `rate[currency]` is currency-per-JPY (e.g. USD = 0.0064).
	 */
	toJpyMinor(amount_minor: number, currency: string): number | null {
		if (currency === 'JPY') return amount_minor;
		const rate = this.byQuote.get(currency);
		if (!rate || rate === 0) return null;
		const major = amount_minor / 100;
		return Math.round(major / rate);
	}
}

export const exchangeRates = new ExchangeRatesStore();
