<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';
	import { Plus, Bot, Trash2 } from '@amanclaw/ui';

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

	let routingError = $state('');

	async function saveRouting() {
		routingSaving = true;
		routingError = '';
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
		} catch (e: any) {
			routingError = e?.toString() || 'Failed to save routing rules';
		}
		routingSaving = false;
	}

	// --- Init ---
	onMount(() => {
		loadAgents();
		loadRouting();
	});
</script>

<div class="max-w-6xl">
	<PageHeader title="Agents" subtitle="{agents.length} agent{agents.length !== 1 ? 's' : ''} configured" />

	<!-- Split view -->
	<div class="flex gap-6">
		<!-- Left Panel: Agent List (1/3) -->
		<div class="w-1/3 shrink-0">
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Agent Profiles</h3>
				<Button size="sm" onclick={startNewAgent}>
					<Plus size={12} />
					Add Agent
				</Button>
			</div>

			{#if loading}
				<p class="text-sm text-fg-muted">Loading...</p>
			{:else if agents.length === 0 && !isNew}
				<Card>
					<EmptyState icon={Bot} title="No agents configured" description="Create your first agent profile" />
				</Card>
			{:else}
				<div class="space-y-1">
					{#each agents as agent}
						<button onclick={() => selectAgent(agent)}
							class="w-full text-left p-3 rounded-lg border transition-colors
								{selectedId === agent.id && !isNew
									? 'border-primary-500 bg-[var(--color-primary-500-10)]'
									: 'border-border bg-surface hover:border-[var(--color-primary-500-10)]'}">
							<p class="text-[13px] font-medium text-fg truncate">{agent.name}</p>
							<div class="flex items-center gap-2 mt-1.5">
								{#if agent.soul_file}
									<Badge variant="info">{agent.soul_file}</Badge>
								{/if}
								<span class="text-[10px] text-fg-muted">
									{agent.allowed_skills.length} skill{agent.allowed_skills.length !== 1 ? 's' : ''}
								</span>
							</div>
							{#if agent.memory_namespace}
								<p class="text-[10px] text-fg-muted mt-1 font-mono truncate">{agent.memory_namespace}</p>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Right Panel: Agent Editor (2/3) -->
		<div class="flex-1 min-w-0">
			{#if isNew || selectedId}
				<Card>
					<div class="flex items-center justify-between mb-4">
						<h3 class="text-sm font-semibold text-fg">
							{isNew ? 'New Agent' : 'Edit Agent'}
						</h3>
						{#if !isNew && selectedId}
							<button onclick={deleteAgent} disabled={deleting}
								class="text-xs text-error hover:text-error/80 font-medium disabled:opacity-50">
								{deleting ? 'Deleting...' : 'Delete'}
							</button>
						{/if}
					</div>

					<!-- Profile Fields -->
					<div class="space-y-3">
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">ID</label>
							<input type="text" value={editId} readonly
								class="w-full px-3 py-1.5 text-xs font-mono border border-border rounded-md bg-elevated text-fg-muted">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Name</label>
							<input type="text" bind:value={editName} placeholder="Agent name..."
								class="w-full px-3 py-1.5 text-xs border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Memory Namespace</label>
							<input type="text" bind:value={editMemoryNamespace} placeholder="e.g. default, community-a..."
								class="w-full px-3 py-1.5 text-xs font-mono border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>

						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Allowed Skills
								<span class="normal-case text-fg-muted">(comma-separated)</span>
							</label>
							<input type="text" bind:value={editAllowedSkills} placeholder="solat, qiblat, hijri..."
								class="w-full px-3 py-1.5 text-xs border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>

						<!-- System Prompt -->
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">
								System Prompt
								<span class="normal-case text-fg-muted">(used when no soul file is set)</span>
							</label>
							<textarea bind:value={editSystemPrompt} rows={4} placeholder="You are a helpful assistant..."
								class="w-full px-3 py-2 text-xs border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y font-mono"></textarea>
						</div>

						<!-- Soul File Section -->
						<div class="border-t border-border pt-3">
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Soul File (SOUL.md)</label>

							<div class="flex gap-2 mb-2">
								<input type="text" bind:value={editSoulFile} placeholder="filename.md"
									class="flex-1 px-3 py-1.5 text-xs font-mono border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
								<Button variant="secondary" size="sm" onclick={loadSoulFile} disabled={soulLoading || !editSoulFile.trim()}>
									{soulLoading ? 'Loading...' : 'Load'}
								</Button>
								<Button variant="secondary" size="sm" onclick={previewSoul} disabled={previewLoading || !editSoulFile.trim()}>
									{previewLoading ? '...' : 'Preview'}
								</Button>
							</div>

							{#if editSoulContent || editSoulFile.trim()}
								<textarea bind:value={editSoulContent} rows={8} placeholder="Soul file content..."
									class="w-full px-3 py-2 text-xs border border-border rounded-md bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y font-mono mb-2"></textarea>
								<Button variant="secondary" size="sm" onclick={saveSoulFile} disabled={soulSaving || !editSoulFile.trim() || !editSoulContent.trim()}>
									{soulSaving ? 'Saving Soul...' : 'Save Soul File'}
								</Button>
							{/if}

							{#if soulPreview}
								<div class="mt-3 bg-elevated rounded-md border border-border p-3">
									<p class="text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Preview</p>
									{#if soulPreview.variables.length > 0}
										<div class="mb-2">
											<span class="text-[10px] text-fg-muted">Variables:</span>
											<div class="flex gap-1 flex-wrap mt-0.5">
												{#each soulPreview.variables as v}
													<span class="px-1.5 py-0.5 bg-[var(--color-info-15)] rounded text-[10px] text-info font-mono">{v}</span>
												{/each}
											</div>
										</div>
									{/if}
									{#if soulPreview.tags.length > 0}
										<div class="mb-2">
											<span class="text-[10px] text-fg-muted">Tags:</span>
											<div class="flex gap-1 flex-wrap mt-0.5">
												{#each soulPreview.tags as t}
													<span class="px-1.5 py-0.5 bg-elevated rounded text-[10px] text-fg-secondary">{t}</span>
												{/each}
											</div>
										</div>
									{/if}
									<pre class="text-[10px] text-fg-secondary whitespace-pre-wrap mt-2 max-h-48 overflow-y-auto">{soulPreview.prompt}</pre>
								</div>
							{/if}
						</div>
					</div>

					<!-- Save button -->
					<div class="mt-4 pt-4 border-t border-border">
						{#if error}
							<div class="mb-3 p-2 bg-[var(--color-error-15)] border border-[var(--color-error-20)] rounded text-xs text-error">{error}</div>
						{/if}
						<Button size="sm" onclick={saveAgent} disabled={saving || !editName.trim()}>
							{saving ? 'Saving...' : isNew ? 'Create Agent' : 'Save Changes'}
						</Button>
					</div>
				</Card>
			{:else}
				<Card>
					<EmptyState icon={Bot} title="Select an agent to edit" description="Or create a new agent profile" />
				</Card>
			{/if}
		</div>
	</div>

	<!-- Bottom Section: Routing Rules -->
	<div class="mt-8">
		<div class="flex items-center justify-between mb-3">
			<div>
				<h3 class="text-sm font-semibold text-fg">Routing Rules</h3>
				<p class="text-xs text-fg-muted mt-0.5">Map platforms, topics, and channels to specific agents</p>
			</div>
			<Button size="sm" onclick={saveRouting} disabled={routingSaving}>
				{routingSaving ? 'Saving...' : 'Save Rules'}
			</Button>
		</div>
		{#if routingError}
			<div class="mb-3 p-2 bg-[var(--color-error-15)] border border-[var(--color-error-20)] rounded text-xs text-error">{routingError}</div>
		{/if}

		<!-- Default agent -->
		<div class="flex items-center gap-3 mb-4 bg-surface rounded-lg border border-border p-3">
			<label class="text-xs text-fg-muted shrink-0">Default Agent:</label>
			<select bind:value={defaultAgent}
				class="px-2 py-1 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-elevated text-fg">
				<option value="">-- None --</option>
				{#each agents as agent}
					<option value={agent.id}>{agent.name}</option>
				{/each}
			</select>
		</div>

		<!-- Rules table -->
		<div class="bg-surface rounded-lg border border-border overflow-hidden">
			<table class="w-full text-xs">
				<thead>
					<tr class="border-b border-border bg-elevated">
						<th class="text-left px-3 py-2 text-[10px] font-medium text-fg-muted uppercase">Platform</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-fg-muted uppercase">Topic ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-fg-muted uppercase">Channel ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-fg-muted uppercase">Group ID</th>
						<th class="text-left px-3 py-2 text-[10px] font-medium text-fg-muted uppercase">Agent</th>
						<th class="px-3 py-2 w-20"></th>
					</tr>
				</thead>
				<tbody>
					{#each routingRules as rule, i}
						{#if editingRuleIndex === i}
							<tr class="border-b border-border bg-[var(--color-primary-500-10)]">
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.platform} placeholder="telegram"
										class="w-full px-2 py-1 text-xs border border-border rounded bg-elevated text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.topic_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-border rounded bg-elevated text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.channel_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-border rounded bg-elevated text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
								</td>
								<td class="px-2 py-1.5">
									<input type="text" bind:value={rule.group_id} placeholder="*"
										class="w-full px-2 py-1 text-xs border border-border rounded bg-elevated text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
								</td>
								<td class="px-2 py-1.5">
									<select bind:value={rule.agent}
										class="w-full px-2 py-1 text-xs border border-border rounded bg-elevated text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
										<option value="">-- Select --</option>
										{#each agents as agent}
											<option value={agent.id}>{agent.name}</option>
										{/each}
									</select>
								</td>
								<td class="px-2 py-1.5 text-center">
									<button onclick={() => finishEditRule()}
										class="text-[10px] text-success hover:text-success/80 font-medium">Done</button>
								</td>
							</tr>
						{:else}
							<tr class="border-b border-border hover:bg-[var(--color-elevated-50)]">
								<td class="px-3 py-2 text-fg-secondary font-mono">{rule.platform || '*'}</td>
								<td class="px-3 py-2 text-fg-muted font-mono">{rule.topic_id || '*'}</td>
								<td class="px-3 py-2 text-fg-muted font-mono">{rule.channel_id || '*'}</td>
								<td class="px-3 py-2 text-fg-muted font-mono">{rule.group_id || '*'}</td>
								<td class="px-3 py-2 text-fg-secondary">
									{agents.find(a => a.id === rule.agent)?.name || rule.agent}
								</td>
								<td class="px-2 py-2 text-center">
									<div class="flex gap-2 justify-center">
										<button onclick={() => startEditRule(i)}
											class="text-[10px] text-fg-muted hover:text-fg font-medium">Edit</button>
										<button onclick={() => removeRule(i)}
											class="text-[10px] text-error/70 hover:text-error font-medium">Del</button>
									</div>
								</td>
							</tr>
						{/if}
					{/each}

					<!-- Add new rule row -->
					<tr class="bg-[var(--color-elevated-50)]">
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.platform} placeholder="telegram"
								class="w-full px-2 py-1 text-xs border border-border rounded bg-surface text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.topic_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-border rounded bg-surface text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.channel_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-border rounded bg-surface text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
						</td>
						<td class="px-2 py-1.5">
							<input type="text" bind:value={newRule.group_id} placeholder="*"
								class="w-full px-2 py-1 text-xs border border-border rounded bg-surface text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
						</td>
						<td class="px-2 py-1.5">
							<select bind:value={newRule.agent}
								class="w-full px-2 py-1 text-xs border border-border rounded bg-surface text-fg focus:outline-none focus:ring-1 focus:ring-primary-500">
								<option value="">-- Select --</option>
								{#each agents as agent}
									<option value={agent.id}>{agent.name}</option>
								{/each}
							</select>
						</td>
						<td class="px-2 py-1.5 text-center">
							<button onclick={addRule} disabled={!newRule.agent}
								class="text-[10px] text-fg hover:text-fg-secondary font-medium disabled:opacity-30">
								Add
							</button>
						</td>
					</tr>
				</tbody>
			</table>

			{#if routingRules.length === 0}
				<div class="text-center py-6 border-t border-border">
					<p class="text-xs text-fg-muted">No routing rules. All messages go to the default agent.</p>
				</div>
			{/if}
		</div>
	</div>
</div>
