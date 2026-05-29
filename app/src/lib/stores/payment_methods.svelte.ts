import { invoke } from '@tauri-apps/api/core';
import type { NewPaymentMethod, PaymentMethod } from '$lib/types';

class PaymentMethodsStore {
	items = $state<PaymentMethod[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.items = await invoke<PaymentMethod[]>('list_payment_methods');
		} catch (e) {
			this.error = String(e);
			this.items = [];
		} finally {
			this.loading = false;
		}
	}

	async create(input: NewPaymentMethod) {
		try {
			await invoke<PaymentMethod>('create_payment_method', { input });
			await this.load();
		} catch (e) {
			this.error = String(e);
			throw e;
		}
	}

	async remove(id: string) {
		try {
			await invoke('delete_payment_method', { id });
			await this.load();
		} catch (e) {
			this.error = String(e);
		}
	}
}

export const paymentMethods = new PaymentMethodsStore();
