<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	// --- Types ---
	interface Agent {
		id: string;
		name: string;
		system_prompt: string;
		soul_file?: string;
		allowed_skills: string[];
		memory_namespace: string;
	}

	interface RoutingRule {
		platform: string;
		topic_id: string;
		channel_id: string;
		group_id: string;
		agent: string;
	}

	// --- State ---
	let agents: Agent[] = $state([]);
	let selectedId = $state<string | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let deleting = $state(false);
	let error = $state('');

	// Editor state
	let editId = $state('');
	let editName = $state('');
	let editMemoryNamespace = $state('');
	let editAllowedSkills = $state('');
	let editSystemPrompt = $state('');
	let editSoulFile = $state('');
	let editSoulContent = $state('');
	let soulLoading = $state(false);
	let soulSaving = $state(false);
	let soulPreview = $state<{ prompt: string; variables: string[]; tags: string[] } | null>(null);
	let previewLoading = $state(false);
	let isNew = $state(false);

	// Routing state
	let routingRules: RoutingRule[] = $state([]);
	let defaultAgent = $state('');
	let routingSaving = $state(false);
	let editingRuleIndex = $state<number | null>(null);
	let newRule = $state<RoutingRule>({ platform: '', topic_id: '', channel_id: '', group_id: '', agent: '' });

	// --- Derived ---
	const selectedAgent = $derived(() => agents.find(a => a.id === selectedId) || null);

	// --- Agent CRUD ---
	async function loadAgents() {
		try {
			const data = await api.listAgents() as any;
			agents = data.agents || [];
		} catch (_) {}
		loading = false;
	}

	async function loadRouting() {
		try {
			const data = await api.getRoutingRules() as any;
			routingRules = (data.rules || []).map((r: any) => ({
				platform: r.match?.platform || '',
				topic_id: r.match?.topic_id || '',
				channel_id: r.match?.channel_id || '',
				group_id: r.match?.group_id || '',
				agent: r.agent || '',
			}));
			defaultAgent = data.default_agent || '';
		} catch (_) {}
	}

	function selectAgent(agent: Agent) {
		selectedId = agent.id;
		editId = agent.id;
		editName = agent.name;
		editMemoryNamespace = agent.memory_namespace;
		editAllowedSkills = agent.allowed_skills.join(', ');
		editSystemPrompt = agent.system_prompt;
		editSoulFile = agent.soul_file || '';
		editSoulContent = '';
		soulPreview = null;
		isNew = false;
	}

	function startNewAgent() {
		selectedId = null;
		editId = crypto.randomUUID().slice(0, 8);
		editName = '';
		editMemoryNamespace = '';
		editAllowedSkills = '';
		editSystemPrompt = '';
		editSoulFile = '';
		editSoulContent = '';
		soulPreview = null;
		isNew = true;
	}

	async function saveAgent() {
		saving = true;
		error = '';
		try {
			const skills = editAllowedSkills
				.split(',')
				.map(s => s.trim())
				.filter(s => s.length > 0);
			await api.saveAgent({
				id: editId,
				name: editName,
				systemPrompt: editSystemPrompt,
				soulFile: editSoulFile || undefined,
				allowedSkills: skills,
				memoryNamespace: editMemoryNamespace,
			});
			await loadAgents();
			selectedId = editId;
			isNew = false;
		} catch (e: any) {
			error = e?.toString() || 'Failed to save agent';
		}
		saving = false;
	}

	async function deleteAgent() {
		if (!selectedId) return;
		deleting = true;
		try {
			await api.deleteAgent(selectedId);
			selectedId = null;
			isNew = false;
			editId = '';
			editName = '';
			editMemoryNamespace = '';
			editAllowedSkills = '';
			editSystemPrompt = '';
			editSoulFile = '';
			editSoulContent = '';
			soulPreview = null;
			await loadAgents();
		} catch (_) {}
		deleting = false;
	}

	// --- Soul file ---
	async function loadSoulFile() {
		if (!editSoulFile.trim()) return;
		soulLoading = true;
		try {
			const content = await api.loadSoulFile(editSoulFile.trim()) as string;
			editSoulContent = content;
		} catch (_) {
			editSoulContent = '';
		}
		soulLoading = false;
	}

	async function saveSoulFile() {
		if (!editSoulFile.trim()) return;
		soulSaving = true;
		try {
			await api.saveSoulFile(editSoulFile.trim(), editSoulContent);
		} catch (_) {}
		soulSaving = false;
	}

	async function previewSoul() {
		if (!editSoulFile.trim()) return;
		previewLoading = true;
		try {
			const data = await api.previewSoul(editSoulFile.trim()) as any;
			soulPreview = data;
		} catch (_) {
			soulPreview = null;
		}
		previewLoading = false;
	}

	// --- Routing rules ---
	function addRule() {
		routingRules = [...routingRules, { ...newRule }];
		newRule = { platform: '', topic_id: '', channel_id: '', group_id: '', agent: '' };
	}

	function removeRule(index: number) {
		routingRules = routingRules.filter((_, i) => i !== index);
	}

	function startEditRule(index: number) {
		editingRuleIndex = index;
	}

	function finishEditRule() {
		editingRuleIndex = null;
	}

	async function saveRouting() {
		routingSaving = true;
		try {
			const nested = routingRules.map(r => ({
				match: {
					platform: r.platform || null,
					topic_id: r.topic_id || null,
					channel_id: r.channel_id || null,
					group_id: r.group_id || null,
				},
				agent: r.agent,
			}));
			await api.saveRoutingRules(defaultAgent, nested);
			await loadRouting();
		} catch (_) {}
		routingSaving = false;
	}

	// --- Init ---
	onMount(() => {
		loadAgents();
		loadRouting();
	});
</script>

<div class="p-8 max-w-6xl">
	<!-- Header -->
	<div class="mb-6">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Agents</h2>
		<p class="text-sm text-gray-500 mt-1">
			{agents.length} agent{agents.length !== 1 ? 's' : ''} configured
		</p>
	</div>

	<!-- Split view -->
	<div class="flex gap-6">
		<!-- Left Panel: Agent List (1/3) -->
		<div class="w-1/3 shrink-0">
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-[11px] font-medium text-gray-400 uppercase tracking-wider">Agent Profiles</h3>
				<button onclick={startNewAgent}
					class="px-3 py-1 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					Add Agent
				</button>
			</div>

			{#if loading}
				<p class="text-sm text-gray-500">Loading...</p>
			{:else if agents.length === 0 && !isNew}
				<div class="text-center py-12 bg-gray-50 rounded-xl border border-gray-200">
					<p class="text-sm text-gray-500">No agents configured</p>
					<p class="text-xs text-gray-400 mt-1">Create your first agent profile</p>
				</div>
			{:else}
				<div class="space-y-1">
					{#each agents as agent}
						<button onclick={() => selectAgent(agent)}
							class="w-full text-left p-3 rounded-lg border transition-colors
								{selectedId === agent.id && !isNew
									? 'border-gray-900 bg-gray-50'
									: 'border-gray-200 bg-white hover:border-gray-300'}">
							<p class="text-sm font-medium text-gray-900 truncate">{agent.name}</p>
							<div class="flex items-center gap-2 mt-1.5">
								{#if agent.soul_file}
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded bg-blue-100 text-blue-700 truncate max-w-[120px]">
										{agent.soul_file}
									</span>
								{/if}
								<span class="text-[10px] text-gray-400">
									{agent.allowed_skills.length} skill{agent.allowed_skills.length !== 1 ? 's' : ''}
								</span>
							</div>
							{#if agent.memory_namespace}
								<p class="text-[10px] text-gray-400 mt-1 font-mono truncate">{agent.memory_namespace}</p>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Right Panel: Agent Editor (2/3) -->
		<div class="flex-1 min-w-0">
			{#if isNew || selectedId}
				<div class="bg-white rounded-xl border border-gray-200 p-5">
					<div class="flex items-center justify-between mb-4">
						<h3 class="text-sm font-semibold text-gray-900">
							{isNew ? 'New Agent' : 'Edit Agent'}
						</h3>
						{#if !isNew && selectedId}
							<button onclick={deleteAgent} disabled={deleting}
								class="text-xs text-red-500 hover:text-red-700 font-medium disabled:opacity-50">
								{deleting ? 'Deleting...' : 'Delete'}
							</button>
						{/if}
					</div>

					<!-- Profile Fields -->
					<div class="space-y-3">
						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">ID</label>
							<input type="text" value={editId} readonly
								class="w-full px-3 py-1.5 text-xs font-mono border border-gray-200 rounded-md bg-gray-50 text-gray-500">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Name</label>
							<input type="text" bind:value={editName} placeholder="Agent name..."
								class="w-full px-3 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Memory Namespace</label>
							<input type="text" bind:value={editMemoryNamespace} placeholder="e.g. default, community-a..."
								class="w-full px-3 py-1.5 text-xs font-mono border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">Allowed Skills
								<span class="normal-case text-gray-400">(comma-separated)</span>
							</label>
							<input type="text" bind:value={editAllowedSkills} placeholder="solat, qiblat, hijri..."
								class="w-full px-3 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
						</div>

						<!-- System Prompt -->
						<div>
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-1">
								System Prompt
								<span class="normal-case text-gray-400">(used when no soul file is set)</span>
							</label>
							<textarea bind:value={editSystemPrompt} rows={4} placeholder="You are a helpful assistant..."
								class="w-full px-3 py-2 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900 resize-y font-mono"></textarea>
						</div>

						<!-- Soul File Section -->
						<div class="border-t border-gray-100 pt-3">
							<label class="block text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-2">Soul File (SOUL.md)</label>

							<div class="flex gap-2 mb-2">
								<input type="text" bind:value={editSoulFile} placeholder="filename.md"
									class="flex-1 px-3 py-1.5 text-xs font-mono border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
								<button onclick={loadSoulFile} disabled={soulLoading || !editSoulFile.trim()}
									class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-700 hover:bg-gray-50 transition-colors disabled:opacity-50">
									{soulLoading ? 'Loading...' : 'Load'}
								</button>
								<button onclick={previewSoul} disabled={previewLoading || !editSoulFile.trim()}
									class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-700 hover:bg-gray-50 transition-colors disabled:opacity-50">
									{previewLoading ? '...' : 'Preview'}
								</button>
							</div>

							{#if editSoulContent || editSoulFile.trim()}
								<textarea bind:value={editSoulContent} rows={8} placeholder="Soul file content..."
									class="w-full px-3 py-2 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900 resize-y font-mono mb-2"></textarea>
								<button onclick={saveSoulFile} disabled={soulSaving || !editSoulFile.trim() || !editSoulContent.trim()}
									class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-700 text-white hover:bg-gray-600 transition-colors disabled:opacity-50">
									{soulSaving ? 'Saving Soul...' : 'Save Soul File'}
								</button>
							{/if}

							{#if soulPreview}
								<div class="mt-3 bg-gray-50 rounded-md border border-gray-200 p-3">
									<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-2">Preview</p>
									{#if soulPreview.variables.length > 0}
										<div class="mb-2">
											<span class="text-[10px] text-gray-400">Variables:</span>
											<div class="flex gap-1 flex-wrap mt-0.5">
												{#each soulPreview.variables as v}
													<span class="px-1.5 py-0.5 bg-blue-50 rounded text-[10px] text-blue-600 font-mono">{v}</span>
												{/each}
											</div>
										</div>
									{/if}
									{#if soulPreview.tags.length > 0}
										<div class="mb-2">
											<span class="text-[10px] text-gray-400">Tags:</span>
											<div class="flex gap-1 flex-wrap mt-0.5">
												{#each soulPreview.tags as t}
													<span class="px-1.5 py-0.5 bg-gray-100 rounded text-[10px] text-gray-600">{t}</span>
												{/each}
											</div>
										</div>
									{/if}
									<pre class="text-[10px] text-gray-600 whitespace-pre-wrap mt-2 max-h-48 overflow-y-auto">{soulPreview.prompt}</pre>
								</div>
							{/if}
						</div>
					</div>

					<!-- Save button -->
					<div class="mt-4 pt-4 border-t border-gray-100">
						{#if error}
							<div class="mb-3 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">{error}</div>
						{/if}
						<button onclick={saveAgent} disabled={saving || !editName.trim()}
							class="px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
							{saving ? 'Saving...' : isNew ? 'Create Agent' : 'Save Changes'}
						</button>
					</div>
				</div>
			{:else}
				<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
					<p class="text-sm text-gray-500">Select an agent to edit</p>
					<p class="text-xs text-gray-400 mt-1">Or create a new agent profile</p>
				</div>
			{/if}
		</div>
	</div>

	<!-- Bottom Section: Routing Rules -->
	<div class="mt-8">
		<div class="flex items-center justify-between mb-3">
			<div>
				<h3 class="text-sm font-semibold text-gray-900">Routing Rules</h3>
				<p class="text-xs text-gray-500 mt-0.5">Map platforms, topics, and channels to specific agents</p>
			</div>
			<button onclick={saveRouting} disabled={routingSaving}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
				{routingSaving ? 'Saving...' : 'Save Rules'}
			</button>
		</div>

		<!-- Default agent -->
		<div class="flex items-center gap-3 mb-4 bg-white rounded-lg border border-gray-200 p-3">
			<label class="text-xs text-gray-500 shrink-0">Default Agent:</label>
			<select bind:value={defaultAgent}
				class="px-2 py-1 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900 bg-white">
				<option value="">-- None --</option>
				{#each agents as agent}
					<option value={agent.id}>{agent.name}</option>
				{/each}
			</select>
		</div>

		<!-- Rules table -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<table class="w-full text-xs">
				<thead>
					<tr class="border-b border-gray-100 bg-gray-50">
						<th class="text-left px-3 py-2 text-[10px] font-medium text-gray-400 uppercase">Platform</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-gray-400 uppercase">Topic ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-gray-400 uppercase">Channel ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-gray-400 uppercase">Group ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-gray-400 uppercase">Agent</th>
						<th class="px-3 py-2 w-20"></th>
					</tr>
				</thead>
				<tbody>
					{#each routingRules as rule, i}
						{#if editingRuleIndex === i}
							<tr class="border-b border-gray-50 bg-blue-50/30">
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.platform} placeholder="telegram"
										class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.topic_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.channel_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.group_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900">
								</td>
								<td class="px-2 py-1.5">
									<select bind:value={rule.agent}
										class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
										<option value="">-- Select --</option>
										{#each agents as agent}
											<option value={agent.id}>{agent.name}</option>
										{/each}
									</select>
								</td>
								<td class="px-2 py-1.5 text-center">
									<button onclick={() => finishEditRule()}
										class="text-[10px] text-green-600 hover:text-green-800 font-medium">Done</button>
								</td>
							</tr>
						{:else}
							<tr class="border-b border-gray-50 hover:bg-gray-50/50">
								<td class="px-3 py-2 text-gray-700 font-mono">{rule.platform || '*'}</td>
								<td class="px-3 py-2 text-gray-500 font-mono">{rule.topic_id || '*'}</td>
								<td class="px-3 py-2 text-gray-500 font-mono">{rule.channel_id || '*'}</td>
								<td class="px-3 py-2 text-gray-500 font-mono">{rule.group_id || '*'}</td>
								<td class="px-3 py-2 text-gray-700">
									{agents.find(a => a.id === rule.agent)?.name || rule.agent}
								</td>
								<td class="px-2 py-2 text-center">
									<div class="flex gap-2 justify-center">
										<button onclick={() => startEditRule(i)}
											class="text-[10px] text-gray-400 hover:text-gray-700 font-medium">Edit</button>
										<button onclick={() => removeRule(i)}
											class="text-[10px] text-red-400 hover:text-red-600 font-medium">Del</button>
									</div>
								</td>
							</tr>
						{/if}
					{/each}

					<!-- Add new rule row -->
					<tr class="bg-gray-50/50">
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.platform} placeholder="telegram"
								class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.topic_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.channel_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.group_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
						</td>
						<td class="px-2 py-1.5">
							<select bind:value={newRule.agent}
								class="w-full px-2 py-1 text-xs border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-gray-900 bg-white">
								<option value="">-- Select --</option>
								{#each agents as agent}
									<option value={agent.id}>{agent.name}</option>
								{/each}
							</select>
						</td>
						<td class="px-2 py-1.5 text-center">
							<button onclick={addRule} disabled={!newRule.agent}
								class="text-[10px] text-gray-900 hover:text-gray-700 font-medium disabled:opacity-30">
								Add
							</button>
						</td>
					</tr>
				</tbody>
			</table>

			{#if routingRules.length === 0}
				<div class="text-center py-6 border-t border-gray-100">
					<p class="text-xs text-gray-400">No routing rules. All messages go to the default agent.</p>
				</div>
			{/if}
		</div>
	</div>
</div>
