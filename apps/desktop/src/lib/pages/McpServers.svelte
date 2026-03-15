<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	let servers: Record<string, any> = $state({});
	let loading = $state(true);
	let showForm = $state(false);
	let tab = $state<'installed' | 'catalog'>('installed');
	let catalogSearch = $state('');
	let catalogCategory = $state('all');

	// Form fields
	let name = $state('');
	let transport = $state<'stdio' | 'http'>('stdio');
	let command = $state('');
	let args = $state('');
	let url = $state('');
	let envPairs = $state<{ key: string; value: string }[]>([]);
	let editingName = $state<string | null>(null);
	let saving = $state(false);

	interface CatalogEntry {
		name: string;
		description: string;
		category: string;
		source: string;
		transport: 'stdio' | 'http';
		command: string;
		args: string[];
		env?: { key: string; description: string }[];
		repo: string;
	}

	const catalog: CatalogEntry[] = [
		// Official Anthropic MCP servers
		{
			name: 'filesystem',
			description: 'Read, write, and manage files and directories',
			category: 'files',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-filesystem', '/home/user'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'github',
			description: 'GitHub API — repos, issues, PRs, code search',
			category: 'dev',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-github'],
			env: [{ key: 'GITHUB_PERSONAL_ACCESS_TOKEN', description: 'GitHub PAT' }],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'git',
			description: 'Git operations — clone, commit, diff, log, branch',
			category: 'dev',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-git'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'postgres',
			description: 'Query and manage PostgreSQL databases',
			category: 'database',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-postgres', 'postgresql://localhost/mydb'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'sqlite',
			description: 'Query and manage SQLite databases',
			category: 'database',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-sqlite', '--db-path', '/path/to/db.sqlite'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'memory',
			description: 'Knowledge graph-based persistent memory',
			category: 'ai',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-memory'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'fetch',
			description: 'Fetch and convert web pages to markdown',
			category: 'web',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-fetch'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'brave-search',
			description: 'Web and local search via Brave Search API',
			category: 'web',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-brave-search'],
			env: [{ key: 'BRAVE_API_KEY', description: 'Brave Search API key' }],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'google-maps',
			description: 'Google Maps — geocoding, directions, places, elevation',
			category: 'web',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-google-maps'],
			env: [{ key: 'GOOGLE_MAPS_API_KEY', description: 'Google Maps API key' }],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'slack',
			description: 'Slack — channels, messages, users, reactions',
			category: 'communication',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-slack'],
			env: [
				{ key: 'SLACK_BOT_TOKEN', description: 'Slack Bot token (xoxb-)' },
				{ key: 'SLACK_TEAM_ID', description: 'Slack workspace ID' },
			],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'puppeteer',
			description: 'Browser automation — navigate, screenshot, click, type',
			category: 'web',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-puppeteer'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'sequential-thinking',
			description: 'Dynamic problem-solving through thought sequences',
			category: 'ai',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-sequential-thinking'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'everything',
			description: 'MCP test server — demo tools, resources, prompts',
			category: 'dev',
			source: 'Anthropic',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@modelcontextprotocol/server-everything'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		// Community / third-party
		{
			name: 'docker',
			description: 'Manage Docker containers, images, volumes, networks',
			category: 'dev',
			source: 'Community',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-docker'],
			repo: 'https://github.com/ckreiling/mcp-server-docker',
		},
		{
			name: 'kubernetes',
			description: 'Manage Kubernetes clusters — pods, deployments, services',
			category: 'dev',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', 'mcp-server-kubernetes'],
			repo: 'https://github.com/Flux159/mcp-server-kubernetes',
		},
		{
			name: 'notion',
			description: 'Search, read, and manage Notion pages and databases',
			category: 'productivity',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@notionhq/notion-mcp-server'],
			env: [{ key: 'OPENAPI_MCP_HEADERS', description: '{"Authorization":"Bearer ntn_...","Notion-Version":"2022-06-28"}' }],
			repo: 'https://github.com/makenotion/notion-mcp-server',
		},
		{
			name: 'linear',
			description: 'Linear — issues, projects, teams, cycles',
			category: 'productivity',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@linear/mcp-server'],
			env: [{ key: 'LINEAR_API_KEY', description: 'Linear API key' }],
			repo: 'https://github.com/linear/linear-mcp-server',
		},
		{
			name: 'sentry',
			description: 'Sentry — error tracking, issues, releases',
			category: 'dev',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@sentry/mcp-server'],
			env: [{ key: 'SENTRY_AUTH_TOKEN', description: 'Sentry auth token' }],
			repo: 'https://github.com/getsentry/sentry-mcp-server',
		},
		{
			name: 'playwright',
			description: 'Browser automation via Playwright — navigate, fill, screenshot',
			category: 'web',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@playwright/mcp-server'],
			repo: 'https://github.com/microsoft/playwright-mcp',
		},
		{
			name: 'redis',
			description: 'Redis operations — get, set, lists, hashes, pub/sub',
			category: 'database',
			source: 'Community',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-redis', '--url', 'redis://localhost:6379'],
			repo: 'https://github.com/modelcontextprotocol/servers',
		},
		{
			name: 'mysql',
			description: 'Query and manage MySQL databases',
			category: 'database',
			source: 'Community',
			transport: 'stdio',
			command: 'uvx',
			args: ['mcp-server-mysql', '--host', 'localhost', '--user', 'root', '--db', 'mydb'],
			repo: 'https://github.com/benborla/mcp-server-mysql',
		},
		{
			name: 'grafana',
			description: 'Grafana — dashboards, datasources, alerts, incidents',
			category: 'dev',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', 'mcp-server-grafana'],
			env: [
				{ key: 'GRAFANA_URL', description: 'Grafana instance URL' },
				{ key: 'GRAFANA_API_KEY', description: 'Grafana API key' },
			],
			repo: 'https://github.com/grafana/mcp-grafana',
		},
		{
			name: 'cloudflare',
			description: 'Cloudflare — Workers, KV, R2, D1, DNS',
			category: 'cloud',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@cloudflare/mcp-server-cloudflare'],
			env: [{ key: 'CLOUDFLARE_API_TOKEN', description: 'Cloudflare API token' }],
			repo: 'https://github.com/cloudflare/mcp-server-cloudflare',
		},
		{
			name: 'aws',
			description: 'AWS services — S3, Lambda, EC2, CloudWatch',
			category: 'cloud',
			source: 'Community',
			transport: 'stdio',
			command: 'uvx',
			args: ['awslabs.core-mcp-server@latest'],
			env: [
				{ key: 'AWS_ACCESS_KEY_ID', description: 'AWS access key' },
				{ key: 'AWS_SECRET_ACCESS_KEY', description: 'AWS secret key' },
				{ key: 'AWS_REGION', description: 'AWS region (e.g. us-east-1)' },
			],
			repo: 'https://github.com/awslabs/mcp',
		},
		{
			name: 'stripe',
			description: 'Stripe — payments, customers, subscriptions, invoices',
			category: 'productivity',
			source: 'Community',
			transport: 'stdio',
			command: 'npx',
			args: ['-y', '@stripe/mcp'],
			env: [{ key: 'STRIPE_SECRET_KEY', description: 'Stripe secret key (sk_...)' }],
			repo: 'https://github.com/stripe/agent-toolkit',
		},
	];

	const categories = [
		{ id: 'all', label: 'All' },
		{ id: 'dev', label: 'Development' },
		{ id: 'database', label: 'Database' },
		{ id: 'web', label: 'Web' },
		{ id: 'ai', label: 'AI' },
		{ id: 'communication', label: 'Communication' },
		{ id: 'productivity', label: 'Productivity' },
		{ id: 'cloud', label: 'Cloud' },
		{ id: 'files', label: 'Files' },
	];

	const filteredCatalog = $derived(
		catalog.filter(entry => {
			const matchCategory = catalogCategory === 'all' || entry.category === catalogCategory;
			const matchSearch = !catalogSearch ||
				entry.name.toLowerCase().includes(catalogSearch.toLowerCase()) ||
				entry.description.toLowerCase().includes(catalogSearch.toLowerCase());
			return matchCategory && matchSearch;
		})
	);

	const installedNames = $derived(new Set(Object.keys(servers)));

	function installFromCatalog(entry: CatalogEntry) {
		editingName = null;
		name = entry.name;
		transport = entry.transport;
		command = entry.command;
		args = entry.args.join(' ');
		url = '';
		envPairs = (entry.env || []).map(e => ({ key: e.key, value: '' }));
		showForm = true;
		tab = 'installed';
	}

	async function quickInstall(entry: CatalogEntry) {
		if (entry.env && entry.env.length > 0) {
			installFromCatalog(entry);
			return;
		}
		saving = true;
		try {
			await api.saveMcpServer({
				name: entry.name,
				command: entry.command,
				args: entry.args,
			});
			await loadServers();
			tab = 'installed';
		} catch (_) {}
		saving = false;
	}

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

<div class="max-w-4xl">
	<PageHeader title="MCP Servers" subtitle="Connect external tool servers via Model Context Protocol">
		{#snippet action()}
			{#if tab === 'installed' && !showForm}
				<Button size="sm" onclick={() => { resetForm(); showForm = true; }}>Add Server</Button>
			{/if}
		{/snippet}
	</PageHeader>

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-elevated rounded-lg p-1">
		<button onclick={() => tab = 'installed'}
			class="flex-1 px-4 py-2 text-xs font-medium rounded-md transition-colors
				{tab === 'installed' ? 'bg-base text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Installed ({Object.keys(servers).length})
		</button>
		<button onclick={() => { tab = 'catalog'; showForm = false; }}
			class="flex-1 px-4 py-2 text-xs font-medium rounded-md transition-colors
				{tab === 'catalog' ? 'bg-base text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Catalog ({catalog.length})
		</button>
	</div>

	{#if tab === 'installed'}
		<!-- Add/Edit Form -->
		{#if showForm}
			<div class="bg-base rounded-xl border border-border p-5 mb-6">
				<h3 class="text-sm font-medium text-fg mb-4">{editingName ? 'Edit' : 'Add'} MCP Server</h3>

				<div class="space-y-4">
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Server Name</label>
						<input type="text" bind:value={name} placeholder="e.g. filesystem, github"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>

					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Transport</label>
						<div class="flex gap-2">
							<button onclick={() => transport = 'stdio'}
								class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
									{transport === 'stdio' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
								Stdio (Local)
							</button>
							<button onclick={() => transport = 'http'}
								class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
									{transport === 'http' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
								HTTP (Remote)
							</button>
						</div>
					</div>

					{#if transport === 'stdio'}
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Command</label>
							<input type="text" bind:value={command} placeholder="e.g. npx, uvx, node"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Arguments</label>
							<input type="text" bind:value={args} placeholder="e.g. -y @modelcontextprotocol/server-filesystem /home/user/docs"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
						<div>
							<div class="flex items-center justify-between mb-1">
								<label class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Environment Variables</label>
								<button onclick={addEnvPair} class="text-xs text-fg-muted hover:text-fg">+ Add</button>
							</div>
							{#each envPairs as pair, i}
								<div class="flex gap-2 mb-2">
									<input type="text" bind:value={pair.key} placeholder="KEY"
										class="w-1/3 px-2 py-1.5 text-xs border border-border rounded-md bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono">
									<input type="text" bind:value={pair.value} placeholder="value"
										class="flex-1 px-2 py-1.5 text-xs border border-border rounded-md bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono">
									<button onclick={() => removeEnvPair(i)} class="text-xs text-error hover:text-error px-1">x</button>
								</div>
							{/each}
						</div>
					{:else}
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Server URL</label>
							<input type="text" bind:value={url} placeholder="e.g. http://localhost:8080/sse"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
					{/if}
				</div>

				<div class="flex gap-2 mt-5">
					<button onclick={saveServer} disabled={saving || !name.trim()}
						class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
						{saving ? 'Saving...' : editingName ? 'Update' : 'Save'}
					</button>
					<button onclick={resetForm}
						class="px-4 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
						Cancel
					</button>
				</div>

				<p class="text-[11px] text-fg-muted mt-3">Restart the engine after adding/removing servers for changes to take effect.</p>
			</div>
		{/if}

		<!-- Installed Servers List -->
		{#if loading}
			<p class="text-sm text-fg-muted">Loading...</p>
		{:else if Object.keys(servers).length === 0 && !showForm}
			<div class="text-center py-16 bg-base/50 rounded-xl border border-border">
				<p class="text-3xl mb-3">&#x2B21;</p>
				<p class="text-sm text-fg-muted mb-1">No MCP servers configured</p>
				<p class="text-xs text-fg-muted mb-4">Browse the catalog to find and install servers</p>
				<button onclick={() => tab = 'catalog'}
					class="px-4 py-2 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors">
					Browse Catalog
				</button>
			</div>
		{:else}
			<div class="space-y-3">
				{#each Object.entries(servers) as [serverName, server]}
					<div class="bg-base rounded-xl border border-border p-4">
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-3">
								<span class="inline-flex px-2 py-0.5 text-[10px] font-medium rounded-full
									{server.transport === 'http' ? 'bg-[var(--color-info-15)] text-info' : 'bg-[var(--color-accent-500-15)] text-[var(--color-accent-500)]'}">
									{server.transport === 'http' ? 'HTTP' : 'STDIO'}
								</span>
								<div>
									<p class="text-sm font-medium text-fg">{serverName}</p>
									<p class="text-xs text-fg-muted font-mono mt-0.5">
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
									class="text-xs text-fg-muted hover:text-fg font-medium">Edit</button>
								<button onclick={() => deleteServer(serverName)}
									class="text-xs text-error hover:text-error font-medium">Remove</button>
							</div>
						</div>
						{#if server.env && Object.keys(server.env).length > 0}
							<div class="mt-2 pt-2 border-t border-border">
								<p class="text-[10px] text-fg-muted uppercase tracking-wider mb-1">Env</p>
								<div class="flex flex-wrap gap-1">
									{#each Object.keys(server.env) as key}
										<span class="px-1.5 py-0.5 bg-elevated rounded text-[10px] font-mono text-fg-secondary">{key}</span>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

	{:else}
		<!-- Catalog Tab -->
		<div class="mb-5 space-y-3">
			<!-- Search -->
			<input type="text" bind:value={catalogSearch} placeholder="Search servers..."
				class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-base text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">

			<!-- Category filter -->
			<div class="flex flex-wrap gap-1.5">
				{#each categories as cat}
					<button onclick={() => catalogCategory = cat.id}
						class="px-2.5 py-1 text-[11px] font-medium rounded-full border transition-colors
							{catalogCategory === cat.id
								? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500'
								: 'border-border text-fg-muted hover:bg-[var(--color-elevated-50)]'}">
						{cat.label}
					</button>
				{/each}
			</div>
		</div>

		{#if filteredCatalog.length === 0}
			<div class="text-center py-12">
				<p class="text-sm text-fg-muted">No servers found matching your search</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
				{#each filteredCatalog as entry}
					<div class="bg-base rounded-xl border border-border p-4 flex flex-col justify-between">
						<div>
							<div class="flex items-start justify-between mb-2">
								<div class="flex items-center gap-2">
									<h4 class="text-sm font-medium text-fg">{entry.name}</h4>
									{#if installedNames.has(entry.name)}
										<span class="px-1.5 py-0.5 text-[9px] font-medium rounded-full bg-[var(--color-success-15)] text-success">Installed</span>
									{/if}
								</div>
								<span class="px-1.5 py-0.5 text-[9px] font-medium rounded-full
									{entry.source === 'Anthropic' ? 'bg-[var(--color-accent-500-15)] text-accent-500' : 'bg-elevated text-fg-secondary'}">
									{entry.source}
								</span>
							</div>
							<p class="text-xs text-fg-muted mb-3 leading-relaxed">{entry.description}</p>
							<div class="flex flex-wrap gap-1 mb-3">
								<span class="px-1.5 py-0.5 bg-elevated rounded text-[10px] font-mono text-fg-muted">
									{entry.command} {entry.args[entry.args.length - 1]}
								</span>
								{#if entry.env && entry.env.length > 0}
									<span class="px-1.5 py-0.5 bg-[var(--color-warning-15)] rounded text-[10px] text-warning">
										{entry.env.length} key{entry.env.length > 1 ? 's' : ''} required
									</span>
								{/if}
							</div>
						</div>
						<div class="flex gap-2">
							{#if installedNames.has(entry.name)}
								<button disabled
									class="flex-1 px-3 py-1.5 text-xs font-medium rounded-md bg-elevated text-fg-muted cursor-not-allowed">
									Already Installed
								</button>
							{:else if entry.env && entry.env.length > 0}
								<button onclick={() => installFromCatalog(entry)} disabled={saving}
									class="flex-1 px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
									Configure & Install
								</button>
							{:else}
								<button onclick={() => quickInstall(entry)} disabled={saving}
									class="flex-1 px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
									{saving ? 'Installing...' : 'Install'}
								</button>
							{/if}
							<a href={entry.repo} target="_blank" rel="noopener noreferrer"
								class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-muted hover:bg-[var(--color-elevated-50)] transition-colors">
								Repo
							</a>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>
