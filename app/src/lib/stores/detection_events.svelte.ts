import { invoke } from '@tauri-apps/api/core';
import type { DetectionEvent, NewSubscription, Subscription } from '$lib/bindings';

class DetectionEventsStore {
	items = $state<DetectionEvent[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.items = await invoke<DetectionEvent[]>('list_detection_events');
		} catch (e) {
			this.error = String(e);
			this.items = [];
		} finally {
			this.loading = false;
		}
	}

	async confirm(id: string): Promise<Subscription | null> {
		try {
			const sub = await invoke<Subscription>('confirm_detection_event', { id });
			await this.load();
			return sub;
		} catch (e) {
			this.error = String(e);
			return null;
		}
	}

	async confirmWithOverrides(id: string, sub: NewSubscription): Promise<Subscription | null> {
		try {
			const created = await invoke<Subscription>('confirm_detection_event_with_overrides', {
				id,
				sub
			});
			await this.load();
			return created;
		} catch (e) {
			this.error = String(e);
			return null;
		}
	}

	async reject(id: string) {
		try {
			await invoke('reject_detection_event', { id });
			await this.load();
		} catch (e) {
			this.error = String(e);
		}
	}

	async bulkReject(ids: string[]): Promise<number> {
		try {
			const n = await invoke<number>('bulk_reject_detection_events', { ids });
			await this.load();
			return n;
		} catch (e) {
			this.error = String(e);
			return 0;
		}
	}
}

export const detectionEvents = new DetectionEventsStore();
