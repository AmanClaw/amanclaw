<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let servers: Record<string, any> = $state({});
	let loading = $state(true);
	let showForm = $state(false);

	// Form fields
	let name = $state('');
	let transport = $state<'stdio' | 'http'>('stdio');
	let command = $state('');
	let args = $state('');
	let url = $state('');
	let envPairs = $state<{ key: string; value: string }[]>([]);
	let editingName = $state<string | null>(null);
	let saving = $state(false);

	async function loadServers() {
		try {
			const data = await api.getMcpServers() as any;
			servers = data.servers || {};
		} catch (_) {}
		loading = false;
	}

	function resetForm() {
		name = '';
		transport = 'stdio';
		command = '';
		args = '';
		url = '';
		envPairs = [];
		editingName = null;
		showForm = false;
	}

	function editServer(serverName: string) {
		const s = servers[serverName];
		editingName = serverName;
		name = serverName;
		transport = s.transport || (s.url ? 'http' : 'stdio');
		command = s.command || '';
		args = (s.args || []).join(' ');
		url = s.url || '';
		envPairs = Object.entries(s.env || {}).map(([key, value]) => ({ key, value: value as string }));
		showForm = true;
	}

	function addEnvPair() {
		envPairs = [...envPairs, { key: '', value: '' }];
	}

	function removeEnvPair(index: number) {
		envPairs = envPairs.filter((_, i) => i !== index);
	}

	async function saveServer() {
		if (!name.trim()) return;
		saving = true;
		try {
			const env: Record<string, string> = {};
			for (const pair of envPairs) {
				if (pair.key.trim()) env[pair.key.trim()] = pair.value;
			}

			if (editingName && editingName !== name) {
				await api.deleteMcpServer(editingName);
			}

			await api.saveMcpServer({
				name: name.trim(),
				command: transport === 'stdio' ? command.trim() || undefined : undefined,
				args: transport === 'stdio' && args.trim() ? args.trim().split(/\s+/) : undefined,
				env: Object.keys(env).length > 0 ? env : undefined,
				url: transport === 'http' ? url.trim() || undefined : undefined,
			});
			resetForm();
			await loadServers();
		} catch (_) {}
		saving = false;
	}

	async function deleteServer(serverName: string) {
		try {
			await api.deleteMcpServer(serverName);
			await loadServers();
		} catch (_) {}
	}

	onMount(() => { loadServers(); });
</script>

<div class="p-8 max-w-4xl">
	<div class="flex items-center justify-between mb-8">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">MCP Servers</h2>
			<p class="text-sm text-gray-500 mt-1">Connect external tool servers via Model Context Protocol</p>
		</div>
		{#if !showForm}
			<button onclick={() => { resetForm(); showForm = true; }}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
				Add Server
			</button>
		{/if}
	</div>

	{#if showForm}
		<div class="bg-white rounded-xl border border-gray-200 p-5 mb-6">
			<h3 class="text-sm font-medium text-gray-900 mb-4">{editingName ? 'Edit' : 'Add'} MCP Server</h3>

			<div class="space-y-4">
				<div>
					<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Server Name</label>
					<input type="text" bind:value={name} placeholder="e.g. filesystem, github"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<div>
					<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-2">Transport</label>
					<div class="flex gap-2">
						<button onclick={() => transport = 'stdio'}
							class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
								{transport === 'stdio' ? 'bg-gray-900 text-white border-gray-900' : 'border-gray-200 text-gray-600 hover:bg-gray-50'}">
							Stdio (Local)
						</button>
						<button onclick={() => transport = 'http'}
							class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
								{transport === 'http' ? 'bg-gray-900 text-white border-gray-900' : 'border-gray-200 text-gray-600 hover:bg-gray-50'}">
							HTTP (Remote)
						</button>
					</div>
				</div>

				{#if transport === 'stdio'}
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Command</label>
						<input type="text" bind:value={command} placeholder="e.g. npx, uvx, node"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Arguments</label>
						<input type="text" bind:value={args} placeholder="e.g. -y @modelcontextprotocol/server-filesystem /home/user/docs"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
					<div>
						<div class="flex items-center justify-between mb-1">
							<label class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Environment Variables</label>
							<button onclick={addEnvPair} class="text-xs text-gray-500 hover:text-gray-900">+ Add</button>
						</div>
						{#each envPairs as pair, i}
							<div class="flex gap-2 mb-2">
								<input type="text" bind:value={pair.key} placeholder="KEY"
									class="w-1/3 px-2 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900 font-mono">
								<input type="text" bind:value={pair.value} placeholder="value"
									class="flex-1 px-2 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900 font-mono">
								<button onclick={() => removeEnvPair(i)} class="text-xs text-red-500 hover:text-red-700 px-1">x</button>
							</div>
						{/each}
					</div>
				{:else}
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Server URL</label>
						<input type="text" bind:value={url} placeholder="e.g. http://localhost:8080/sse"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
				{/if}
			</div>

			<div class="flex gap-2 mt-5">
				<button onclick={saveServer} disabled={saving || !name.trim()}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
					{saving ? 'Saving...' : editingName ? 'Update' : 'Save'}
				</button>
				<button onclick={resetForm}
					class="px-4 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-600 hover:bg-gray-50 transition-colors">
					Cancel
				</button>
			</div>

			<p class="text-[11px] text-gray-400 mt-3">Restart the engine after adding/removing servers for changes to take effect.</p>
		</div>
	{/if}

	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else if Object.keys(servers).length === 0 && !showForm}
		<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
			<p class="text-sm text-gray-500 mb-1">No MCP servers configured</p>
			<p class="text-xs text-gray-400">Add external tool servers to extend your bot's capabilities</p>
		</div>
	{:else}
		<div class="space-y-3">
			{#each Object.entries(servers) as [serverName, server]}
				<div class="bg-white rounded-xl border border-gray-200 p-4">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-3">
							<span class="inline-flex px-2 py-0.5 text-[10px] font-medium rounded-full
								{server.transport === 'http' ? 'bg-blue-100 text-blue-700' : 'bg-purple-100 text-purple-700'}">
								{server.transport === 'http' ? 'HTTP' : 'STDIO'}
							</span>
							<div>
								<p class="text-sm font-medium text-gray-900">{serverName}</p>
								<p class="text-xs text-gray-500 font-mono mt-0.5">
									{#if server.transport === 'http'}
										{server.url}
									{:else}
										{server.command} {(server.args || []).join(' ')}
									{/if}
								</p>
							</div>
						</div>
						<div class="flex gap-2">
							<button onclick={() => editServer(serverName)}
								class="text-xs text-gray-500 hover:text-gray-900 font-medium">Edit</button>
							<button onclick={() => deleteServer(serverName)}
								class="text-xs text-red-500 hover:text-red-700 font-medium">Remove</button>
						</div>
					</div>
					{#if server.env && Object.keys(server.env).length > 0}
						<div class="mt-2 pt-2 border-t border-gray-100">
							<p class="text-[10px] text-gray-400 uppercase tracking-wider mb-1">Env</p>
							<div class="flex flex-wrap gap-1">
								{#each Object.keys(server.env) as key}
									<span class="px-1.5 py-0.5 bg-gray-100 rounded text-[10px] font-mono text-gray-600">{key}</span>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
