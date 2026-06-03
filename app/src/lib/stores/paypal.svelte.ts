import { invoke } from '@tauri-apps/api/core';
import type { OAuthCredentials } from '$lib/bindings';

class PaypalStore {
	credentials = $state<OAuthCredentials | null>(null);
	connected = $state(false);
	loading = $state(false);
	saving = $state(false);
	connecting = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			const [creds, connected] = await Promise.all([
				invoke<OAuthCredentials | null>('get_paypal_oauth_credentials'),
				invoke<boolean>('has_paypal_tokens')
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
			await invoke('save_paypal_oauth_credentials', { creds });
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
			await invoke('start_paypal_oauth');
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

	async disconnect() {
		this.error = null;
		try {
			await invoke('disconnect_paypal');
			this.connected = false;
		} catch (e) {
			this.error = String(e);
		}
	}
}

export const paypal = new PaypalStore();
