<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

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
		if (topic.startsWith('message.')) return 'text-info';
		if (topic.startsWith('security.')) return 'text-error';
		if (topic.startsWith('engine.')) return 'text-success';
		return 'text-fg-secondary';
	}

	function topicBg(topic: string): string {
		if (topic.startsWith('message.')) return 'bg-[var(--color-info-15)] border-[var(--color-info-20)]';
		if (topic.startsWith('security.')) return 'bg-[var(--color-error-15)] border-[var(--color-error-20)]';
		if (topic.startsWith('engine.')) return 'bg-[var(--color-success-15)] border-[var(--color-success-20)]';
		return 'bg-surface border-border';
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
			await api.saveGatewayConfig({ enabled, heartbeatIntervalSecs, maxConnections, staleSessionTimeoutSecs });
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

<div class="max-w-4xl">
	<PageHeader title="Gateway" subtitle="WebSocket gateway configuration and live event monitor" />

	<!-- Configuration Panel -->
	{#if loading}
		<p class="text-sm text-fg-muted">Loading...</p>
	{:else}
		<div class="bg-surface rounded-xl border border-border p-5 mb-6">
			<h3 class="text-sm font-medium text-fg mb-4">Configuration</h3>

			<div class="space-y-4">
				<!-- Enabled toggle -->
				<div class="flex items-center justify-between">
					<label class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Enabled</label>
					<button onclick={() => enabled = !enabled}
						class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors
							{enabled ? 'bg-primary-500' : 'bg-border'}">
						<span class="inline-block h-3.5 w-3.5 rounded-full bg-surface transition-transform
							{enabled ? 'translate-x-4' : 'translate-x-0.5'}"></span>
					</button>
				</div>

				<div class="grid grid-cols-3 gap-4">
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Heartbeat Interval (s)</label>
						<input type="number" bind:value={heartbeatIntervalSecs} min="1"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Max Connections</label>
						<input type="number" bind:value={maxConnections} min="1"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Stale Session Timeout (s)</label>
						<input type="number" bind:value={staleSessionTimeoutSecs} min="1"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
				</div>
			</div>

			<div class="mt-5">
				<button onclick={saveConfig} disabled={saving}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
					{saving ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>

		<!-- Live Events Panel -->
		<div class="bg-surface rounded-xl border border-border p-5">
			<div class="flex items-center justify-between mb-4">
				<div class="flex items-center gap-3">
					<h3 class="text-sm font-medium text-fg">Live Events</h3>
					<span class="inline-flex items-center gap-1.5 text-[10px] font-medium
						{connected ? 'text-success' : 'text-fg-muted'}">
						<span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-success animate-pulse' : 'bg-border'}"></span>
						{connected ? 'Connected' : 'Disconnected'}
					</span>
				</div>
				<div class="flex items-center gap-2">
					<input type="text" bind:value={topicFilter} placeholder="Filter topics..."
						class="px-2 py-1 text-xs border border-border rounded-md w-36 focus:outline-none focus:ring-2 focus:ring-primary-500">
					{#if !connected}
						<button onclick={connectWs}
							class="px-3 py-1 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors">
							Connect
						</button>
					{:else}
						<button onclick={() => paused = !paused}
							class="px-3 py-1 text-xs font-medium rounded-md border transition-colors
								{paused ? 'border-[var(--color-warning-20)] text-warning bg-[var(--color-warning-15)]' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
							{paused ? 'Resume' : 'Pause'}
						</button>
						<button onclick={clearEvents}
							class="px-3 py-1 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
							Clear
						</button>
						<button onclick={disconnectWs}
							class="px-3 py-1 text-xs font-medium rounded-md border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)] transition-colors">
							Disconnect
						</button>
					{/if}
				</div>
			</div>

			{#if filteredEvents().length === 0}
				<div class="text-center py-12 bg-surface rounded-lg border border-border">
					<p class="text-sm text-fg-muted">
						{#if !connected}
							Connect to the WebSocket to see live events
						{:else}
							Waiting for events...
						{/if}
					</p>
					{#if !connected}
						<p class="text-xs text-fg-muted mt-1">ws://127.0.0.1:3000/ws</p>
					{/if}
				</div>
			{:else}
				<div class="space-y-1 max-h-[480px] overflow-y-auto font-mono text-xs">
					{#each filteredEvents() as event, i}
						<div class="rounded-md border px-3 py-1.5 {topicBg(event.topic)}">
							<button onclick={() => toggleExpand(i)} class="w-full flex items-center gap-3 text-left">
								<span class="text-fg-muted shrink-0">{event.ts}</span>
								<span class="font-medium {topicColor(event.topic)} shrink-0">{event.topic}</span>
								{#if !event.expanded}
									<span class="text-fg-muted truncate">{JSON.stringify(event.payload).slice(0, 80)}</span>
								{/if}
								<span class="text-fg-muted ml-auto shrink-0 transition-transform {event.expanded ? 'rotate-90' : ''}">&#9656;</span>
							</button>
							{#if event.expanded}
								<pre class="mt-2 p-2 bg-surface rounded text-[11px] text-fg-secondary overflow-x-auto border border-border">{JSON.stringify(event.payload, null, 2)}</pre>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			{#if events.length > 0}
				<p class="text-[10px] text-fg-muted mt-3 text-right">{events.length} event{events.length !== 1 ? 's' : ''} (max {MAX_EVENTS})</p>
			{/if}
		</div>
	{/if}
</div>
