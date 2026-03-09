<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	// --- State ---
	let skills: any[] = $state([]);
	let installedServers: Record<string, any> = $state({});
	let loading = $state(true);
	let search = $state('');
	let expandedSkill = $state<string | null>(null);
	let showRestart = $state(false);
	let engineRunning = $state(false);
	let disabledSkills = $state<string[]>([]);

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

	// MCP servers installed in config but not yet active (no skills loaded from them)
	const pendingServers = $derived(() => {
		const activeServerNames = new Set(
			skills.filter((s: any) => s.source === 'mcp').map((s: any) => s.name.split('__')[0])
		);
		return Object.entries(installedServers)
			.filter(([name]) => !activeServerNames.has(name))
			.map(([name, server]) => ({
				name,
				server,
				// If engine is running but no skills from this server → connection failed
				failed: engineRunning && !activeServerNames.has(name),
			}));
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

	async function uninstallServer(name: string) {
		try {
			await api.deleteMcpServer(name);
			showRestart = true;
			await loadInstalledServers();
		} catch (_) {}
	}

	async function handleRestart() {
		try {
			if (engineRunning) {
				await api.restartEngine();
			} else {
				await api.startEngine();
			}
			showRestart = false;
			// Reload skills and status after engine starts
			await loadEngineStatus();
			await loadSkills();
			// Retry after a delay in case MCP servers are slow to connect
			setTimeout(async () => {
				await loadSkills();
				await loadEngineStatus();
			}, 3000);
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

	async function loadEngineStatus() {
		try {
			const data = await api.getStatus() as any;
			engineRunning = data.engine_status === 'running';
		} catch (_) {}
	}

	async function loadDisabledSkills() {
		try {
			disabledSkills = await api.getDisabledSkills();
		} catch (_) {}
	}

	async function toggleSkill(name: string) {
		const isDisabled = disabledSkills.includes(name);
		try {
			if (isDisabled) {
				await api.enableSkill(name);
			} else {
				await api.disableSkill(name);
			}
			await loadDisabledSkills();
			showRestart = true;
		} catch (_) {}
	}

	onMount(() => {
		loadSkills();
		loadInstalledServers();
		loadEngineStatus();
		loadDisabledSkills();
		const interval = setInterval(() => {
			loadSkills();
			loadEngineStatus();
		}, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="p-8 max-w-4xl">
	<!-- Header -->
	<div class="flex items-center justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Skills</h2>
			<p class="text-sm text-gray-500 mt-1">
				{skills.length} active skill{skills.length !== 1 ? 's' : ''}{#if pendingServers().length > 0}, {pendingServers().length} pending{/if}
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

	<!-- ===================== INSTALLED SKILLS ===================== -->
	{#if true}
		{#if loading}
			<p class="text-sm text-gray-500">Loading...</p>
		{:else if skills.length === 0 && pendingServers().length === 0}
			<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">No skills registered</p>
				<p class="text-xs text-gray-400 mt-1">Start the engine or install skills from the Marketplace page</p>
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

											<!-- Actions -->
											<div class="mt-3 pt-3 border-t border-gray-100 flex gap-3">
												{#if skill.source === 'builtin'}
													<button onclick={() => toggleSkill(skill.name)}
														class="text-xs text-red-500 hover:text-red-700 font-medium">
														Remove
													</button>
												{:else if skill.source === 'mcp'}
													{@const serverName = skill.name.split('__')[0]}
													{#if installedServers[serverName]}
														<button onclick={() => uninstallServer(serverName)}
															class="text-xs text-red-500 hover:text-red-700 font-medium">
															Uninstall {serverName}
														</button>
													{/if}
												{/if}
											</div>
										</div>
									{/if}
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Pending MCP servers (installed but not yet active) -->
		{#if pendingServers().length > 0}
			<div class="mt-6">
				<h3 class="text-[11px] font-medium text-gray-400 uppercase tracking-wider mb-2">
					{#if pendingServers().some(s => s.failed)}
						Connection Issues ({pendingServers().length})
					{:else}
						Pending Restart ({pendingServers().length})
					{/if}
				</h3>
				<div class="space-y-1">
					{#each pendingServers() as { name: serverName, server, failed }}
						<div class="bg-white rounded-lg border border-dashed {failed ? 'border-red-300' : 'border-yellow-300'} overflow-hidden">
							<div class="flex items-center justify-between p-3">
								<div class="flex items-center gap-3 min-w-0">
									{#if failed}
										<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded bg-red-100 text-red-700">
											Failed
										</span>
									{:else}
										<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded bg-yellow-100 text-yellow-700">
											Pending
										</span>
									{/if}
									<div class="min-w-0">
										<p class="text-sm font-medium text-gray-900 truncate">{serverName}</p>
										<p class="text-xs text-gray-400 truncate mt-0.5 font-mono">
											{server.command} {(server.args || []).join(' ')}
										</p>
										{#if failed}
											<p class="text-xs text-red-500 mt-1">Failed to connect. Check that the command is available in your PATH.</p>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2 shrink-0 ml-3">
									<button onclick={() => uninstallServer(serverName)}
										class="text-xs text-red-400 hover:text-red-600">Remove</button>
									<button onclick={handleRestart}
										class="px-2.5 py-1 text-[10px] font-medium rounded-md {failed ? 'bg-gray-700' : 'bg-yellow-600'} text-white hover:opacity-90 transition-colors">
										{engineRunning ? 'Retry' : 'Start Engine'}
									</button>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Disabled skills -->
		{#if disabledSkills.length > 0}
			<div class="mt-6">
				<h3 class="text-[11px] font-medium text-gray-400 uppercase tracking-wider mb-2">
					Removed ({disabledSkills.length})
				</h3>
				<div class="space-y-1">
					{#each disabledSkills as skillName}
						<div class="bg-white rounded-lg border border-gray-200 overflow-hidden opacity-60">
							<div class="flex items-center justify-between p-3">
								<div class="flex items-center gap-3 min-w-0">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded bg-gray-100 text-gray-500">
										Removed
									</span>
									<p class="text-sm font-medium text-gray-500 truncate">{skillName}</p>
								</div>
								<button onclick={() => toggleSkill(skillName)}
									class="text-xs text-green-600 hover:text-green-800 font-medium shrink-0 ml-3">
									Re-add
								</button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

	{/if}
</div>
