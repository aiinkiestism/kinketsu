import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
	OAuthCredentials,
	ScanEstimate,
	ScanOptsDto,
	ScanSummary,
	YearMonthDto
} from '$lib/bindings';

export type ScanPhase = 'extracting' | 'indexing';

export type ScanProgress = {
	processed: number;
	total: number;
	created: number;
	skippedClassified: number;
	skippedSeen: number;
	skippedBlocked?: number;
	phase?: ScanPhase;
};

class GmailStore {
	credentials = $state<OAuthCredentials | null>(null);
	connected = $state(false);
	loading = $state(false);
	saving = $state(false);
	connecting = $state(false);
	scanning = $state(false);
	error = $state<string | null>(null);
	lastScanResult = $state<number | null>(null);
	progress = $state<ScanProgress | null>(null);
	previewing = $state(false);
	estimate = $state<ScanEstimate | null>(null);
	summary = $state<ScanSummary | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			const [creds, connected] = await Promise.all([
				invoke<OAuthCredentials | null>('get_gmail_oauth_credentials'),
				invoke<boolean>('has_gmail_tokens')
			]);
			this.credentials = creds;
			this.connected = connected;
		} catch (e) {
			this.error = String(e);
		} finally {
			this.loading = false;
		}
	}

	async saveCredentials(creds: OAuthCredentials) {
		this.saving = true;
		this.error = null;
		try {
			await invoke('save_gmail_oauth_credentials', { creds });
			this.credentials = creds;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			this.saving = false;
		}
	}

	async connect() {
		this.connecting = true;
		this.error = null;
		try {
			await invoke('start_gmail_oauth');
			this.connected = true;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			this.connecting = false;
		}
	}

	async cancel() {
		try {
			await invoke('cancel_oauth');
		} catch (e) {
			this.error = String(e);
		}
	}

	async cancelScan() {
		try {
			await invoke('cancel_scan');
		} catch (e) {
			this.error = String(e);
		}
	}

	async disconnect() {
		this.error = null;
		try {
			await invoke('disconnect_gmail');
			this.connected = false;
		} catch (e) {
			this.error = String(e);
		}
	}

	async loadSummary() {
		try {
			this.summary = await invoke<ScanSummary | null>('get_last_scan_summary');
		} catch (e) {
			this.error = String(e);
		}
	}

	/** Dry run: estimate counts + cost without spending any LLM calls. */
	async preview(range: YearMonthDto[], deep: boolean, opts: ScanOptsDto): Promise<ScanEstimate> {
		this.previewing = true;
		this.error = null;
		this.progress = null;

		let unlisten: UnlistenFn | null = null;
		try {
			unlisten = await listen<ScanProgress>('scan-progress', (event) => {
				this.progress = event.payload;
			});
			const est = await invoke<ScanEstimate>('preview_gmail_scan', { range, deep, opts });
			this.estimate = est;
			return est;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			if (unlisten) unlisten();
			this.previewing = false;
			this.progress = null;
		}
	}

	async runScan(range: YearMonthDto[], opts: ScanOptsDto): Promise<number> {
		return this.runScanInner('run_gmail_scan', range, opts);
	}

	async runDeepScan(range: YearMonthDto[], opts: ScanOptsDto): Promise<number> {
		return this.runScanInner('run_gmail_deep_scan', range, opts);
	}

	private async runScanInner(
		command: string,
		range: YearMonthDto[],
		opts: ScanOptsDto
	): Promise<number> {
		this.scanning = true;
		this.error = null;
		this.lastScanResult = null;
		this.progress = null;

		let unlisten: UnlistenFn | null = null;
		try {
			unlisten = await listen<ScanProgress>('scan-progress', (event) => {
				this.progress = event.payload;
			});
			const created = await invoke<number>(command, { range, opts });
			this.lastScanResult = created;
			// Refresh the dashboard summary the backend just persisted.
			await this.loadSummary();
			return created;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			if (unlisten) unlisten();
			this.scanning = false;
			this.progress = null;
		}
	}
}

export const gmail = new GmailStore();
