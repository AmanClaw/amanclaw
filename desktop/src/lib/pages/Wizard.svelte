<script lang="ts">
	import { api } from '$lib/api';
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
				llm_base_url: baseUrl,
				llm_model: model,
				llm_api_key: apiKey,
			});
			await api.startEngine();
			isFirstRun.set(false);
			currentPage.set('dashboard');
		} catch (e: any) {
			error = e?.toString() || 'Failed to start';
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex items-center justify-center h-full">
	<div class="w-full max-w-md p-8">
		<div class="text-center mb-8">
			<h1 class="text-2xl font-semibold text-gray-900">Welcome to AmanClaw</h1>
			<p class="text-sm text-gray-500 mt-2">Let's get your bot running in 2 steps</p>
		</div>

		{#if step === 1}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-gray-900">Step 1: LLM Configuration</h2>
				<p class="text-xs text-gray-500">Connect to any OpenAI-compatible API (Ollama, vLLM, OpenAI, etc.)</p>

				<div>
					<label for="base-url" class="block text-xs font-medium text-gray-700 mb-1">Base URL</label>
					<input id="base-url" type="text" bind:value={baseUrl}
						placeholder="http://localhost:11434/v1"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<div>
					<label for="model-name" class="block text-xs font-medium text-gray-700 mb-1">Model</label>
					<input id="model-name" type="text" bind:value={model}
						placeholder="qwen3:8b"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<div>
					<label for="api-key" class="block text-xs font-medium text-gray-700 mb-1">API Key <span class="text-gray-400">(optional for local)</span></label>
					<input id="api-key" type="password" bind:value={apiKey}
						placeholder="sk-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<button onclick={() => { step = 2 }}
					class="w-full mt-2 px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					Next
				</button>
			</div>
		{:else}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-gray-900">Step 2: Ready to Launch</h2>

				<div class="bg-gray-50 rounded-lg border border-gray-200 p-4 space-y-2">
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">LLM URL</span>
						<span class="text-gray-900 font-mono">{baseUrl}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">Model</span>
						<span class="text-gray-900 font-mono">{model}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">API Key</span>
						<span class="text-gray-900">{apiKey ? '********' : 'Not set'}</span>
					</div>
				</div>

				<p class="text-xs text-gray-500">You can configure channels (Telegram, Discord, etc.) later in Settings.</p>

				{#if error}
					<p class="text-xs text-red-600 bg-red-50 p-2 rounded">{error}</p>
				{/if}

				<div class="flex gap-2">
					<button onclick={() => { step = 1 }}
						class="px-4 py-2.5 text-sm font-medium rounded-lg border border-gray-300 text-gray-700 hover:bg-gray-50 transition-colors">
						Back
					</button>
					<button onclick={finish} disabled={saving}
						class="flex-1 px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
						{saving ? 'Starting...' : 'Start AmanClaw'}
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
