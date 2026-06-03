// App-level constants and derived helper types.
//
// All *data shapes* (Subscription, DetectionEvent, LlmConfig, …) come from the
// specta-generated `$lib/bindings`. This file holds only runtime values the
// backend doesn't model — currency lists, per-provider UI defaults, and small
// enum-mirroring arrays — plus thin types derived from the generated ones.

import type { LlmConfig, PaymentMethodKind, SubscriptionStatus } from '$lib/bindings';

/**
 * Default currency by locale. Used as the seeded preference when the user
 * hasn't picked a default currency yet. The dashboard total and the
 * subscription form pre-select this value.
 */
export const LOCALE_DEFAULT_CURRENCY: Record<string, string> = {
	en: 'USD',
	ja: 'JPY'
};

/**
 * Minor-unit divisor per ISO 4217 currency. Most currencies use 100 minor units
 * per major unit (cents); JPY is the notable exception with 1:1.
 */
export function minorPerMajor(currency: string): number {
	return currency === 'JPY' ? 1 : 100;
}

export const CURRENCIES: readonly string[] = [
	'JPY',
	'USD',
	'EUR',
	'GBP',
	'CHF',
	'CAD',
	'AUD',
	'NZD',
	'CNY',
	'HKD',
	'SGD',
	'KRW',
	'TWD',
	'INR',
	'BRL',
	'MXN',
	'ZAR',
	'SEK',
	'NOK',
	'DKK',
	'PLN',
	'CZK',
	'HUF',
	'TRY',
	'THB',
	'IDR',
	'PHP',
	'MYR',
	'VND',
	'AED'
] as const;

export const SUBSCRIPTION_STATUSES: SubscriptionStatus[] = [
	'active',
	'trial',
	'paused',
	'cancelled'
];

export const PAYMENT_METHOD_KINDS: PaymentMethodKind[] = [
	'credit_card',
	'debit_card',
	'bank_account',
	'paypal',
	'carrier',
	'wallet',
	'app_store',
	'play_store',
	'crypto',
	'other'
];

// ---- LLM provider configuration ----

// The discriminant of the generated `LlmConfig` union — stays in sync with the
// Rust provider enum automatically.
export type LlmProviderKind = LlmConfig['provider'];

export const LLM_PROVIDERS: LlmProviderKind[] = ['claude', 'openai', 'gemini', 'ollama', 'lmstudio'];

export const LLM_PROVIDER_LABEL: Record<LlmProviderKind, string> = {
	claude: 'Claude',
	openai: 'OpenAI',
	gemini: 'Gemini',
	ollama: 'Ollama',
	lmstudio: 'LM Studio'
};

export interface LlmProviderDefaults {
	model: string;
	key_hint?: string;
	endpoint?: string;
}

export const LLM_DEFAULTS: Record<LlmProviderKind, LlmProviderDefaults> = {
	claude: { model: 'claude-sonnet-4-5', key_hint: 'sk-ant-...' },
	openai: { model: 'gpt-5-mini', key_hint: 'sk-...' },
	gemini: { model: 'gemini-2.5-flash', key_hint: 'AIza...' },
	ollama: { model: 'llama3.1:8b', endpoint: 'http://localhost:11434' },
	lmstudio: { model: 'any-loaded-model', endpoint: 'http://localhost:1234' }
};

export function isCloudProvider(p: LlmProviderKind): boolean {
	return p === 'claude' || p === 'openai' || p === 'gemini';
}
