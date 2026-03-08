<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let mode = $state('local');
	let remoteUrl = $state('');
	let remoteToken = $state('');

	// LLM
	let llmBaseUrl = $state('');
	let llmModel = $state('');
	let llmApiKey = $state('');
	let maxTokens = $state(4096);
	let temperature = $state(0.7);

	// Channels
	let telegramToken = $state('');
	let discordToken = $state('');
	let slackBotToken = $state('');
	let slackAppToken = $state('');

	// Engine
	let rateLimit = $state(20);

	let saved = $state(false);
	let saving = $state(false);
	let dataDir = $state('');

	onMount(async () => {
		try {
			const m = await api.getMode() as string;
			if (m.startsWith('remote:')) {
				mode = 'remote';
				remoteUrl = m.replace('remote:', '');
			}

			const cfg = await api.getConfig() as any;
			if (cfg) {
				llmBaseUrl = cfg.llm?.base_url || '';
				llmModel = cfg.llm?.model || '';
				llmApiKey = cfg.llm?.api_key || '';
				maxTokens = cfg.llm?.max_tokens || 4096;
				temperature = cfg.llm?.temperature || 0.7;
				rateLimit = cfg.rate_limit_per_minute || 20;
				telegramToken = cfg.channels?.telegram || '';
				discordToken = cfg.channels?.discord || '';
				slackBotToken = cfg.channels?.slack_bot || '';
				slackAppToken = cfg.channels?.slack_app || '';
			}

			dataDir = await api.getDataDir();
		} catch (e) {
			// Not connected
		}
	});

	async function saveAll() {
		saving = true;
		try {
			await api.setMode(mode, remoteUrl, remoteToken);
			await api.saveConfig({
				llmBaseUrl: llmBaseUrl,
				llmModel: llmModel,
				llmApiKey: llmApiKey,
				maxTokens: maxTokens,
				temperature: temperature,
				rateLimit: rateLimit,
				telegramToken: telegramToken || undefined,
				discordToken: discordToken || undefined,
				slackBotToken: slackBotToken || undefined,
				slackAppToken: slackAppToken || undefined,
			});
			saved = true;
			setTimeout(() => saved = false, 2000);
		} catch (e) {
			// Handle error
		} finally {
			saving = false;
		}
	}
</script>

<div class="p-8 max-w-2xl">
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Settings</h2>
		<p class="text-sm text-gray-500 mt-1">Configure your AmanClaw instance</p>
	</div>

	<!-- LLM Config -->
	<section class="mb-8">
		<h3 class="text-sm font-medium text-gray-900 mb-3">LLM Configuration</h3>
		<div class="space-y-3">
			<div>
				<label for="s-base-url" class="block text-xs font-medium text-gray-700 mb-1">Base URL</label>
				<input id="s-base-url" type="text" bind:value={llmBaseUrl} placeholder="http://localhost:11434/v1"
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-model" class="block text-xs font-medium text-gray-700 mb-1">Model</label>
					<input id="s-model" type="text" bind:value={llmModel} placeholder="qwen3:8b"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-api-key" class="block text-xs font-medium text-gray-700 mb-1">API Key</label>
					<input id="s-api-key" type="password" bind:value={llmApiKey} placeholder="Optional"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-max-tokens" class="block text-xs font-medium text-gray-700 mb-1">Max Tokens</label>
					<input id="s-max-tokens" type="number" bind:value={maxTokens}
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-temperature" class="block text-xs font-medium text-gray-700 mb-1">Temperature</label>
					<input id="s-temperature" type="number" step="0.1" min="0" max="2" bind:value={temperature}
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		</div>
	</section>

	<!-- Channels -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Channel Tokens</h3>
		<p class="text-xs text-gray-500 mb-3">Leave empty to disable a channel. Restart engine after changes.</p>
		<div class="space-y-3">
			<div>
				<label for="s-telegram" class="block text-xs font-medium text-gray-700 mb-1">Telegram Bot Token</label>
				<input id="s-telegram" type="password" bind:value={telegramToken} placeholder="123456:ABC-DEF..."
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div>
				<label for="s-discord" class="block text-xs font-medium text-gray-700 mb-1">Discord Bot Token</label>
				<input id="s-discord" type="password" bind:value={discordToken} placeholder="MTIz..."
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-slack-bot" class="block text-xs font-medium text-gray-700 mb-1">Slack Bot Token</label>
					<input id="s-slack-bot" type="password" bind:value={slackBotToken} placeholder="xoxb-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-slack-app" class="block text-xs font-medium text-gray-700 mb-1">Slack App Token</label>
					<input id="s-slack-app" type="password" bind:value={slackAppToken} placeholder="xapp-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		</div>
	</section>

	<!-- Engine -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Engine</h3>
		<div>
			<label for="s-rate-limit" class="block text-xs font-medium text-gray-700 mb-1">Rate Limit (per minute per user)</label>
			<input id="s-rate-limit" type="number" bind:value={rateLimit} min="1" max="100"
				class="w-40 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
		</div>
	</section>

	<!-- Connection Mode -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Connection Mode</h3>
		<div class="space-y-2">
			<label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
				{mode === 'local' ? 'border-gray-900 bg-gray-50' : ''}">
				<input type="radio" bind:group={mode} value="local" class="accent-gray-900">
				<div>
					<p class="text-sm font-medium text-gray-900">Local Mode</p>
					<p class="text-xs text-gray-500">Bot engine runs in this app</p>
				</div>
			</label>
			<label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
				{mode === 'remote' ? 'border-gray-900 bg-gray-50' : ''}">
				<input type="radio" bind:group={mode} value="remote" class="accent-gray-900">
				<div>
					<p class="text-sm font-medium text-gray-900">Remote Mode</p>
					<p class="text-xs text-gray-500">Connect to a remote AmanClaw server</p>
				</div>
			</label>
		</div>
		{#if mode === 'remote'}
			<div class="mt-4 space-y-3">
				<div>
					<label for="s-remote-url" class="block text-xs font-medium text-gray-700 mb-1">Server URL</label>
					<input id="s-remote-url" type="text" bind:value={remoteUrl} placeholder="https://your-server.com"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-remote-token" class="block text-xs font-medium text-gray-700 mb-1">API Token</label>
					<input id="s-remote-token" type="password" bind:value={remoteToken} placeholder="Bearer token"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		{/if}
	</section>

	<!-- Save -->
	<div class="flex items-center gap-3">
		<button onclick={saveAll} disabled={saving}
			class="px-4 py-2 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
			{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Settings'}
		</button>
		<p class="text-xs text-gray-400">Restart engine after changing LLM or channel settings</p>
	</div>

	<!-- Data Dir -->
	<section class="mt-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-2">Data</h3>
		<p class="text-xs text-gray-500">Config, database, and plugins stored at:</p>
		<p class="text-xs text-gray-700 font-mono mt-1 bg-gray-50 p-2 rounded">{dataDir}</p>
	</section>

	<div class="border-t border-gray-200 pt-6 mt-6">
		<p class="text-xs text-gray-500">AmanClaw Desktop v0.1.0 · Built in Malaysia</p>
	</div>
</div>
