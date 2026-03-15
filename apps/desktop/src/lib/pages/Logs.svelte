<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	let logs: any[] = $state([]);
	let filter = $state('');

	let filteredLogs = $derived(
		filter
			? logs.filter((l: any) => l.message.toLowerCase().includes(filter.toLowerCase())
				|| l.level.toLowerCase().includes(filter.toLowerCase())
				|| l.target.toLowerCase().includes(filter.toLowerCase()))
			: logs
	);

	async function refreshLogs() {
		try {
			const result = await api.getLogs();
			if (Array.isArray(result)) {
				logs = result;
			}
		} catch (_) {}
	}

	onMount(() => {
		refreshLogs();
		const interval = setInterval(refreshLogs, 2000);
		return () => clearInterval(interval);
	});
</script>

<div class="max-w-5xl">
	<PageHeader title="Logs" subtitle="Live bot activity ({logs.length} entries)">
		{#snippet action()}
			<input type="text" bind:value={filter} placeholder="Filter logs..."
				class="px-3 py-1.5 text-xs border border-border rounded-md w-48 bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
		{/snippet}
	</PageHeader>

	<div class="bg-base rounded-xl p-4 font-mono text-xs h-[calc(100vh-200px)] overflow-y-auto">
		{#each filteredLogs as log}
			<div class="py-0.5 flex gap-3">
				<span class="text-fg-secondary shrink-0">{log.timestamp}</span>
				<span class="shrink-0 w-12 {
					log.level === 'ERROR' ? 'text-error/70' :
					log.level === 'WARN' ? 'text-yellow-400' :
					log.level === 'INFO' ? 'text-info' :
					'text-fg-muted'
				}">{log.level}</span>
				<span class="text-fg-muted shrink-0">{log.target}</span>
				<span class="text-fg-muted">{log.message}</span>
			</div>
		{/each}
		{#if filteredLogs.length === 0}
			<p class="text-fg-secondary">No logs yet. Start the bot to see activity.</p>
		{/if}
	</div>
</div>
