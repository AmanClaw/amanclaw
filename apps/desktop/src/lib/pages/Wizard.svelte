<script lang="ts">
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';
	import { currentPage, isFirstRun } from '$lib/stores/app';

	let step = $state(1);
	let baseUrl = $state('http://localhost:11434/v1');
	let model = $state('qwen3:8b');
	let apiKey = $state('');
	let error = $state('');
	let saving = $state(false);

	async function finish() {
		if (!baseUrl.trim() || !model.trim()) {
			error = 'Base URL and Model are required';
			return;
		}
		saving = true;
		error = '';
		try {
			await api.saveConfig({
				llmBaseUrl: baseUrl,
				llmModel: model,
				llmApiKey: apiKey,
			});
			try {
				await api.startEngine();
			} catch (_) {
				// Engine may already be running from auto-start — that's OK
			}
			isFirstRun.set(false);
			currentPage.set('dashboard');
		} catch (e: any) {
			error = String(e) || 'Failed to start';
			saving = false;
		}
	}
</script>

<div class="flex items-center justify-center h-full">
	<div class="w-full max-w-md p-8">
		<div class="text-center mb-8">
			<h1 class="text-2xl font-semibold text-fg">Welcome to AmanClaw</h1>
			<p class="text-sm text-fg-muted mt-2">Let's get your bot running in 2 steps</p>
		</div>

		{#if step === 1}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-fg">Step 1: LLM Configuration</h2>
				<p class="text-xs text-fg-muted">Connect to any OpenAI-compatible API (Ollama, vLLM, OpenAI, etc.)</p>

				<div>
					<label for="base-url" class="block text-xs font-medium text-fg-secondary mb-1">Base URL</label>
					<input id="base-url" type="text" bind:value={baseUrl}
						placeholder="http://localhost:11434/v1"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>

				<div>
					<label for="model-name" class="block text-xs font-medium text-fg-secondary mb-1">Model</label>
					<input id="model-name" type="text" bind:value={model}
						placeholder="qwen3:8b"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>

				<div>
					<label for="api-key" class="block text-xs font-medium text-fg-secondary mb-1">API Key <span class="text-fg-muted">(optional for local)</span></label>
					<input id="api-key" type="password" bind:value={apiKey}
						placeholder="sk-..."
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>

				<button onclick={() => { step = 2 }}
					class="w-full mt-2 px-4 py-2.5 text-sm font-medium rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors">
					Next
				</button>
			</div>
		{:else}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-fg">Step 2: Ready to Launch</h2>

				<div class="bg-surface rounded-lg border border-border p-4 space-y-2">
					<div class="flex justify-between text-xs">
						<span class="text-fg-muted">LLM URL</span>
						<span class="text-fg font-mono">{baseUrl}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-fg-muted">Model</span>
						<span class="text-fg font-mono">{model}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-fg-muted">API Key</span>
						<span class="text-fg">{apiKey ? '********' : 'Not set'}</span>
					</div>
				</div>

				<p class="text-xs text-fg-muted">You can configure channels (Telegram, Discord, etc.) later in Settings.</p>

				{#if error}
					<p class="text-xs text-error bg-[var(--color-error-15)] p-2 rounded">{error}</p>
				{/if}

				<div class="flex gap-2">
					<button onclick={() => { step = 1 }}
						class="px-4 py-2.5 text-sm font-medium rounded-lg border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
						Back
					</button>
					<button onclick={finish} disabled={saving}
						class="flex-1 px-4 py-2.5 text-sm font-medium rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
						{saving ? 'Starting...' : 'Start AmanClaw'}
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
