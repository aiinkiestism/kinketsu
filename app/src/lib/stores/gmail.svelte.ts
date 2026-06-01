import { invoke } from '@tauri-apps/api/core';
import type { OAuthCredentials, YearMonth } from '$lib/types';

class GmailStore {
	credentials = $state<OAuthCredentials | null>(null);
	connected = $state(false);
	loading = $state(false);
	saving = $state(false);
	connecting = $state(false);
	scanning = $state(false);
	error = $state<string | null>(null);
	lastScanResult = $state<number | null>(null);

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

	async runScan(range: YearMonth[]): Promise<number> {
		this.scanning = true;
		this.error = null;
		this.lastScanResult = null;
		try {
			const created = await invoke<number>('run_gmail_scan', { range });
			this.lastScanResult = created;
			return created;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			this.scanning = false;
		}
	}
}

export const gmail = new GmailStore();
