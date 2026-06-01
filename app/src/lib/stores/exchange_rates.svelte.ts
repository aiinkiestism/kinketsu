import { invoke } from '@tauri-apps/api/core';
import { LOCALE_DEFAULT_CURRENCY, minorPerMajor, type ExchangeRate } from '$lib/types';

class ExchangeRatesStore {
	base = $state('JPY');
	items = $state<ExchangeRate[]>([]);
	loading = $state(false);
	refreshing = $state(false);
	error = $state<string | null>(null);

	byQuote = $derived(new Map(this.items.map((r) => [r.quote, r.rate])));
	lastFetched = $derived(this.items.length > 0 ? this.items[0].fetched_at : null);

	/**
	 * Initialize the store's base currency. Reads the persisted default from the
	 * settings store; falls back to a locale-derived default; finally JPY.
	 */
	async init(locale: string) {
		try {
			const stored = await invoke<string | null>('get_default_currency');
			if (stored) {
				this.base = stored;
			} else {
				this.base = LOCALE_DEFAULT_CURRENCY[locale] ?? 'JPY';
			}
		} catch {
			this.base = LOCALE_DEFAULT_CURRENCY[locale] ?? 'JPY';
		}
		await this.load();
	}

	async setBase(currency: string) {
		this.base = currency;
		try {
			await invoke('set_default_currency', { currency });
		} catch (e) {
			this.error = String(e);
		}
		await this.load();
	}

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
	 * Convert `amount_minor` in `currency` into minor units of the configured
	 * base currency, using the cached rates. Returns null when no rate is
	 * available (e.g. user hasn't refreshed yet, or the API doesn't quote the
	 * target).
	 *
	 * Convention: rates are stored with this.base on the left side, so
	 * `rate[X]` means "X units per 1 base". To convert X amount to base, divide
	 * by rate[X]. Minor-unit ratios respect JPY (1:1) vs. others (1:100).
	 */
	toBaseMinor(amount_minor: number, currency: string): number | null {
		if (currency === this.base) return amount_minor;
		const rate = this.byQuote.get(currency);
		if (!rate || rate === 0) return null;
		const amount_major = amount_minor / minorPerMajor(currency);
		const base_major = amount_major / rate;
		return Math.round(base_major * minorPerMajor(this.base));
	}
}

export const exchangeRates = new ExchangeRatesStore();
