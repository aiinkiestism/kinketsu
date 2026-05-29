import { invoke } from '@tauri-apps/api/core';
import type { DetectionEvent, Subscription } from '$lib/types';

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

	async reject(id: string) {
		try {
			await invoke('reject_detection_event', { id });
			await this.load();
		} catch (e) {
			this.error = String(e);
		}
	}
}

export const detectionEvents = new DetectionEventsStore();
