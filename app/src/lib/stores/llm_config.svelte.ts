import { invoke } from '@tauri-apps/api/core';
import type { LlmConfig } from '$lib/types';

class LlmConfigStore {
	current = $state<LlmConfig | null>(null);
	loading = $state(false);
	saving = $state(false);
	error = $state<string | null>(null);

	async load() {
		this.loading = true;
		this.error = null;
		try {
			this.current = await invoke<LlmConfig | null>('get_llm_config');
		} catch (e) {
			this.error = String(e);
			this.current = null;
		} finally {
			this.loading = false;
		}
	}

	async save(config: LlmConfig) {
		this.saving = true;
		this.error = null;
		try {
			await invoke('set_llm_config', { config });
			this.current = config;
		} catch (e) {
			this.error = String(e);
			throw e;
		} finally {
			this.saving = false;
		}
	}
}

export const llmConfig = new LlmConfigStore();
