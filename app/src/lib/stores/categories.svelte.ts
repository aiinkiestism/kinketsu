import { invoke } from '@tauri-apps/api/core';
import type { Category, NewCategory } from '$lib/types';

class CategoriesStore {
	items = $state<Category[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.items = await invoke<Category[]>('list_categories');
		} catch (e) {
			this.error = String(e);
			this.items = [];
		} finally {
			this.loading = false;
		}
	}

	async create(input: NewCategory) {
		try {
			await invoke<Category>('create_category', { input });
			await this.load();
		} catch (e) {
			this.error = String(e);
			throw e;
		}
	}

	async remove(id: string) {
		try {
			await invoke('delete_category', { id });
			await this.load();
		} catch (e) {
			this.error = String(e);
		}
	}
}

export const categories = new CategoriesStore();
