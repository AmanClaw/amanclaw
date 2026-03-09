<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	// --- Config state ---
	let enabled = $state(false);
	let heartbeatIntervalSecs = $state(30);
	let maxConnections = $state(100);
	let staleSessionTimeoutSecs = $state(300);
	let loading = $state(true);
	let saving = $state(false);

	// --- WebSocket / Events state ---
	let ws: WebSocket | null = $state(null);
	let connected = $state(false);
	let paused = $state(false);
	let topicFilter = $state('');
	let events: { ts: string; topic: string; payload: any; expanded: boolean }[] = $state([]);

	const MAX_EVENTS = 200;

	const filteredEvents = $derived(() => {
		if (!topicFilter.trim()) return events;
		const q = topicFilter.toLowerCase();
		return events.filter(e => e.topic.toLowerCase().includes(q));
	});

	function topicColor(topic: string): string {
		if (topic.startsWith('message.')) return 'text-blue-600';
		if (topic.startsWith('security.')) return 'text-red-600';
		if (topic.startsWith('engine.')) return 'text-green-600';
		return 'text-gray-600';
	}

	function topicBg(topic: string): string {
		if (topic.startsWith('message.')) return 'bg-blue-50 border-blue-200';
		if (topic.startsWith('security.')) return 'bg-red-50 border-red-200';
		if (topic.startsWith('engine.')) return 'bg-green-50 border-green-200';
		return 'bg-gray-50 border-gray-200';
	}

	// --- Config actions ---
	async function loadConfig() {
		try {
			const data = await api.getGatewayConfig() as any;
			enabled = data.enabled ?? false;
			heartbeatIntervalSecs = data.heartbeat_interval_secs ?? 30;
			maxConnections = data.max_connections ?? 100;
			staleSessionTimeoutSecs = data.stale_session_timeout_secs ?? 300;
		} catch (_) {}
		loading = false;
	}

	async function saveConfig() {
		saving = true;
		try {
			await api.saveGatewayConfig({ enabled, heartbeat_interval_secs: heartbeatIntervalSecs, max_connections: maxConnections, stale_session_timeout_secs: staleSessionTimeoutSecs });
		} catch (_) {}
		saving = false;
	}

	// --- WebSocket ---
	function connectWs() {
		if (ws) ws.close();
		try {
			ws = new WebSocket('ws://127.0.0.1:3000/ws');
			ws.onopen = () => { connected = true; };
			ws.onclose = () => { connected = false; ws = null; };
			ws.onerror = () => { connected = false; };
			ws.onmessage = (ev) => {
				if (paused) return;
				try {
					const data = JSON.parse(ev.data);
					const entry = {
						ts: new Date().toISOString().slice(11, 23),
						topic: data.topic || 'unknown',
						payload: data,
						expanded: false,
					};
					events = [...events.slice(-(MAX_EVENTS - 1)), entry];
				} catch (_) {
					events = [...events.slice(-(MAX_EVENTS - 1)), {
						ts: new Date().toISOString().slice(11, 23),
						topic: 'raw',
						payload: ev.data,
						expanded: false,
					}];
				}
			};
		} catch (_) {
			connected = false;
		}
	}

	function disconnectWs() {
		if (ws) ws.close();
		ws = null;
		connected = false;
	}

	function clearEvents() {
		events = [];
	}

	function toggleExpand(index: number) {
		events = events.map((e, i) => i === index ? { ...e, expanded: !e.expanded } : e);
	}

	onMount(() => {
		loadConfig();
		return () => { if (ws) ws.close(); };
	});
</script>

<div class="p-8 max-w-4xl">
	<!-- Header -->
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Gateway</h2>
		<p class="text-sm text-gray-500 mt-1">WebSocket gateway configuration and live event monitor</p>
	</div>

	<!-- Configuration Panel -->
	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else}
		<div class="bg-white rounded-xl border border-gray-200 p-5 mb-6">
			<h3 class="text-sm font-medium text-gray-900 mb-4">Configuration</h3>

			<div class="space-y-4">
				<!-- Enabled toggle -->
				<div class="flex items-center justify-between">
					<label class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Enabled</label>
					<button onclick={() => enabled = !enabled}
						class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors
							{enabled ? 'bg-gray-900' : 'bg-gray-300'}">
						<span class="inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform
							{enabled ? 'translate-x-4' : 'translate-x-0.5'}"></span>
					</button>
				</div>

				<div class="grid grid-cols-3 gap-4">
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Heartbeat Interval (s)</label>
						<input type="number" bind:value={heartbeatIntervalSecs} min="1"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Max Connections</label>
						<input type="number" bind:value={maxConnections} min="1"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Stale Session Timeout (s)</label>
						<input type="number" bind:value={staleSessionTimeoutSecs} min="1"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
				</div>
			</div>

			<div class="mt-5">
				<button onclick={saveConfig} disabled={saving}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
					{saving ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>

		<!-- Live Events Panel -->
		<div class="bg-white rounded-xl border border-gray-200 p-5">
			<div class="flex items-center justify-between mb-4">
				<div class="flex items-center gap-3">
					<h3 class="text-sm font-medium text-gray-900">Live Events</h3>
					<span class="inline-flex items-center gap-1.5 text-[10px] font-medium
						{connected ? 'text-green-600' : 'text-gray-400'}">
						<span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-green-500 animate-pulse' : 'bg-gray-300'}"></span>
						{connected ? 'Connected' : 'Disconnected'}
					</span>
				</div>
				<div class="flex items-center gap-2">
					<input type="text" bind:value={topicFilter} placeholder="Filter topics..."
						class="px-2 py-1 text-xs border border-gray-200 rounded-md w-36 focus:outline-none focus:ring-2 focus:ring-gray-900">
					{#if !connected}
						<button onclick={connectWs}
							class="px-3 py-1 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
							Connect
						</button>
					{:else}
						<button onclick={() => paused = !paused}
							class="px-3 py-1 text-xs font-medium rounded-md border transition-colors
								{paused ? 'border-yellow-300 text-yellow-700 bg-yellow-50' : 'border-gray-200 text-gray-600 hover:bg-gray-50'}">
							{paused ? 'Resume' : 'Pause'}
						</button>
						<button onclick={clearEvents}
							class="px-3 py-1 text-xs font-medium rounded-md border border-gray-200 text-gray-600 hover:bg-gray-50 transition-colors">
							Clear
						</button>
						<button onclick={disconnectWs}
							class="px-3 py-1 text-xs font-medium rounded-md border border-red-200 text-red-600 hover:bg-red-50 transition-colors">
							Disconnect
						</button>
					{/if}
				</div>
			</div>

			{#if filteredEvents().length === 0}
				<div class="text-center py-12 bg-gray-50 rounded-lg border border-gray-200">
					<p class="text-sm text-gray-500">
						{#if !connected}
							Connect to the WebSocket to see live events
						{:else}
							Waiting for events...
						{/if}
					</p>
					{#if !connected}
						<p class="text-xs text-gray-400 mt-1">ws://127.0.0.1:3000/ws</p>
					{/if}
				</div>
			{:else}
				<div class="space-y-1 max-h-[480px] overflow-y-auto font-mono text-xs">
					{#each filteredEvents() as event, i}
						<div class="rounded-md border px-3 py-1.5 {topicBg(event.topic)}">
							<button onclick={() => toggleExpand(i)} class="w-full flex items-center gap-3 text-left">
								<span class="text-gray-400 shrink-0">{event.ts}</span>
								<span class="font-medium {topicColor(event.topic)} shrink-0">{event.topic}</span>
								{#if !event.expanded}
									<span class="text-gray-400 truncate">{JSON.stringify(event.payload).slice(0, 80)}</span>
								{/if}
								<span class="text-gray-300 ml-auto shrink-0 transition-transform {event.expanded ? 'rotate-90' : ''}">&#9656;</span>
							</button>
							{#if event.expanded}
								<pre class="mt-2 p-2 bg-white rounded text-[11px] text-gray-700 overflow-x-auto border border-gray-100">{JSON.stringify(event.payload, null, 2)}</pre>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			{#if events.length > 0}
				<p class="text-[10px] text-gray-400 mt-3 text-right">{events.length} event{events.length !== 1 ? 's' : ''} (max {MAX_EVENTS})</p>
			{/if}
		</div>
	{/if}
</div>
