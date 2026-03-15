<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	// --- Config state ---
	let configOpen = $state(true);
	let enabled = $state(false);
	let maxPerSession = $state(5);
	let maxGlobal = $state(20);
	let maxDepth = $state(3);
	let defaultTimeoutSecs = $state(60);
	let saving = $state(false);
	let loading = $state(true);

	// --- Sub-agents state ---
	let subagents: any[] = $state([]);
	let sessionFilter = $state('');
	let cancelling = $state<string | null>(null);

	const filteredAgents = $derived(() => {
		if (!sessionFilter.trim()) return subagents;
		const q = sessionFilter.toLowerCase();
		return subagents.filter((a: any) =>
			(a.parent_session || '').toLowerCase().includes(q) ||
			(a.id || '').toLowerCase().includes(q)
		);
	});

	const summary = $derived(() => {
		let running = 0, completed = 0, failed = 0, cancelled = 0;
		for (const a of subagents) {
			if (a.status === 'running') running++;
			else if (a.status === 'completed') completed++;
			else if (a.status === 'failed') failed++;
			else if (a.status === 'cancelled') cancelled++;
		}
		return { running, completed, failed, cancelled };
	});

	function statusBadge(status: string): string {
		switch (status) {
			case 'running': return 'bg-[var(--color-info-15)] text-info';
			case 'completed': return 'bg-[var(--color-success-15)] text-success';
			case 'failed': return 'bg-[var(--color-error-15)] text-error';
			case 'cancelled': return 'bg-elevated text-fg-muted';
			default: return 'bg-elevated text-fg-muted';
		}
	}

	function truncate(str: string, len: number): string {
		if (!str) return '';
		return str.length > len ? str.slice(0, len) + '...' : str;
	}

	// --- Actions ---
	async function loadConfig() {
		try {
			const data = await api.getSubagentConfig() as any;
			enabled = data.enabled ?? false;
			maxPerSession = data.max_per_session ?? 5;
			maxGlobal = data.max_global ?? 20;
			maxDepth = data.max_depth ?? 3;
			defaultTimeoutSecs = data.default_timeout_secs ?? 60;
		} catch (_) {}
		loading = false;
	}

	async function saveConfig() {
		saving = true;
		try {
			await api.saveSubagentConfig({ enabled, maxPerSession, maxGlobal, maxDepth, defaultTimeoutSecs });
		} catch (_) {}
		saving = false;
	}

	async function loadSubagents() {
		try {
			const data = await api.listSubagents(sessionFilter.trim() || undefined) as any;
			subagents = data.subagents || data || [];
		} catch (_) {}
	}

	async function cancelAgent(id: string) {
		cancelling = id;
		try {
			await api.cancelSubagent(id);
			await loadSubagents();
		} catch (_) {}
		cancelling = null;
	}

	async function cancelAll() {
		if (!sessionFilter.trim()) return;
		try {
			await api.cancelAllSubagents(sessionFilter.trim());
			await loadSubagents();
		} catch (_) {}
	}

	onMount(() => {
		loadConfig();
		loadSubagents();
		const interval = setInterval(loadSubagents, 5000);
		return () => clearInterval(interval);
	});
</script>

<div class="max-w-5xl">
	<PageHeader title="Sub-Agents" subtitle="Manage sub-agent spawning limits and monitor active instances" />

	<!-- Config Panel (Collapsible) -->
	<div class="bg-surface rounded-xl border border-border mb-6">
		<button onclick={() => configOpen = !configOpen}
			class="w-full flex items-center justify-between p-5 text-left hover:from-primary-400 hover:to-primary-600 transition-colors rounded-xl">
			<h3 class="text-sm font-medium text-fg">Configuration</h3>
			<span class="text-fg-muted text-xs transition-transform {configOpen ? 'rotate-90' : ''}">&#9656;</span>
		</button>

		{#if configOpen}
			<div class="px-5 pb-5 border-t border-border pt-4">
				{#if loading}
					<p class="text-sm text-fg-muted">Loading...</p>
				{:else}
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

						<div class="grid grid-cols-4 gap-4">
							<div>
								<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Max Per Session</label>
								<input type="number" bind:value={maxPerSession} min="1"
									class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
							</div>
							<div>
								<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Max Global</label>
								<input type="number" bind:value={maxGlobal} min="1"
									class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
							</div>
							<div>
								<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Max Depth</label>
								<input type="number" bind:value={maxDepth} min="1"
									class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
							</div>
							<div>
								<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Default Timeout (s)</label>
								<input type="number" bind:value={defaultTimeoutSecs} min="1"
									class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
							</div>
						</div>

						<div>
							<button onclick={saveConfig} disabled={saving}
								class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
								{saving ? 'Saving...' : 'Save'}
							</button>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Summary Bar -->
	<div class="flex items-center gap-4 mb-4">
		<div class="flex items-center gap-4 text-xs">
			<span class="text-info font-medium">{summary().running} running</span>
			<span class="text-fg-muted">|</span>
			<span class="text-success font-medium">{summary().completed} completed</span>
			<span class="text-fg-muted">|</span>
			<span class="text-error font-medium">{summary().failed} failed</span>
			{#if summary().cancelled > 0}
				<span class="text-fg-muted">|</span>
				<span class="text-fg-muted font-medium">{summary().cancelled} cancelled</span>
			{/if}
		</div>
		<div class="ml-auto flex items-center gap-2">
			<input type="text" bind:value={sessionFilter} placeholder="Filter by session..."
				class="px-2 py-1 text-xs border border-border rounded-md w-44 focus:outline-none focus:ring-2 focus:ring-primary-500">
			{#if sessionFilter.trim()}
				<button onclick={cancelAll}
					class="px-3 py-1 text-xs font-medium rounded-md border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)] transition-colors">
					Cancel All
				</button>
			{/if}
		</div>
	</div>

	<!-- Sub-agents Table -->
	{#if filteredAgents().length === 0}
		<div class="text-center py-16 bg-surface rounded-xl border border-border">
			<p class="text-sm text-fg-muted">No active sub-agents</p>
			<p class="text-xs text-fg-muted mt-1">Sub-agents will appear here when spawned during conversations</p>
		</div>
	{:else}
		<div class="bg-surface rounded-xl border border-border overflow-hidden">
			<table class="w-full text-xs">
				<thead>
					<tr class="border-b border-border bg-surface">
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">ID</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Agent Profile</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Prompt</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Parent Session</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Depth</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Status</th>
						<th class="text-right px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredAgents() as agent}
						<tr class="border-b border-border hover:from-primary-400 hover:to-primary-600 transition-colors">
							<td class="px-4 py-2.5 font-mono text-fg-secondary" title={agent.id}>{truncate(agent.id, 8)}</td>
							<td class="px-4 py-2.5 text-fg font-medium">{agent.agent_profile || '-'}</td>
							<td class="px-4 py-2.5 text-fg-muted" title={agent.prompt}>{truncate(agent.prompt || '', 40)}</td>
							<td class="px-4 py-2.5 font-mono text-fg-muted" title={agent.parent_session}>{truncate(agent.parent_session || '', 12)}</td>
							<td class="px-4 py-2.5 text-fg-secondary">{agent.depth ?? '-'}</td>
							<td class="px-4 py-2.5">
								<span class="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium rounded-full {statusBadge(agent.status)}">
									{#if agent.status === 'running'}
										<span class="w-1.5 h-1.5 rounded-full bg-[var(--color-info-15)]0 animate-pulse"></span>
									{/if}
									{agent.status}
								</span>
							</td>
							<td class="px-4 py-2.5 text-right">
								{#if agent.status === 'running'}
									<button onclick={() => cancelAgent(agent.id)}
										disabled={cancelling === agent.id}
										class="px-2 py-0.5 text-[10px] font-medium rounded border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)] transition-colors disabled:opacity-50">
										{cancelling === agent.id ? '...' : 'Cancel'}
									</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
