// Hand-mirrored from crates/core/src/models/*.rs and crates/core/src/llm/mod.rs.
// TODO: replace with generated types (candidate: ts-rs or specta).

export type BillingCycle =
	| 'weekly'
	| 'monthly'
	| 'quarterly'
	| 'semi_annual'
	| 'annual'
	| 'custom';

export type SubscriptionStatus = 'active' | 'trial' | 'paused' | 'cancelled';

export type PaymentMethodKind =
	| 'credit_card'
	| 'debit_card'
	| 'bank_account'
	| 'paypal'
	| 'carrier'
	| 'wallet'
	| 'app_store'
	| 'play_store'
	| 'crypto'
	| 'other';

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

export interface Subscription {
	id: string;
	name: string;
	service_icon: string | null;
	plan: string | null;
	amount_minor: number;
	currency: string;
	billing_cycle: BillingCycle;
	next_billing_date: string | null;
	started_at: string | null;
	payment_method_id: string | null;
	category_id: string | null;
	status: SubscriptionStatus;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export interface NewSubscription {
	name: string;
	service_icon: string | null;
	plan: string | null;
	amount_minor: number;
	currency: string;
	billing_cycle: BillingCycle;
	next_billing_date: string | null;
	started_at: string | null;
	payment_method_id: string | null;
	category_id: string | null;
	status: SubscriptionStatus | null;
	notes: string | null;
}

export interface PaymentMethod {
	id: string;
	name: string;
	kind: PaymentMethodKind;
	last4: string | null;
	color: string | null;
	icon: string | null;
	created_at: string;
	updated_at: string;
}

export interface NewPaymentMethod {
	name: string;
	kind: PaymentMethodKind;
	last4: string | null;
	color: string | null;
	icon: string | null;
}

export interface Category {
	id: string;
	name: string;
	icon: string | null;
	color: string | null;
	created_at: string;
	updated_at: string;
}

export interface NewCategory {
	name: string;
	icon: string | null;
	color: string | null;
}

// ---- LLM provider configuration ----

export type LlmProviderKind = 'claude' | 'openai' | 'gemini' | 'ollama' | 'lmstudio';

export type LlmConfig =
	| { provider: 'claude'; api_key: string; model: string }
	| { provider: 'openai'; api_key: string; model: string }
	| { provider: 'gemini'; api_key: string; model: string }
	| { provider: 'ollama'; endpoint: string; model: string }
	| { provider: 'lmstudio'; endpoint: string; model: string };

export const LLM_PROVIDERS: LlmProviderKind[] = [
	'claude',
	'openai',
	'gemini',
	'ollama',
	'lmstudio'
];

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
