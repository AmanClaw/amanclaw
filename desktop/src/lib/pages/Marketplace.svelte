<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	// --- State ---
	let tab = $state<'browse' | 'installed' | 'publish'>('installed');
	let installed: any[] = $state([]);
	let loading = $state(true);
	let search = $state('');
	let confirmUninstall = $state<string | null>(null);
	let uninstalling = $state<string | null>(null);
	let showInstallForm = $state(false);
	let installPath = $state('');
	let installing = $state(false);

	const filteredInstalled = $derived(() => {
		if (!search.trim()) return installed;
		const q = search.toLowerCase();
		return installed.filter((p: any) =>
			(p.name || '').toLowerCase().includes(q) ||
			(p.description || '').toLowerCase().includes(q) ||
			(p.type || '').toLowerCase().includes(q)
		);
	});

	// --- Actions ---
	async function loadInstalled() {
		try {
			const data = await api.registryListInstalled() as any;
			installed = data.plugins || data || [];
		} catch (_) {}
		loading = false;
	}

	async function uninstallPlugin(name: string) {
		uninstalling = name;
		try {
			await api.registryUninstall(name);
			confirmUninstall = null;
			await loadInstalled();
		} catch (_) {}
		uninstalling = null;
	}

	async function installFromPath() {
		if (!installPath.trim()) return;
		installing = true;
		try {
			await api.registryInstallFromPath(installPath.trim());
			installPath = '';
			showInstallForm = false;
			await loadInstalled();
		} catch (_) {}
		installing = false;
	}

	function formatDate(ts: string): string {
		if (!ts) return '-';
		try {
			return new Date(ts).toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' });
		} catch (_) { return ts; }
	}

	function typeBadge(type: string): string {
		switch (type) {
			case 'wasm': return 'bg-purple-100 text-purple-700';
			case 'script': return 'bg-yellow-100 text-yellow-700';
			case 'builtin': return 'bg-gray-100 text-gray-700';
			default: return 'bg-gray-100 text-gray-500';
		}
	}

	onMount(() => { loadInstalled(); });
</script>

<div class="p-8 max-w-4xl">
	<!-- Header -->
	<div class="flex items-center justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Marketplace</h2>
			<p class="text-sm text-gray-500 mt-1">Browse, install, and manage skill packages</p>
		</div>
		{#if tab === 'installed'}
			<input type="text" bind:value={search} placeholder="Search installed..."
				class="px-3 py-1.5 text-xs border border-gray-200 rounded-md w-48 focus:outline-none focus:ring-2 focus:ring-gray-900">
		{/if}
	</div>

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-gray-100 rounded-lg p-0.5 w-fit">
		<button onclick={() => tab = 'browse'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'browse' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'}">
			Browse
		</button>
		<button onclick={() => tab = 'installed'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'installed' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'}">
			Installed
		</button>
		<button onclick={() => tab = 'publish'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'publish' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'}">
			Publish
		</button>
	</div>

	<!-- ===================== BROWSE TAB ===================== -->
	{#if tab === 'browse'}
		<div class="text-center py-20 bg-gray-50 rounded-xl border border-gray-200">
			<div class="text-gray-300 text-4xl mb-3">&#9741;</div>
			<p class="text-sm text-gray-500 font-medium">No remote registry configured</p>
			<p class="text-xs text-gray-400 mt-1">A public skill registry will be available in a future release.</p>
			<p class="text-xs text-gray-400 mt-0.5">For now, install skills from local folders in the Installed tab.</p>
		</div>

	<!-- ===================== INSTALLED TAB ===================== -->
	{:else if tab === 'installed'}
		<!-- Install from folder -->
		<div class="mb-4 flex items-center gap-2">
			{#if showInstallForm}
				<input type="text" bind:value={installPath} placeholder="/path/to/skill/folder"
					class="flex-1 px-3 py-1.5 text-xs font-mono border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
				<button onclick={installFromPath} disabled={installing || !installPath.trim()}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
					{installing ? 'Installing...' : 'Install'}
				</button>
				<button onclick={() => { showInstallForm = false; installPath = ''; }}
					class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-600 hover:bg-gray-50 transition-colors">
					Cancel
				</button>
			{:else}
				<button onclick={() => showInstallForm = true}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					Install from Folder
				</button>
			{/if}
		</div>

		{#if loading}
			<p class="text-sm text-gray-500">Loading...</p>
		{:else if filteredInstalled().length === 0}
			<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">
					{search.trim() ? 'No plugins match your search' : 'No plugins installed'}
				</p>
				{#if !search.trim()}
					<p class="text-xs text-gray-400 mt-1">Install skill packages from local folders to get started</p>
				{/if}
			</div>
		{:else}
			<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
				<table class="w-full text-xs">
					<thead>
						<tr class="border-b border-gray-100 bg-gray-50">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Name</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Version</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Type</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Description</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Installed</th>
							<th class="text-right px-4 py-2.5 text-[10px] font-medium text-gray-400 uppercase tracking-wider">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each filteredInstalled() as plugin}
							<tr class="border-b border-gray-50 hover:bg-gray-50 transition-colors">
								<td class="px-4 py-2.5 text-gray-900 font-medium">{plugin.name}</td>
								<td class="px-4 py-2.5 font-mono text-gray-500">{plugin.version || '-'}</td>
								<td class="px-4 py-2.5">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {typeBadge(plugin.type)}">
										{plugin.type || 'unknown'}
									</span>
								</td>
								<td class="px-4 py-2.5 text-gray-500 max-w-xs truncate">{plugin.description || '-'}</td>
								<td class="px-4 py-2.5 text-gray-400">{formatDate(plugin.installed_at)}</td>
								<td class="px-4 py-2.5 text-right">
									{#if confirmUninstall === plugin.name}
										<span class="text-[10px] text-gray-500 mr-2">Confirm?</span>
										<button onclick={() => uninstallPlugin(plugin.name)}
											disabled={uninstalling === plugin.name}
											class="px-2 py-0.5 text-[10px] font-medium rounded border border-red-300 text-red-600 hover:bg-red-50 mr-1 disabled:opacity-50">
											{uninstalling === plugin.name ? '...' : 'Yes'}
										</button>
										<button onclick={() => confirmUninstall = null}
											class="px-2 py-0.5 text-[10px] font-medium rounded border border-gray-200 text-gray-500 hover:bg-gray-50">
											No
										</button>
									{:else}
										<button onclick={() => confirmUninstall = plugin.name}
											class="text-xs text-red-500 hover:text-red-700 font-medium">
											Uninstall
										</button>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}

	<!-- ===================== PUBLISH TAB ===================== -->
	{:else}
		<div class="bg-white rounded-xl border border-gray-200 p-6">
			<h3 class="text-sm font-medium text-gray-900 mb-2">Publishing Skills</h3>
			<p class="text-xs text-gray-500 mb-4">
				Publishing to a remote registry is not yet available. When the AmanClaw Marketplace launches,
				you will be able to share your skills with the community.
			</p>

			<div class="bg-gray-50 rounded-lg border border-gray-200 p-4">
				<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-3">Example Manifest: amanclaw-skill.toml</p>
				<pre class="text-xs font-mono text-gray-700 leading-relaxed">[skill]
name = "my-custom-skill"
version = "0.1.0"
description = "A short description of your skill"
author = "Your Name"
license = "MIT"
type = "wasm"  # wasm | script

[skill.metadata]
keywords = ["example", "demo"]
homepage = "https://github.com/your/repo"

[build]
entry = "src/lib.rs"       # or "main.py" / "index.js"
target = "wasm32-wasip1"   # for WASM skills</pre>
			</div>

			<p class="text-xs text-gray-400 mt-4">
				Place this file at the root of your skill project. Use "Install from Folder" in the Installed tab to test locally.
			</p>
		</div>
	{/if}
</div>
