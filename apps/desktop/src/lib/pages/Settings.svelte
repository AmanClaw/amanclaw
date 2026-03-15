<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { currentPage } from '$lib/stores/app';

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

	// Feature summaries
	let agentCount = $state(0);
	let routingRuleCount = $state(0);
	let defaultAgent = $state('default');
	let cronJobCount = $state(0);
	let cronTimezone = $state('');
	let webhookCount = $state(0);
	let gatewayEnabled = $state(false);
	let subagentsEnabled = $state(false);
	let installedSkillCount = $state(0);
	let embeddingConfigured = $state(false);
	let embeddingModel = $state('');
	let kbCount = $state(0);

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

			// Load feature summaries (fire-and-forget)
			loadFeatureSummaries();
		} catch (_) {}
	});

	async function loadFeatureSummaries() {
		try {
			const agents = await api.listAgents() as any;
			agentCount = agents.count || 0;
		} catch (_) {}
		try {
			const routing = await api.getRoutingRules() as any;
			routingRuleCount = routing.rules?.length || 0;
			defaultAgent = routing.default_agent || 'default';
		} catch (_) {}
		try {
			const cron = await api.listCronJobs() as any;
			cronJobCount = cron.count || 0;
			cronTimezone = cron.timezone || 'UTC';
		} catch (_) {}
		try {
			const wh = await api.listWebhookEndpoints() as any;
			webhookCount = wh.count || 0;
		} catch (_) {}
		try {
			const gw = await api.getGatewayConfig() as any;
			gatewayEnabled = gw.enabled || false;
		} catch (_) {}
		try {
			const sa = await api.getSubagentConfig() as any;
			subagentsEnabled = sa.enabled || false;
		} catch (_) {}
		try {
			const reg = await api.registryListInstalled() as any;
			installedSkillCount = reg.count || 0;
		} catch (_) {}
		try {
			const emb = await api.getEmbeddingConfig() as any;
			embeddingConfigured = emb.configured || false;
			embeddingModel = emb.model || '';
		} catch (_) {}
		try {
			const kb = await api.listKnowledgeBases() as any;
			kbCount = kb.count || 0;
		} catch (_) {}
	}

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
		} catch (_) {
		} finally {
			saving = false;
		}
	}

	function goTo(page: string) {
		currentPage.set(page);
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

	<!-- Feature Summaries -->
	<section class="mt-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-4">Feature Overview</h3>
		<div class="grid grid-cols-2 gap-3">
			<button onclick={() => goTo('agents')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Agent Routing</p>
					<p class="text-[10px] text-gray-500">Default: {defaultAgent} · {routingRuleCount} rule{routingRuleCount !== 1 ? 's' : ''}</p>
				</div>
				<span class="text-xs text-gray-400">{agentCount} agent{agentCount !== 1 ? 's' : ''}</span>
			</button>

			<button onclick={() => goTo('cron')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Cron Jobs</p>
					<p class="text-[10px] text-gray-500">Timezone: {cronTimezone || 'UTC'}</p>
				</div>
				<span class="text-xs text-gray-400">{cronJobCount} job{cronJobCount !== 1 ? 's' : ''}</span>
			</button>

			<button onclick={() => goTo('webhooks')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Webhooks</p>
					<p class="text-[10px] text-gray-500">Base path: /hooks</p>
				</div>
				<span class="text-xs text-gray-400">{webhookCount} endpoint{webhookCount !== 1 ? 's' : ''}</span>
			</button>

			<button onclick={() => goTo('gateway')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Gateway</p>
					<p class="text-[10px] text-gray-500">WebSocket event stream</p>
				</div>
				<span class="text-[10px] font-medium px-1.5 py-0.5 rounded {gatewayEnabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'}">
					{gatewayEnabled ? 'Enabled' : 'Disabled'}
				</span>
			</button>

			<button onclick={() => goTo('subagents')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Sub-Agents</p>
					<p class="text-[10px] text-gray-500">Task delegation</p>
				</div>
				<span class="text-[10px] font-medium px-1.5 py-0.5 rounded {subagentsEnabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'}">
					{subagentsEnabled ? 'Enabled' : 'Disabled'}
				</span>
			</button>

			<button onclick={() => goTo('marketplace')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left">
				<div>
					<p class="text-xs font-medium text-gray-900">Marketplace</p>
					<p class="text-[10px] text-gray-500">Skill registry</p>
				</div>
				<span class="text-xs text-gray-400">{installedSkillCount} installed</span>
			</button>

			<button onclick={() => goTo('knowledgebases')}
				class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200 hover:border-gray-300 transition-colors text-left col-span-2">
				<div>
					<p class="text-xs font-medium text-gray-900">Knowledge Bases</p>
					<p class="text-[10px] text-gray-500">{embeddingConfigured ? `Embeddings: ${embeddingModel}` : 'Embeddings not configured'} · {kbCount} KB{kbCount !== 1 ? 's' : ''}</p>
				</div>
				<span class="text-[10px] font-medium px-1.5 py-0.5 rounded {embeddingConfigured ? 'bg-green-100 text-green-700' : 'bg-yellow-100 text-yellow-700'}">
					{embeddingConfigured ? 'Configured' : 'Not configured'}
				</span>
			</button>
		</div>
	</section>

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
