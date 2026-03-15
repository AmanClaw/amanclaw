<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	// --- State ---
	let tab = $state<'browse' | 'installed' | 'publish'>('browse');
	let installed: any[] = $state([]);
	let loading = $state(true);
	let search = $state('');
	let confirmUninstall = $state<string | null>(null);
	let uninstalling = $state<string | null>(null);
	let showInstallForm = $state(false);
	let installPath = $state('');
	let installing = $state(false);

	// Browse state
	let browseSkills: any[] = $state([]);
	let browsePacks: Record<string, string[]> = $state({});
	let browseLoading = $state(true);
	let browseSearch = $state('');
	let browseFilter = $state<'all' | 'official' | 'verified' | 'community'>('all');
	let selectedPack = $state<string | null>(null);
	let installedNames = $state<Set<string>>(new Set());

	const filteredBrowse = $derived(() => {
		let list = browseSkills;

		// Filter by tier
		if (browseFilter !== 'all') {
			list = list.filter((s: any) => s.tier === browseFilter);
		}

		// Filter by pack
		if (selectedPack && browsePacks[selectedPack]) {
			const packNames = new Set(browsePacks[selectedPack]);
			list = list.filter((s: any) => packNames.has(s.name));
		}

		// Filter by search
		if (browseSearch.trim()) {
			const q = browseSearch.toLowerCase();
			list = list.filter((s: any) =>
				(s.name || '').toLowerCase().includes(q) ||
				(s.description || '').toLowerCase().includes(q) ||
				(s.tags || []).some((t: string) => t.toLowerCase().includes(q)) ||
				(s.author || '').toLowerCase().includes(q)
			);
		}

		return list;
	});

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
	async function loadBrowse() {
		browseLoading = true;
		try {
			const data = await api.marketplaceBrowse() as any;
			browseSkills = data.skills || [];
			browsePacks = data.packs || {};
		} catch (_) {}
		browseLoading = false;
	}

	async function loadInstalled() {
		try {
			const data = await api.registryListInstalled() as any;
			installed = data.plugins || data.skills || data || [];
			installedNames = new Set(installed.map((p: any) => p.name));
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
			case 'wasm': return 'bg-[var(--color-accent-500-15)] text-[var(--color-accent-500)]';
			case 'script': return 'bg-[var(--color-warning-15)] text-warning';
			case 'builtin': return 'bg-elevated text-fg-secondary';
			default: return 'bg-elevated text-fg-muted';
		}
	}

	function tierBadge(tier: string): { label: string; icon: string; cls: string } {
		switch (tier) {
			case 'official': return { label: 'Official', icon: '⭐', cls: 'bg-[var(--color-warning-15)] text-warning' };
			case 'verified': return { label: 'Verified', icon: '✅', cls: 'bg-[var(--color-success-15)] text-success' };
			case 'community': return { label: 'Community', icon: '🌱', cls: 'bg-[var(--color-info-15)] text-info' };
			default: return { label: tier, icon: '', cls: 'bg-elevated text-fg-secondary' };
		}
	}

	function langBadge(lang: string): string {
		switch (lang) {
			case 'rust': return 'bg-[var(--color-accent-500-15)] text-accent-500';
			case 'python': return 'bg-[var(--color-info-15)] text-info';
			case 'javascript': case 'js': return 'bg-[var(--color-warning-15)] text-warning';
			default: return 'bg-elevated text-fg-muted';
		}
	}

	onMount(() => {
		loadBrowse();
		loadInstalled();
	});
</script>

<div class="max-w-4xl">
	<PageHeader title="Marketplace" subtitle="Browse, install, and manage skill packages">
		{#snippet action()}
			{#if tab === 'browse'}
				<input type="text" bind:value={browseSearch} placeholder="Search skills..."
					class="px-3 py-1.5 text-xs border border-border rounded-md w-56 bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
			{:else if tab === 'installed'}
				<input type="text" bind:value={search} placeholder="Search installed..."
					class="px-3 py-1.5 text-xs border border-border rounded-md w-48 bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
			{/if}
		{/snippet}
	</PageHeader>

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-elevated rounded-lg p-0.5 w-fit">
		<button onclick={() => tab = 'browse'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'browse' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Browse
		</button>
		<button onclick={() => tab = 'installed'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'installed' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Installed
		</button>
		<button onclick={() => tab = 'publish'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'publish' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Publish
		</button>
	</div>

	<!-- ===================== BROWSE TAB ===================== -->
	{#if tab === 'browse'}
		<!-- Filters row -->
		<div class="flex items-center gap-3 mb-4 flex-wrap">
			<!-- Tier filter pills -->
			<div class="flex gap-1 bg-elevated rounded-lg p-0.5">
				{#each [['all', 'All'], ['official', '⭐ Official'], ['verified', '✅ Verified'], ['community', '🌱 Community']] as [value, label]}
					<button onclick={() => browseFilter = value as any}
						class="px-3 py-1 text-[11px] font-medium rounded-md transition-colors
							{browseFilter === value ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
						{label}
					</button>
				{/each}
			</div>

			<!-- Pack filter dropdown -->
			<select bind:value={selectedPack}
				class="px-2.5 py-1 text-xs border border-border rounded-md bg-surface focus:outline-none focus:ring-2 focus:ring-primary-500">
				<option value={null}>All Packs</option>
				{#each Object.keys(browsePacks) as pack}
					<option value={pack}>{pack} ({browsePacks[pack].length})</option>
				{/each}
			</select>

			<span class="text-xs text-fg-muted ml-auto">{filteredBrowse().length} skill{filteredBrowse().length !== 1 ? 's' : ''}</span>
		</div>

		{#if browseLoading}
			<div class="text-center py-16">
				<p class="text-sm text-fg-muted">Loading skill index...</p>
			</div>
		{:else if filteredBrowse().length === 0}
			<div class="text-center py-16 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted">No skills match your filters</p>
				<button onclick={() => { browseSearch = ''; browseFilter = 'all'; selectedPack = null; }}
					class="mt-2 text-xs text-fg underline hover:no-underline">Clear filters</button>
			</div>
		{:else}
			<!-- Packs showcase (only when no filter is active) -->
			{#if !browseSearch.trim() && browseFilter === 'all' && !selectedPack && Object.keys(browsePacks).length > 0}
				<div class="mb-6">
					<h3 class="text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Skill Packs</h3>
					<div class="grid grid-cols-3 gap-3">
						{#each Object.entries(browsePacks) as [packName, packSkills]}
							<button onclick={() => selectedPack = packName}
								class="bg-surface rounded-xl border border-border p-4 text-left hover:border-border hover:shadow-sm transition-all">
								<p class="text-sm font-medium text-fg capitalize">{packName}</p>
								<p class="text-xs text-fg-muted mt-1">{packSkills.length} skill{packSkills.length !== 1 ? 's' : ''}</p>
								<div class="flex flex-wrap gap-1 mt-2">
									{#each packSkills.slice(0, 3) as name}
										<span class="inline-flex px-1.5 py-0.5 text-[10px] font-mono rounded bg-elevated text-fg-muted">{name}</span>
									{/each}
									{#if packSkills.length > 3}
										<span class="text-[10px] text-fg-muted">+{packSkills.length - 3} more</span>
									{/if}
								</div>
							</button>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Selected pack header -->
			{#if selectedPack}
				<div class="flex items-center gap-2 mb-4">
					<span class="text-xs font-medium text-fg-secondary">Pack: <span class="capitalize">{selectedPack}</span></span>
					<button onclick={() => selectedPack = null}
						class="text-xs text-fg-muted hover:text-fg-secondary">✕ Clear</button>
				</div>
			{/if}

			<!-- Skill cards grid -->
			<div class="grid grid-cols-1 gap-3">
				{#each filteredBrowse() as skill}
					{@const tier = tierBadge(skill.tier)}
					{@const isInstalled = installedNames.has(skill.name)}
					<div class="bg-surface rounded-xl border border-border p-4 hover:border-border hover:shadow-sm transition-all">
						<div class="flex items-start justify-between gap-4">
							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-2 flex-wrap">
									<h4 class="text-sm font-semibold text-fg">{skill.name}</h4>
									<span class="inline-flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] font-medium rounded {tier.cls}">
										{tier.icon} {tier.label}
									</span>
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {langBadge(skill.lang)}">
										{skill.lang}
									</span>
									<span class="text-[10px] text-fg-muted font-mono">v{skill.version}</span>
								</div>
								<p class="text-xs text-fg-muted mt-1.5">{skill.description}</p>
								<div class="flex items-center gap-3 mt-2">
									<span class="text-[10px] text-fg-muted">by {skill.author}</span>
									{#if skill.tags && skill.tags.length > 0}
										<div class="flex gap-1 flex-wrap">
											{#each skill.tags as tag}
												<span class="inline-flex px-1.5 py-0.5 text-[9px] rounded-full bg-elevated text-fg-muted">{tag}</span>
											{/each}
										</div>
									{/if}
								</div>
							</div>
							<div class="shrink-0 flex flex-col items-end gap-1.5">
								{#if isInstalled}
									<span class="px-3 py-1.5 text-[10px] font-medium rounded-md bg-[var(--color-success-15)] text-success border border-[var(--color-success-20)]">
										✓ Installed
									</span>
								{:else}
									<span class="px-3 py-1.5 text-[10px] font-medium rounded-md bg-elevated text-fg-muted">
										Available
									</span>
								{/if}
								{#if skill.repo}
									<a href={skill.repo.startsWith('http') ? skill.repo : `https://github.com/${skill.repo}`}
										class="text-[10px] text-fg-muted hover:text-fg-secondary underline"
										target="_blank" rel="noopener noreferrer">
										View Source
									</a>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}

	<!-- ===================== INSTALLED TAB ===================== -->
	{:else if tab === 'installed'}
		<!-- Install from folder -->
		<div class="mb-4 flex items-center gap-2">
			{#if showInstallForm}
				<input type="text" bind:value={installPath} placeholder="/path/to/skill/folder"
					class="flex-1 px-3 py-1.5 text-xs font-mono border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
				<button onclick={installFromPath} disabled={installing || !installPath.trim()}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
					{installing ? 'Installing...' : 'Install'}
				</button>
				<button onclick={() => { showInstallForm = false; installPath = ''; }}
					class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
					Cancel
				</button>
			{:else}
				<button onclick={() => showInstallForm = true}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors">
					Install from Folder
				</button>
			{/if}
		</div>

		{#if loading}
			<p class="text-sm text-fg-muted">Loading...</p>
		{:else if filteredInstalled().length === 0}
			<div class="text-center py-16 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted">
					{search.trim() ? 'No plugins match your search' : 'No plugins installed'}
				</p>
				{#if !search.trim()}
					<p class="text-xs text-fg-muted mt-1">Install skill packages from local folders to get started</p>
				{/if}
			</div>
		{:else}
			<div class="bg-surface rounded-xl border border-border overflow-hidden">
				<table class="w-full text-xs">
					<thead>
						<tr class="border-b border-border bg-surface">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Name</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Version</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Type</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Description</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Installed</th>
							<th class="text-right px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each filteredInstalled() as plugin}
							<tr class="border-b border-border hover:from-primary-400 hover:to-primary-600 transition-colors">
								<td class="px-4 py-2.5 text-fg font-medium">{plugin.name}</td>
								<td class="px-4 py-2.5 font-mono text-fg-muted">{plugin.version || '-'}</td>
								<td class="px-4 py-2.5">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {typeBadge(plugin.type)}">
										{plugin.type || 'unknown'}
									</span>
								</td>
								<td class="px-4 py-2.5 text-fg-muted max-w-xs truncate">{plugin.description || '-'}</td>
								<td class="px-4 py-2.5 text-fg-muted">{formatDate(plugin.installed_at)}</td>
								<td class="px-4 py-2.5 text-right">
									{#if confirmUninstall === plugin.name}
										<span class="text-[10px] text-fg-muted mr-2">Confirm?</span>
										<button onclick={() => uninstallPlugin(plugin.name)}
											disabled={uninstalling === plugin.name}
											class="px-2 py-0.5 text-[10px] font-medium rounded border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)] mr-1 disabled:opacity-50">
											{uninstalling === plugin.name ? '...' : 'Yes'}
										</button>
										<button onclick={() => confirmUninstall = null}
											class="px-2 py-0.5 text-[10px] font-medium rounded border border-border text-fg-muted hover:bg-[var(--color-elevated-50)]">
											No
										</button>
									{:else}
										<button onclick={() => confirmUninstall = plugin.name}
											class="text-xs text-error hover:text-error font-medium">
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
		<div class="bg-surface rounded-xl border border-border p-6">
			<h3 class="text-sm font-medium text-fg mb-2">Publishing Skills</h3>
			<p class="text-xs text-fg-muted mb-4">
				Publishing to a remote registry is not yet available. When the AmanClaw Marketplace launches,
				you will be able to share your skills with the community.
			</p>

			<div class="bg-surface rounded-lg border border-border p-4">
				<p class="text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-3">Example Manifest: amanclaw-skill.toml</p>
				<pre class="text-xs font-mono text-fg-secondary leading-relaxed">[skill]
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

			<p class="text-xs text-fg-muted mt-4">
				Place this file at the root of your skill project. Use "Install from Folder" in the Installed tab to test locally.
			</p>
		</div>
	{/if}
</div>
