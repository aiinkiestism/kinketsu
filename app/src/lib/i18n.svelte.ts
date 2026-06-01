// Runtime i18n: English is the source of truth. For non-English locales we
// translate the entire dictionary via the user's configured LLM provider on
// first use, persist the result in the kinketsu settings DB, and look it up
// in-memory thereafter.
//
// Brand rule: the product name "kinketsu" never appears in any translation —
// the LLM prompt forbids it.

import { invoke } from '@tauri-apps/api/core';

export type LocaleCode = 'en' | 'ja';

const LOCALE_STORAGE_KEY = 'kinketsu.locale';

/**
 * English source dictionary. Every visible UI string lives here. The backend
 * notification templates are also keyed by their literal English text so they
 * round-trip through the same translation cache.
 */
const EN: Record<string, string> = {
	tagline: 'Every subscription, from your inbox.',

	'nav.dashboard': 'Dashboard',
	'nav.inbox': 'Inbox',
	'nav.scan': 'Scan',
	'nav.settings': 'Settings',

	'dashboard.monthly_total': 'Monthly equivalent total ({currency})',
	'dashboard.active_one': '1 active subscription',
	'dashboard.active_other': '{count} active subscriptions',
	'dashboard.upcoming': 'Upcoming charges',
	'dashboard.upcoming_empty': 'No upcoming charges scheduled.',
	'dashboard.export_ics': 'Export .ics',
	'dashboard.unconvertible_one': '1 subscription has no rate yet — refresh in Settings.',
	'dashboard.unconvertible_other':
		'{count} subscriptions have no rate yet — refresh in Settings.',

	'subs.heading': 'Subscriptions',
	'subs.add': '+ Add',
	'subs.cancel': 'Cancel',
	'subs.empty': 'No subscriptions yet — use “+ Add” to register your first one.',
	'subs.payment_method': 'Payment method',
	'subs.category': 'Category',
	'subs.none': '—',

	'form.name': 'Name',
	'form.name_placeholder': 'e.g. Netflix',
	'form.amount': 'Amount',
	'form.currency': 'Currency',
	'form.billing_cycle': 'Billing cycle',
	'form.status': 'Status',
	'form.plan': 'Plan',
	'form.next_billing_date': 'Next billing date',
	'form.started_at': 'Started at',
	'form.notes': 'Notes',
	'form.submit': 'Save',
	'form.update': 'Update',
	'form.edit_heading': 'Edit subscription',

	'sub_status.active': 'Active',
	'sub_status.trial': 'Trial',
	'sub_status.paused': 'Paused',
	'sub_status.cancelled': 'Cancelled',

	'common.edit': 'Edit',
	'common.delete': 'Delete',
	'common.error': 'Error',
	'common.loading': 'Loading…',
	'common.show': 'Show',
	'common.hide': 'Hide',

	'cycle.weekly': 'Weekly',
	'cycle.monthly': 'Monthly',
	'cycle.quarterly': 'Quarterly',
	'cycle.semi_annual': 'Semi-annual',
	'cycle.annual': 'Annual',
	'cycle.custom': 'Custom',

	'manage.heading': 'Manage',
	'manage.payment_methods': 'Payment methods',
	'manage.categories': 'Categories',
	'manage.empty_pm': 'No payment methods yet.',
	'manage.empty_cat': 'No categories yet.',

	'kind.credit_card': 'Credit card',
	'kind.debit_card': 'Debit card',
	'kind.bank_account': 'Bank account',
	'kind.paypal': 'PayPal',
	'kind.carrier': 'Carrier billing',
	'kind.wallet': 'Mobile wallet',
	'kind.app_store': 'App Store',
	'kind.play_store': 'Google Play',
	'kind.crypto': 'Crypto',
	'kind.other': 'Other',

	'settings.heading': 'Settings',
	'settings.llm_heading': 'LLM provider',
	'settings.llm_description':
		'kinketsu uses an LLM to read your subscription receipts. Pick a provider and paste your credentials — Ollama and LM Studio run locally so nothing leaves your machine.',
	'settings.provider': 'Provider',
	'settings.api_key': 'API key',
	'settings.endpoint': 'Endpoint',
	'settings.model': 'Model',
	'settings.save': 'Save',
	'settings.saved': 'Saved.',

	'settings.gmail_heading': 'Gmail integration',
	'settings.gmail_description':
		'kinketsu uses your own Google Cloud OAuth client to read subscription emails. Create a Desktop-app OAuth client at console.cloud.google.com/apis/credentials and paste the client ID + secret here.',
	'settings.gmail_client_id': 'Client ID',
	'settings.gmail_client_secret': 'Client secret',
	'settings.gmail_status': 'Status:',
	'settings.gmail_connected': 'Connected',
	'settings.gmail_not_connected': 'Not connected',
	'settings.gmail_disconnect': 'Disconnect',
	'settings.gmail_save_creds': 'Save credentials',

	'settings.paypal_heading': 'PayPal integration',
	'settings.paypal_description':
		'kinketsu uses your PayPal Developer OAuth client (Log In with PayPal). Create one at developer.paypal.com/dashboard/applications and paste the client ID + secret. Scan is wired through token refresh only — the Transaction Search API integration is scoped for a follow-up.',
	'settings.paypal_connected': 'Connected',
	'settings.paypal_not_connected': 'Not connected',

	'settings.notifications_heading': 'Renewal notifications',
	'settings.notifications_description':
		'kinketsu sends a system notification when an active subscription is within 7 days of its next billing date. A background check runs once a day; the button below triggers it on demand.',
	'settings.notifications_check_now': 'Check now',
	'settings.notifications_result_zero': 'No upcoming renewals in the next 7 days.',
	'settings.notifications_result_one': 'Sent 1 notification.',
	'settings.notifications_result_other': 'Sent {count} notifications.',

	'settings.rates_heading': 'Exchange rates',
	'settings.rates_description':
		'kinketsu caches exchange rates against your base currency so the dashboard total reflects all active subscriptions, regardless of currency.',
	'settings.rates_last': 'Last refreshed:',
	'settings.rates_never': 'never',
	'settings.rates_refresh': 'Refresh rates',
	'settings.rates_count': '{count} rates cached',
	'settings.rates_default_currency': 'Default currency',

	'settings.language_heading': 'Language',
	'settings.language_description':
		'kinketsu auto-detects your locale from the browser; override it here if you prefer. Non-English locales translate the UI through your configured LLM provider on first switch.',
	'settings.language_auto': 'Auto (browser)',
	'settings.language_en': 'English',
	'settings.language_ja': '日本語',
	'settings.language_translating': 'Translating…',

	'scan.mode_single': 'Single receipt',
	'scan.mode_csv': 'CSV import',
	'scan.csv_description':
		'Paste a CSV export from a bank, card, or PayPal Activity report. The configured LLM identifies recurring subscription rows and queues them in the Inbox for review.',
	'scan.csv_placeholder': 'Paste CSV rows (or any multi-row receipt text) here…',
	'scan.csv_import': 'Import',
	'scan.csv_result_zero': 'Import complete — no subscription-like rows detected.',
	'scan.csv_result_one': 'Import complete — 1 entry queued in the Inbox for review.',
	'scan.csv_result_other': 'Import complete — {count} entries queued in the Inbox for review.',
	'scan.csv_go_inbox': 'Open Inbox',

	'scan.heading': 'Scan a receipt',
	'scan.description':
		'Paste a subscription confirmation or renewal email below. The LLM you picked in Settings extracts the structured fields. Use this to validate your provider before connecting Gmail.',
	'scan.placeholder': 'Paste the body of a subscription email here…',
	'scan.extract': 'Extract',
	'scan.result_heading': 'Extracted',
	'scan.charged_at': 'Charged at',
	'scan.save_as_sub': 'Save as subscription',
	'scan.no_provider': 'You haven’t configured an LLM provider yet.',
	'scan.go_settings': 'Open Settings',
	'scan.missing_fields':
		'The extracted hint is missing required fields (name, amount, currency, cycle).',

	'inbox.heading': 'Subscription inbox',
	'inbox.description':
		'kinketsu queues every subscription it detects from your connected sources. Confirm to add them to your list; reject if it was misidentified.',
	'inbox.sources_heading': 'Sources',
	'inbox.sources_description':
		'Connect external sources to start detecting subscriptions automatically.',
	'inbox.gmail_coming_soon': 'Coming soon — Google account integration is in development.',
	'inbox.paypal_coming_soon':
		'PayPal personal accounts cannot use the Transactions API. Connect to verify your identity, then rely on (a) Gmail parsing for PayPal email receipts and (b) PayPal Activity CSV import (coming soon).',
	'inbox.connect_gmail': 'Connect Gmail',
	'inbox.connect_paypal': 'Connect PayPal',
	'inbox.range_heading': 'Scan range',
	'inbox.range_description':
		'Pick the months kinketsu should scan when you trigger a source sync.',
	'inbox.pending_heading': 'Pending review',
	'inbox.reviewed_heading': 'Recently reviewed',
	'inbox.empty':
		'Nothing to review yet. Connect a source above or paste a receipt on the Scan page.',
	'inbox.confirm': 'Confirm',
	'inbox.edit_confirm': 'Edit & confirm',
	'inbox.reject': 'Reject',
	'inbox.scan_run': 'Run Gmail scan',
	'inbox.scan_running': 'Scanning…',
	'inbox.scan_complete_zero': 'Scan complete — no new subscriptions detected.',
	'inbox.scan_complete_one': 'Scan complete — 1 new detection.',
	'inbox.scan_complete_other': 'Scan complete — {count} new detections.',
	'inbox.scan_needs_range': 'Select at least one month to scan.',
	'inbox.scan_needs_creds': 'Save Gmail credentials in Settings first.',
	'inbox.scan_needs_connection': 'Connect Gmail first.',
	'inbox.scan_needs_llm': 'Configure an LLM provider in Settings first.',
	'inbox.scan_cancelled': 'Scan cancelled.',
	'inbox.connect_gmail_loading': 'Opening browser…',
	'inbox.gmail_connected': 'Gmail connected',
	'inbox.gmail_disconnect': 'Disconnect',

	'source.gmail': 'Gmail',
	'source.paypal': 'PayPal',
	'source.csv_import': 'CSV import',
	'source.manual': 'Manual',

	'status.pending': 'Pending',
	'status.confirmed': 'Confirmed',
	'status.rejected': 'Rejected',
	'status.duplicate': 'Duplicate',

	'range.all': 'All',
	'range.none': 'None',
	'range.selected_one': '1 month selected',
	'range.selected_other': '{count} months selected',
	'range.clear_all': 'Clear all',
	'range.last_12_months': 'Last 12 months',
	'range.this_year': 'This year',
	'range.last_year': 'Last year',

	// Backend notification templates — keyed by their literal text so the
	// Rust-side notify_renewals can index the same translation cache.
	'{name} renews soon': '{name} renews soon',
	'Next charge: {date}': 'Next charge: {date}'
};

class I18n {
	locale = $state<LocaleCode>('en');
	translations = $state<Record<string, string>>({});
	translating = $state(false);

	async init() {
		let resolved: LocaleCode = 'en';

		try {
			const stored = await invoke<string | null>('get_user_locale');
			if (stored === 'en' || stored === 'ja') {
				resolved = stored;
			}
		} catch {
			/* ignore — fall back to localStorage / navigator */
		}

		if (resolved === 'en' && typeof window !== 'undefined') {
			const ls = window.localStorage.getItem(LOCALE_STORAGE_KEY);
			if (ls === 'en' || ls === 'ja') {
				resolved = ls;
			} else if (typeof navigator !== 'undefined') {
				const tag = navigator.language?.toLowerCase() ?? 'en';
				resolved = tag.startsWith('ja') ? 'ja' : 'en';
			}
		}

		this.locale = resolved;
		if (resolved !== 'en') {
			await this.ensureTranslations();
		}
	}

	async setLocale(code: LocaleCode) {
		this.locale = code;
		if (typeof window !== 'undefined') {
			window.localStorage.setItem(LOCALE_STORAGE_KEY, code);
		}
		try {
			await invoke('set_user_locale', { locale: code });
		} catch {
			/* persistence is best-effort */
		}
		if (code === 'en') {
			this.translations = {};
		} else {
			await this.ensureTranslations();
		}
	}

	private async ensureTranslations() {
		// Pull whatever's already cached in the DB.
		let cached: Record<string, string> | null = null;
		try {
			cached = await invoke<Record<string, string> | null>('get_translations', {
				locale: this.locale
			});
		} catch {
			/* ignore */
		}
		if (cached) {
			this.translations = cached;
		}

		// Anything still missing? Send to the configured LLM.
		const missingKeys = Object.keys(EN).filter((k) => !this.translations[k]);
		if (missingKeys.length === 0) return;

		const subset: Record<string, string> = {};
		for (const k of missingKeys) subset[k] = EN[k];

		this.translating = true;
		try {
			const result = await invoke<Record<string, string>>('translate_strings', {
				targetLocale: this.locale,
				strings: subset
			});
			const merged = { ...this.translations, ...result };
			this.translations = merged;
			try {
				await invoke('save_translations', {
					locale: this.locale,
					translations: merged
				});
			} catch {
				/* persistence is best-effort */
			}
		} catch (e) {
			// Translation failed — UI stays in English. Surface to console only.
			console.warn('kinketsu: translation failed, staying in English fallback', e);
		} finally {
			this.translating = false;
		}
	}

	get bcp47(): string {
		return this.locale === 'ja' ? 'ja-JP' : 'en-US';
	}
}

export const i18n = new I18n();

export function t(key: string, vars?: Record<string, string | number>): string {
	let str = i18n.translations[key] ?? EN[key] ?? key;
	if (vars) {
		for (const [k, v] of Object.entries(vars)) {
			str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
		}
	}
	return str;
}

/** Plural helper: picks `<key>_one` for count 1, `<key>_other` otherwise. */
export function tn(
	key: string,
	count: number,
	vars?: Record<string, string | number>
): string {
	const suffix = count === 1 ? '_one' : '_other';
	return t(`${key}${suffix}`, { count, ...vars });
}
