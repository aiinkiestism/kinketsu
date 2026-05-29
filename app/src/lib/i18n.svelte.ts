// Minimal runtime i18n.
//
// English is the source-of-truth dictionary. The user's locale is detected once
// at startup (navigator.language) and used to look up translations. Unknown
// keys or missing translations fall back to English, then to the key itself.
//
// The product name `kinketsu` is intentionally not part of any dictionary — it
// always appears as the ASCII string, never translated.

export type LocaleCode = 'en' | 'ja';

const dictionaries: Record<LocaleCode, Record<string, string>> = {
	en: {
		tagline: 'Every subscription, from your inbox.',

		'nav.dashboard': 'Dashboard',
		'nav.scan': 'Scan',
		'nav.settings': 'Settings',

		'dashboard.monthly_total': 'Monthly equivalent total (JPY)',
		'dashboard.active_one': '1 active subscription',
		'dashboard.active_other': '{count} active subscriptions',
		'dashboard.upcoming': 'Upcoming charges',
		'dashboard.upcoming_empty': 'No upcoming charges scheduled.',

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
		'form.submit': 'Save',

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
		'scan.missing_fields': 'The extracted hint is missing required fields (name, amount, currency, cycle).',

		'common.delete': 'Delete',
		'common.error': 'Error',
		'common.loading': 'Loading…'
	},
	ja: {
		tagline: 'サブスクの全貌、メールから。',

		'nav.dashboard': 'ダッシュボード',
		'nav.scan': 'スキャン',
		'nav.settings': '設定',

		'dashboard.monthly_total': '月額換算合計 (JPY)',
		'dashboard.active_one': 'アクティブ 1 件',
		'dashboard.active_other': 'アクティブ {count} 件',
		'dashboard.upcoming': 'このあと請求',
		'dashboard.upcoming_empty': '予定された請求はありません。',

		'subs.heading': 'サブスク一覧',
		'subs.add': '+ 追加',
		'subs.cancel': 'キャンセル',
		'subs.empty': 'まだサブスクが登録されていません。「+ 追加」から最初の 1 件を。',
		'subs.payment_method': '支払い方法',
		'subs.category': 'カテゴリ',
		'subs.none': '—',

		'form.name': '名前',
		'form.name_placeholder': '例: Netflix',
		'form.amount': '金額',
		'form.currency': '通貨',
		'form.billing_cycle': '請求サイクル',
		'form.submit': '登録',

		'cycle.weekly': '週次',
		'cycle.monthly': '月次',
		'cycle.quarterly': '四半期',
		'cycle.semi_annual': '半年',
		'cycle.annual': '年次',
		'cycle.custom': 'カスタム',

		'manage.heading': '管理',
		'manage.payment_methods': '支払い方法',
		'manage.categories': 'カテゴリ',
		'manage.empty_pm': 'まだ支払い方法がありません。',
		'manage.empty_cat': 'まだカテゴリがありません。',

		'kind.credit_card': 'クレジットカード',
		'kind.debit_card': 'デビットカード',
		'kind.bank_account': '銀行口座',
		'kind.paypal': 'PayPal',
		'kind.carrier': 'キャリア決済',
		'kind.wallet': 'モバイルウォレット',
		'kind.app_store': 'App Store',
		'kind.play_store': 'Google Play',
		'kind.crypto': '暗号資産',
		'kind.other': 'その他',

		'settings.heading': '設定',
		'settings.llm_heading': 'LLM プロバイダー',
		'settings.llm_description':
			'kinketsu はメール明細の読み取りに LLM を使います。プロバイダーを選択して認証情報を入力してください。Ollama と LM Studio はローカル実行なので情報は端末から出ません。',
		'settings.provider': 'プロバイダー',
		'settings.api_key': 'API キー',
		'settings.endpoint': 'エンドポイント',
		'settings.model': 'モデル',
		'settings.save': '保存',
		'settings.saved': '保存しました。',

		'scan.heading': '明細をスキャン',
		'scan.description':
			'サブスクの確認メールや更新メールをここに貼り付けてください。設定で選んだ LLM が構造化フィールドを抽出します。Gmail 連携の前にプロバイダーの動作確認に使えます。',
		'scan.placeholder': 'サブスクの明細メール本文をここに貼り付け…',
		'scan.extract': '抽出',
		'scan.result_heading': '抽出結果',
		'scan.charged_at': '請求日',
		'scan.save_as_sub': 'サブスクとして保存',
		'scan.no_provider': 'LLM プロバイダーが未設定です。',
		'scan.go_settings': '設定を開く',
		'scan.missing_fields': '必須フィールド(名前・金額・通貨・サイクル)が不足しています。',

		'common.delete': '削除',
		'common.error': 'エラー',
		'common.loading': '読み込み中…'
	}
};

class I18n {
	locale = $state<LocaleCode>('en');

	init() {
		if (typeof navigator === 'undefined') return;
		const tag = navigator.language?.toLowerCase() ?? 'en';
		this.locale = tag.startsWith('ja') ? 'ja' : 'en';
	}

	setLocale(code: LocaleCode) {
		this.locale = code;
	}

	get bcp47(): string {
		return this.locale === 'ja' ? 'ja-JP' : 'en-US';
	}
}

export const i18n = new I18n();

export function t(key: string, vars?: Record<string, string | number>): string {
	let str = dictionaries[i18n.locale]?.[key] ?? dictionaries.en[key] ?? key;
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
