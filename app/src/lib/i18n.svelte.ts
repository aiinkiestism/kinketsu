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
		'nav.inbox': 'Inbox',
		'nav.scan': 'Scan',
		'nav.settings': 'Settings',

		'dashboard.monthly_total': 'Monthly equivalent total (JPY)',
		'dashboard.active_one': '1 active subscription',
		'dashboard.active_other': '{count} active subscriptions',
		'dashboard.upcoming': 'Upcoming charges',
		'dashboard.upcoming_empty': 'No upcoming charges scheduled.',
		'dashboard.export_ics': 'Export .ics',
		'dashboard.unconvertible_one': '1 subscription has no rate yet — refresh in Settings.',
		'dashboard.unconvertible_other': '{count} subscriptions have no rate yet — refresh in Settings.',

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

		'settings.language_heading': 'Language',
		'settings.language_description':
			'kinketsu auto-detects your locale from the browser; override it here if you prefer.',
		'settings.language_auto': 'Auto (browser)',
		'settings.language_en': 'English',
		'settings.language_ja': '日本語',

		'range.last_12_months': 'Last 12 months',
		'range.this_year': 'This year',
		'range.last_year': 'Last year',

		'common.show': 'Show',
		'common.hide': 'Hide',

		'settings.rates_heading': 'Exchange rates',
		'settings.rates_description':
			'kinketsu caches exchange rates against JPY so the dashboard total reflects all active subscriptions, regardless of currency.',
		'settings.rates_last': 'Last refreshed:',
		'settings.rates_never': 'never',
		'settings.rates_refresh': 'Refresh rates',
		'settings.rates_count': '{count} rates cached',

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

		'inbox.heading': 'Subscription inbox',
		'inbox.description':
			'kinketsu queues every subscription it detects from your connected sources. Confirm to add them to your list; reject if it was misidentified.',
		'inbox.sources_heading': 'Sources',
		'inbox.sources_description':
			'Connect external sources to start detecting subscriptions automatically.',
		'inbox.gmail_coming_soon': 'Coming soon — Google account integration is in development.',
		'inbox.paypal_coming_soon': 'Coming soon — PayPal subscription import is in development.',
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

		'common.delete': 'Delete',
		'common.error': 'Error',
		'common.loading': 'Loading…'
	},
	ja: {
		tagline: 'サブスクの全貌、メールから。',

		'nav.dashboard': 'ダッシュボード',
		'nav.inbox': 'インボックス',
		'nav.scan': 'スキャン',
		'nav.settings': '設定',

		'dashboard.monthly_total': '月額換算合計 (JPY)',
		'dashboard.active_one': 'アクティブ 1 件',
		'dashboard.active_other': 'アクティブ {count} 件',
		'dashboard.upcoming': 'このあと請求',
		'dashboard.upcoming_empty': '予定された請求はありません。',
		'dashboard.export_ics': 'カレンダー (.ics) 出力',
		'dashboard.unconvertible_one': '1 件のサブスクにレートが無いため集計に含まれていません。',
		'dashboard.unconvertible_other': '{count} 件のサブスクにレートが無いため集計に含まれていません。',

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
		'form.status': 'ステータス',
		'form.plan': 'プラン',
		'form.next_billing_date': '次回請求日',
		'form.started_at': '開始日',
		'form.notes': 'メモ',
		'form.submit': '登録',
		'form.update': '更新',
		'form.edit_heading': 'サブスクを編集',

		'sub_status.active': 'アクティブ',
		'sub_status.trial': 'トライアル',
		'sub_status.paused': '一時停止',
		'sub_status.cancelled': 'キャンセル',

		'common.edit': '編集',

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

		'settings.gmail_heading': 'Gmail 連携',
		'settings.gmail_description':
			'kinketsu はあなた自身の Google Cloud OAuth クライアントを使ってメールを読み取ります。console.cloud.google.com/apis/credentials で「デスクトップアプリ」用 OAuth クライアントを作成し、Client ID と Secret をここに貼り付けてください。',
		'settings.gmail_client_id': 'Client ID',
		'settings.gmail_client_secret': 'Client Secret',
		'settings.gmail_status': '状態:',
		'settings.gmail_connected': '接続済み',
		'settings.gmail_not_connected': '未接続',
		'settings.gmail_disconnect': '切断',
		'settings.gmail_save_creds': '認証情報を保存',

		'settings.paypal_heading': 'PayPal 連携',
		'settings.paypal_description':
			'kinketsu はあなた自身の PayPal Developer OAuth クライアント (Log In with PayPal) を使います。developer.paypal.com/dashboard/applications で作成し、Client ID と Secret を貼り付けてください。スキャンはトークン更新のみ動作し、Transaction Search API の連携は次回ラウンドで対応予定です。',
		'settings.paypal_connected': '接続済み',
		'settings.paypal_not_connected': '未接続',

		'settings.notifications_heading': '更新通知',
		'settings.notifications_description':
			'kinketsu はアクティブなサブスクの次回請求が 7 日以内に迫るとシステム通知を送ります。バックグラウンドで 1 日 1 回チェックします。下のボタンで今すぐ実行できます。',
		'settings.notifications_check_now': '今すぐ確認',
		'settings.notifications_result_zero': '今後 7 日以内に更新予定のサブスクはありません。',
		'settings.notifications_result_one': '通知を 1 件送信しました。',
		'settings.notifications_result_other': '通知を {count} 件送信しました。',

		'settings.language_heading': '言語',
		'settings.language_description':
			'kinketsu はブラウザの言語設定から自動判定しますが、ここで手動で変更できます。',
		'settings.language_auto': '自動 (ブラウザ)',
		'settings.language_en': 'English',
		'settings.language_ja': '日本語',

		'range.last_12_months': '直近 12 ヶ月',
		'range.this_year': '今年',
		'range.last_year': '昨年',

		'common.show': '表示',
		'common.hide': '隠す',

		'settings.rates_heading': '為替レート',
		'settings.rates_description':
			'kinketsu は為替レートをキャッシュして、ダッシュボードの合計を通貨横断で表示します。',
		'settings.rates_last': '最終更新:',
		'settings.rates_never': '未取得',
		'settings.rates_refresh': 'レートを更新',
		'settings.rates_count': 'キャッシュ済み {count} 件',

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

		'inbox.heading': 'サブスク インボックス',
		'inbox.description':
			'kinketsu は接続したソースから検出したサブスクをここにキューします。「Confirm」で一覧に追加、「Reject」で却下します。',
		'inbox.sources_heading': 'ソース',
		'inbox.sources_description': '外部ソースを接続するとサブスクの自動検出が始まります。',
		'inbox.gmail_coming_soon': '準備中 — Google アカウント連携を実装中です。',
		'inbox.paypal_coming_soon': '準備中 — PayPal の定期支払い取得を実装中です。',
		'inbox.connect_gmail': 'Gmail に接続',
		'inbox.connect_paypal': 'PayPal に接続',
		'inbox.range_heading': 'スキャン範囲',
		'inbox.range_description': 'ソース同期を実行する際にスキャンする月を選択してください。',
		'inbox.pending_heading': 'レビュー待ち',
		'inbox.reviewed_heading': '最近の処理',
		'inbox.empty':
			'まだ何もありません。上のソースを接続するか、Scan ページから明細を貼り付けてください。',
		'inbox.confirm': '確定',
		'inbox.edit_confirm': '編集して確定',
		'inbox.reject': '却下',
		'inbox.scan_run': 'Gmail スキャン実行',
		'inbox.scan_running': 'スキャン中…',
		'inbox.scan_complete_zero': 'スキャン完了 — 新しい検出はありませんでした。',
		'inbox.scan_complete_one': 'スキャン完了 — 1 件の新規検出。',
		'inbox.scan_complete_other': 'スキャン完了 — {count} 件の新規検出。',
		'inbox.scan_needs_range': '少なくとも 1 ヶ月を選択してください。',
		'inbox.scan_needs_creds': 'まず Settings で Gmail 認証情報を保存してください。',
		'inbox.scan_needs_connection': 'まず Gmail に接続してください。',
		'inbox.scan_needs_llm': 'まず Settings で LLM プロバイダーを設定してください。',
		'inbox.connect_gmail_loading': 'ブラウザを開いています…',
		'inbox.gmail_connected': 'Gmail 接続済み',
		'inbox.gmail_disconnect': '切断',

		'source.gmail': 'Gmail',
		'source.paypal': 'PayPal',
		'source.csv_import': 'CSV 取込',
		'source.manual': '手動',

		'status.pending': '未処理',
		'status.confirmed': '確定済み',
		'status.rejected': '却下',
		'status.duplicate': '重複',

		'range.all': '全選択',
		'range.none': '解除',
		'range.selected_one': '1 ヶ月選択中',
		'range.selected_other': '{count} ヶ月選択中',
		'range.clear_all': 'すべて解除',

		'common.delete': '削除',
		'common.error': 'エラー',
		'common.loading': '読み込み中…'
	}
};

const LOCALE_STORAGE_KEY = 'kinketsu.locale';

class I18n {
	locale = $state<LocaleCode>('en');

	init() {
		if (typeof window !== 'undefined') {
			const stored = window.localStorage.getItem(LOCALE_STORAGE_KEY);
			if (stored === 'en' || stored === 'ja') {
				this.locale = stored;
				return;
			}
		}
		if (typeof navigator !== 'undefined') {
			const tag = navigator.language?.toLowerCase() ?? 'en';
			this.locale = tag.startsWith('ja') ? 'ja' : 'en';
		}
	}

	setLocale(code: LocaleCode) {
		this.locale = code;
		if (typeof window !== 'undefined') {
			window.localStorage.setItem(LOCALE_STORAGE_KEY, code);
		}
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
