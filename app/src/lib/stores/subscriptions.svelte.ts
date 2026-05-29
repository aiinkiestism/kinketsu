import { invoke } from '@tauri-apps/api/core';
import type { NewSubscription, Subscription } from '$lib/types';

class SubscriptionsStore {
	items = $state<Subscription[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.items = await invoke<Subscription[]>('list_subscriptions');
		} catch (e) {
			this.error = String(e);
			this.items = [];
		} finally {
			this.loading = false;
		}
	}

	async create(input: NewSubscription) {
		try {
			await invoke<Subscription>('create_subscription', { input });
			await this.load();
		} catch (e) {
			this.error = String(e);
			throw e;
		}
	}

	async update(sub: Subscription) {
		try {
			await invoke('update_subscription', { sub });
			await this.load();
		} catch (e) {
			this.error = String(e);
			throw e;
		}
	}

	async remove(id: string) {
		try {
			await invoke('delete_subscription', { id });
			await this.load();
		} catch (e) {
			this.error = String(e);
		}
	}
}

export const subscriptions = new SubscriptionsStore();
