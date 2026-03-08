<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	// --- State ---
	let tab = $state<'installed' | 'marketplace'>('installed');
	let skills: any[] = $state([]);
	let installedServers: Record<string, any> = $state({});
	let loading = $state(true);
	let search = $state('');
	let expandedSkill = $state<string | null>(null);
	let installing = $state<string | null>(null);
	let showRestart = $state(false);
	let marketplaceCategory = $state('all');

	// --- Marketplace Catalog ---
	interface MarketplaceItem {
		name: string;
		description: string;
		category: string;
		author: string;
		command: string;
		args: string[];
		env?: Record<string, string>;
		url?: string;
		tags: string[];
	}

	const catalog: MarketplaceItem[] = [
		{
			name: 'filesystem',
			description: 'Read, write, and manage files on the local filesystem with directory listing and search.',
			category: 'Utilities',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-filesystem', '/tmp'],
			tags: ['files', 'read', 'write', 'directory'],
		},
		{
			name: 'brave-search',
			description: 'Web and local search using the Brave Search API.',
			category: 'Search',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-brave-search'],
			env: { BRAVE_API_KEY: '' },
			tags: ['web', 'search', 'internet'],
		},
		{
			name: 'github',
			description: 'Manage GitHub repositories, issues, pull requests, and more.',
			category: 'Developer Tools',
			author: 'GitHub',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-github'],
			env: { GITHUB_PERSONAL_ACCESS_TOKEN: '' },
			tags: ['git', 'code', 'repository', 'issues', 'pr'],
		},
		{
			name: 'postgres',
			description: 'Query and manage PostgreSQL databases with read-only access.',
			category: 'Databases',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-postgres', 'postgresql://localhost/mydb'],
			tags: ['database', 'sql', 'query'],
		},
		{
			name: 'sqlite',
			description: 'Query and analyze SQLite databases with business intelligence capabilities.',
			category: 'Databases',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-sqlite', '/path/to/db.sqlite'],
			tags: ['database', 'sql', 'analytics'],
		},
		{
			name: 'memory',
			description: 'Knowledge graph-based persistent memory for maintaining context across conversations.',
			category: 'Utilities',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-memory'],
			tags: ['memory', 'knowledge', 'graph', 'context'],
		},
		{
			name: 'puppeteer',
			description: 'Browser automation for web scraping, screenshots, and interaction.',
			category: 'Web',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-puppeteer'],
			tags: ['browser', 'scraping', 'automation', 'screenshot'],
		},
		{
			name: 'fetch',
			description: 'Fetch and convert web content to markdown for easy consumption.',
			category: 'Web',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-fetch'],
			tags: ['http', 'web', 'markdown', 'scrape'],
		},
		{
			name: 'slack',
			description: 'Interact with Slack workspaces — read channels, send messages, manage threads.',
			category: 'Communication',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-slack'],
			env: { SLACK_BOT_TOKEN: '', SLACK_TEAM_ID: '' },
			tags: ['chat', 'messaging', 'team'],
		},
		{
			name: 'google-maps',
			description: 'Location services, directions, and place search using Google Maps API.',
			category: 'Utilities',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-google-maps'],
			env: { GOOGLE_MAPS_API_KEY: '' },
			tags: ['maps', 'location', 'directions', 'places'],
		},
		{
			name: 'sequential-thinking',
			description: 'Dynamic problem-solving through structured sequential thinking and reflection.',
			category: 'Reasoning',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-sequential-thinking'],
			tags: ['reasoning', 'thinking', 'analysis'],
		},
		{
			name: 'everything',
			description: 'MCP test server with prompts, resources, and tools for testing and debugging.',
			category: 'Developer Tools',
			author: 'Anthropic',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-everything'],
			tags: ['testing', 'debug', 'development'],
		},
	];

	const categories = $derived(() => {
		const cats = new Set(catalog.map(c => c.category));
		return ['all', ...Array.from(cats).sort()];
	});

	const filteredCatalog = $derived(() => {
		let items = catalog;
		// Exclude already-installed
		items = items.filter(c => !installedServers[c.name]);
		if (marketplaceCategory !== 'all') {
			items = items.filter(c => c.category === marketplaceCategory);
		}
		if (search.trim()) {
			const q = search.toLowerCase();
			items = items.filter(c =>
				c.name.toLowerCase().includes(q) ||
				c.description.toLowerCase().includes(q) ||
				c.tags.some(t => t.includes(q))
			);
		}
		return items;
	});

	// --- Installed Skills ---
	const filteredSkills = $derived(() => {
		if (!search.trim()) return skills;
		const q = search.toLowerCase();
		return skills.filter((s: any) =>
			s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q)
		);
	});

	const groupedSkills = $derived(() => {
		const groups: Record<string, any[]> = {};
		for (const skill of filteredSkills()) {
			const parts = skill.name.split('__');
			const group = skill.source === 'mcp' && parts.length > 1 ? `MCP: ${parts[0]}` : 'Built-in';
			if (!groups[group]) groups[group] = [];
			groups[group].push(skill);
		}
		const sorted: [string, any[]][] = [];
		if (groups['Built-in']) sorted.push(['Built-in', groups['Built-in']]);
		for (const key of Object.keys(groups).sort()) {
			if (key !== 'Built-in') sorted.push([key, groups[key]]);
		}
		return sorted;
	});

	// --- Actions ---
	function toggleExpand(name: string) {
		expandedSkill = expandedSkill === name ? null : name;
	}

	function formatParams(schema: any): { name: string; type: string; description: string; required: boolean }[] {
		if (!schema || !schema.properties) return [];
		const required = schema.required || [];
		return Object.entries(schema.properties).map(([name, prop]: [string, any]) => ({
			name,
			type: prop.type || 'any',
			description: prop.description || '',
			required: required.includes(name),
		}));
	}

	function sourceLabel(source: string): string {
		if (source === 'mcp') return 'MCP';
		if (source === 'builtin') return 'Built-in';
		return source;
	}

	function sourceBadgeClass(source: string): string {
		if (source === 'mcp') return 'bg-blue-100 text-blue-700';
		return 'bg-gray-100 text-gray-700';
	}

	// Install state for env var editing
	let installTarget = $state<MarketplaceItem | null>(null);
	let installEnv = $state<{ key: string; value: string }[]>([]);
	let installArgs = $state<string>('');

	function startInstall(item: MarketplaceItem) {
		installTarget = item;
		installArgs = item.args.join(' ');
		installEnv = item.env
			? Object.entries(item.env).map(([key, value]) => ({ key, value }))
			: [];
	}

	function cancelInstall() {
		installTarget = null;
		installEnv = [];
		installArgs = '';
	}

	async function confirmInstall() {
		if (!installTarget) return;
		installing = installTarget.name;
		try {
			const env: Record<string, string> = {};
			for (const pair of installEnv) {
				if (pair.key.trim() && pair.value.trim()) env[pair.key.trim()] = pair.value;
			}
			await api.saveMcpServer({
				name: installTarget.name,
				command: installTarget.command,
				args: installArgs.trim() ? installArgs.trim().split(/\s+/) : installTarget.args,
				env: Object.keys(env).length > 0 ? env : undefined,
			});
			showRestart = true;
			await loadInstalledServers();
			cancelInstall();
		} catch (_) {}
		installing = null;
	}

	async function uninstallServer(name: string) {
		try {
			await api.deleteMcpServer(name);
			showRestart = true;
			await loadInstalledServers();
		} catch (_) {}
	}

	async function handleRestart() {
		try {
			await api.restartEngine();
			showRestart = false;
			setTimeout(loadSkills, 2000);
		} catch (_) {}
	}

	// --- Data Loading ---
	async function loadSkills() {
		try {
			const data = await api.getSkills() as any;
			skills = data.skills || [];
		} catch (_) {}
		loading = false;
	}

	async function loadInstalledServers() {
		try {
			const data = await api.getMcpServers() as any;
			installedServers = data.servers || {};
		} catch (_) {}
	}

	onMount(() => {
		loadSkills();
		loadInstalledServers();
		const interval = setInterval(loadSkills, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="p-8 max-w-4xl">
	<!-- Header -->
	<div class="flex items-center justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Skills</h2>
			<p class="text-sm text-gray-500 mt-1">
				{#if tab === 'installed'}
					{skills.length} active skill{skills.length !== 1 ? 's' : ''}
				{:else}
					Browse and install skill packages
				{/if}
			</p>
		</div>
		<input type="text" bind:value={search} placeholder="Search..."
			class="px-3 py-1.5 text-xs border border-gray-200 rounded-md w-48 focus:outline-none focus:ring-2 focus:ring-gray-900">
	</div>

	<!-- Restart banner -->
	{#if showRestart}
		<div class="flex items-center justify-between p-3 mb-4 bg-yellow-50 border border-yellow-200 rounded-lg">
			<p class="text-xs text-yellow-800">MCP servers changed. Restart engine to apply.</p>
			<button onclick={handleRestart}
				class="px-3 py-1 text-xs font-medium rounded-md bg-yellow-600 text-white hover:bg-yellow-700 transition-colors">
				Restart Now
			</button>
		</div>
	{/if}

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-gray-100 rounded-lg p-0.5 w-fit">
		<button onclick={() => tab = 'installed'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'installed' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'}">
			Installed
		</button>
		<button onclick={() => tab = 'marketplace'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'marketplace' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-500 hover:text-gray-700'}">
			Marketplace
		</button>
	</div>

	<!-- ===================== INSTALLED TAB ===================== -->
	{#if tab === 'installed'}
		{#if loading}
			<p class="text-sm text-gray-500">Loading...</p>
		{:else if skills.length === 0}
			<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">No skills registered</p>
				<p class="text-xs text-gray-400 mt-1">Start the engine or install skills from the Marketplace</p>
				<button onclick={() => tab = 'marketplace'}
					class="mt-3 px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					Browse Marketplace
				</button>
			</div>
		{:else if filteredSkills().length === 0}
			<div class="text-center py-12 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">No skills match your search</p>
			</div>
		{:else}
			<div class="space-y-6">
				{#each groupedSkills() as [group, groupSkills]}
					<div>
						<h3 class="text-[11px] font-medium text-gray-400 uppercase tracking-wider mb-2">{group} ({groupSkills.length})</h3>
						<div class="space-y-1">
							{#each groupSkills as skill}
								<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
									<button onclick={() => toggleExpand(skill.name)}
										class="w-full flex items-center justify-between p-3 hover:bg-gray-50 transition-colors text-left">
										<div class="flex items-center gap-3 min-w-0">
											<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {sourceBadgeClass(skill.source)}">
												{sourceLabel(skill.source)}
											</span>
											<div class="min-w-0">
												<p class="text-sm font-medium text-gray-900 truncate">{skill.name}</p>
												<p class="text-xs text-gray-500 truncate mt-0.5">{skill.description}</p>
											</div>
										</div>
										<div class="flex items-center gap-3 shrink-0 ml-3">
											{#if skill.timeout_ms}
												<span class="text-[10px] text-gray-400">{skill.timeout_ms / 1000}s</span>
											{/if}
											<span class="text-gray-400 text-xs transition-transform {expandedSkill === skill.name ? 'rotate-90' : ''}">&#9656;</span>
										</div>
									</button>

									{#if expandedSkill === skill.name}
										<div class="border-t border-gray-100 p-4 bg-gray-50/50">
											<div class="grid grid-cols-3 gap-4 text-xs mb-3">
												<div>
													<span class="text-gray-400 block">Version</span>
													<span class="text-gray-700 font-mono">{skill.version}</span>
												</div>
												<div>
													<span class="text-gray-400 block">Source</span>
													<span class="text-gray-700">{sourceLabel(skill.source)}</span>
												</div>
												<div>
													<span class="text-gray-400 block">Timeout</span>
													<span class="text-gray-700">{skill.timeout_ms ? `${skill.timeout_ms}ms` : 'default'}</span>
												</div>
											</div>

											{#if skill.parameters && skill.parameters.properties}
												<div>
													<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-2">Parameters</p>
													<div class="bg-white rounded-md border border-gray-200 overflow-hidden">
														<table class="w-full text-xs">
															<thead>
																<tr class="border-b border-gray-100">
																	<th class="text-left px-3 py-1.5 text-[10px] font-medium text-gray-400 uppercase">Name</th>
																	<th class="text-left px-3 py-1.5 text-[10px] font-medium text-gray-400 uppercase">Type</th>
																	<th class="text-left px-3 py-1.5 text-[10px] font-medium text-gray-400 uppercase">Description</th>
																</tr>
															</thead>
															<tbody>
																{#each formatParams(skill.parameters) as param}
																	<tr class="border-b border-gray-50">
																		<td class="px-3 py-1.5 font-mono text-gray-900">
																			{param.name}{#if param.required}<span class="text-red-400 ml-0.5">*</span>{/if}
																		</td>
																		<td class="px-3 py-1.5 text-gray-500">{param.type}</td>
																		<td class="px-3 py-1.5 text-gray-500">{param.description}</td>
																	</tr>
																{/each}
															</tbody>
														</table>
													</div>
												</div>
											{/if}

											<!-- Uninstall for MCP skills -->
											{#if skill.source === 'mcp'}
												{@const serverName = skill.name.split('__')[0]}
												{#if installedServers[serverName]}
													<div class="mt-3 pt-3 border-t border-gray-100">
														<button onclick={() => uninstallServer(serverName)}
															class="text-xs text-red-500 hover:text-red-700 font-medium">
															Uninstall {serverName}
														</button>
													</div>
												{/if}
											{/if}
										</div>
									{/if}
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}

	<!-- ===================== MARKETPLACE TAB ===================== -->
	{:else}
		<!-- Category filters -->
		<div class="flex gap-1 mb-5 flex-wrap">
			{#each categories() as cat}
				<button onclick={() => marketplaceCategory = cat}
					class="px-3 py-1 text-xs font-medium rounded-md transition-colors
						{marketplaceCategory === cat ? 'bg-gray-900 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}">
					{cat === 'all' ? 'All' : cat}
				</button>
			{/each}
		</div>

		<!-- Install dialog -->
		{#if installTarget}
			<div class="bg-white rounded-xl border-2 border-gray-900 p-5 mb-6">
				<h3 class="text-sm font-semibold text-gray-900 mb-1">Install "{installTarget.name}"</h3>
				<p class="text-xs text-gray-500 mb-4">{installTarget.description}</p>

				<div class="space-y-3">
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Command</label>
						<p class="text-xs font-mono text-gray-700 bg-gray-50 px-3 py-2 rounded-md">{installTarget.command}</p>
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Arguments</label>
						<input type="text" bind:value={installArgs}
							class="w-full px-3 py-2 text-xs font-mono border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>

					{#if installEnv.length > 0}
						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">
								Environment Variables <span class="normal-case text-gray-400">(required)</span>
							</label>
							{#each installEnv as pair}
								<div class="flex gap-2 mb-2">
									<span class="px-2 py-1.5 text-xs font-mono bg-gray-100 rounded-md text-gray-700 shrink-0">{pair.key}</span>
									<input type="text" bind:value={pair.value} placeholder="Enter value..."
										class="flex-1 px-2 py-1.5 text-xs font-mono border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<div class="flex gap-2 mt-4">
					<button onclick={confirmInstall} disabled={installing === installTarget.name}
						class="px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
						{installing === installTarget.name ? 'Installing...' : 'Install'}
					</button>
					<button onclick={cancelInstall}
						class="px-4 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-600 hover:bg-gray-50 transition-colors">
						Cancel
					</button>
				</div>
			</div>
		{/if}

		<!-- Catalog grid -->
		{#if filteredCatalog().length === 0}
			<div class="text-center py-12 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">
					{Object.keys(installedServers).length === catalog.length ? 'All available skills installed' : 'No skills match your search'}
				</p>
			</div>
		{:else}
			<div class="grid grid-cols-2 gap-3">
				{#each filteredCatalog() as item}
					<div class="bg-white rounded-xl border border-gray-200 p-4 hover:border-gray-300 transition-colors">
						<div class="flex items-start justify-between mb-2">
							<div>
								<p class="text-sm font-medium text-gray-900">{item.name}</p>
								<p class="text-[10px] text-gray-400 mt-0.5">{item.author}</p>
							</div>
							<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded bg-gray-100 text-gray-600 shrink-0">
								{item.category}
							</span>
						</div>
						<p class="text-xs text-gray-500 mb-3 line-clamp-2">{item.description}</p>
						<div class="flex items-center justify-between">
							<div class="flex gap-1 flex-wrap">
								{#each item.tags.slice(0, 3) as tag}
									<span class="px-1.5 py-0.5 bg-gray-50 rounded text-[10px] text-gray-400">{tag}</span>
								{/each}
							</div>
							{#if installedServers[item.name]}
								<span class="text-[10px] font-medium text-green-600">Installed</span>
							{:else}
								<button onclick={() => startInstall(item)}
									class="px-3 py-1 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
									Install
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
