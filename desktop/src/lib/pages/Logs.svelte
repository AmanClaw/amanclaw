<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

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

<div class="p-8 max-w-5xl">
	<div class="flex items-center justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Logs</h2>
			<p class="text-sm text-gray-500 mt-1">Live bot activity ({logs.length} entries)</p>
		</div>
		<input type="text" bind:value={filter} placeholder="Filter logs..."
			class="px-3 py-1.5 text-xs border border-gray-200 rounded-md w-48 focus:outline-none focus:ring-2 focus:ring-gray-900">
	</div>

	<div class="bg-gray-950 rounded-xl p-4 font-mono text-xs h-[calc(100vh-200px)] overflow-y-auto">
		{#each filteredLogs as log}
			<div class="py-0.5 flex gap-3">
				<span class="text-gray-600 shrink-0">{log.timestamp}</span>
				<span class="shrink-0 w-12 {
					log.level === 'ERROR' ? 'text-red-400' :
					log.level === 'WARN' ? 'text-yellow-400' :
					log.level === 'INFO' ? 'text-blue-400' :
					'text-gray-500'
				}">{log.level}</span>
				<span class="text-gray-500 shrink-0">{log.target}</span>
				<span class="text-gray-300">{log.message}</span>
			</div>
		{/each}
		{#if filteredLogs.length === 0}
			<p class="text-gray-600">No logs yet. Start the bot to see activity.</p>
		{/if}
	</div>
</div>
