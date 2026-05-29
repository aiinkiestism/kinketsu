// Hand-mirrored from crates/core/src/models/*.rs.
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

export interface Category {
	id: string;
	name: string;
	icon: string | null;
	color: string | null;
	created_at: string;
	updated_at: string;
}
