<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	// --- State ---
	let tab = $state<'endpoints' | 'history'>('endpoints');
	let endpoints: any[] = $state([]);
	let basePath = $state('');
	let history: any[] = $state([]);
	let loading = $state(true);
	let showForm = $state(false);
	let saving = $state(false);
	let editingId = $state<string | null>(null);

	// Form fields
	let name = $state('');
	let path = $state('');
	let pathManual = $state(false);
	let authType = $state<'none' | 'hmac_sha256' | 'bearer' | 'header_match'>('none');
	let authSecret = $state('');
	let authToken = $state('');
	let authHeaderName = $state('');
	let authHeaderValue = $state('');
	let transformType = $state<'raw_json' | 'json_path' | 'template' | 'agent_prompt' | 'skill_invocation'>('raw_json');
	let messagePath = $state('');
	let titlePath = $state('');
	let templateBody = $state('');
	let promptTemplate = $state('');
	let agent = $state('');
	let skill = $state('');
	let inputTemplate = $state('');
	let targets = $state<{ platform: string; chat_id: string; topic_id: string }[]>([]);
	let rateLimit = $state('');
	let enabled = $state(true);

	// Derived
	let autoPath = $derived(name.trim() ? '/hooks/' + name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-') : '/hooks/');

	// --- Data Loading ---
	async function loadEndpoints() {
		try {
			const data = await api.listWebhookEndpoints() as any;
			endpoints = data.endpoints || [];
			basePath = data.base_path || '';
		} catch (_) {}
		loading = false;
	}

	async function loadHistory() {
		try {
			const data = await api.getWebhookHistory() as any;
			history = data.entries || [];
		} catch (_) {}
	}

	// --- Form ---
	function resetForm() {
		name = '';
		path = '';
		pathManual = false;
		authType = 'none';
		authSecret = '';
		authToken = '';
		authHeaderName = '';
		authHeaderValue = '';
		transformType = 'raw_json';
		messagePath = '';
		titlePath = '';
		templateBody = '';
		promptTemplate = '';
		agent = '';
		skill = '';
		inputTemplate = '';
		targets = [];
		rateLimit = '';
		enabled = true;
		editingId = null;
		showForm = false;
	}

	function addTarget() {
		targets = [...targets, { platform: '', chat_id: '', topic_id: '' }];
	}

	function removeTarget(index: number) {
		targets = targets.filter((_, i) => i !== index);
	}

	function editEndpoint(ep: any) {
		editingId = ep.id;
		name = ep.name || '';
		path = ep.path || '';
		pathManual = true;
		authType = ep.auth?.type || 'none';
		authSecret = ep.auth?.secret || '';
		authToken = ep.auth?.token || '';
		authHeaderName = ep.auth?.header_name || '';
		authHeaderValue = ep.auth?.header_value || '';
		transformType = ep.transform?.type || 'raw_json';
		messagePath = ep.transform?.message_path || '';
		titlePath = ep.transform?.title_path || '';
		templateBody = ep.transform?.template || '';
		promptTemplate = ep.transform?.prompt_template || '';
		agent = ep.transform?.agent || '';
		skill = ep.transform?.skill || '';
		inputTemplate = ep.transform?.input_template || '';
		targets = (ep.targets || []).map((t: any) => ({ platform: t.platform || '', chat_id: t.chat_id || '', topic_id: t.topic_id || '' }));
		rateLimit = ep.rate_limit ? String(ep.rate_limit) : '';
		enabled = ep.enabled !== false;
		showForm = true;
	}

	async function saveEndpoint() {
		if (!name.trim()) return;
		saving = true;
		try {
			const auth: any = {};
			auth.type = authType;
			if (authType === 'hmac_sha256') auth.secret = authSecret;
			if (authType === 'bearer') auth.token = authToken;
			if (authType === 'header_match') {
				auth.header_name = authHeaderName;
				auth.header_value = authHeaderValue;
			}

			const transform: any = { type: transformType };
			if (transformType === 'json_path') {
				transform.message_path = messagePath;
				transform.title_path = titlePath;
			}
			if (transformType === 'template') {
				transform.template = templateBody;
			}
			if (transformType === 'agent_prompt') {
				transform.prompt_template = promptTemplate;
				transform.agent = agent;
			}
			if (transformType === 'skill_invocation') {
				transform.skill = skill;
				transform.input_template = inputTemplate;
			}

			const endpoint: any = {
				name: name.trim(),
				path: pathManual ? path.trim() : autoPath,
				auth: authType !== 'none' ? auth : undefined,
				transform,
				targets: targets.filter(t => t.platform.trim() && t.chat_id.trim()),
				enabled,
			};
			if (rateLimit.trim()) endpoint.rate_limit = parseInt(rateLimit, 10);

			await api.saveWebhookEndpoint(editingId || name.trim(), endpoint);
			resetForm();
			await loadEndpoints();
		} catch (_) {}
		saving = false;
	}

	async function deleteEndpoint(id: string) {
		try {
			await api.deleteWebhookEndpoint(id);
			await loadEndpoints();
		} catch (_) {}
	}

	async function toggleEnabled(ep: any) {
		try {
			await api.saveWebhookEndpoint(ep.id, { ...ep, enabled: !ep.enabled });
			await loadEndpoints();
		} catch (_) {}
	}

	function statusBadgeClass(status: string): string {
		if (status === 'success' || status === 'ok') return 'bg-[var(--color-success-15)] text-success';
		if (status === 'error' || status === 'failed') return 'bg-[var(--color-error-15)] text-error';
		if (status === 'rate_limited') return 'bg-[var(--color-warning-15)] text-warning';
		return 'bg-elevated text-fg-secondary';
	}

	function authBadgeClass(type: string): string {
		if (type === 'hmac_sha256') return 'bg-[var(--color-accent-500-15)] text-[var(--color-accent-500)]';
		if (type === 'bearer') return 'bg-[var(--color-info-15)] text-info';
		if (type === 'header_match') return 'bg-[var(--color-accent-500-15)] text-accent-500';
		return 'bg-elevated text-fg-muted';
	}

	function truncate(s: string, len: number): string {
		if (!s) return '';
		return s.length > len ? s.slice(0, len) + '...' : s;
	}

	function formatTime(ts: string): string {
		if (!ts) return '';
		try {
			return new Date(ts).toLocaleString();
		} catch {
			return ts;
		}
	}

	onMount(() => {
		loadEndpoints();
		loadHistory();
		const interval = setInterval(() => {
			if (tab === 'history') loadHistory();
		}, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="max-w-5xl">
	<PageHeader title="Webhooks" subtitle={tab === 'endpoints' ? `${endpoints.length} endpoint${endpoints.length !== 1 ? 's' : ''} configured` : 'Incoming webhook delivery history'}>
		{#snippet action()}
			{#if tab === 'endpoints' && !showForm}
				<Button size="sm" onclick={() => { resetForm(); showForm = true; }}>Add Webhook</Button>
			{/if}
		{/snippet}
	</PageHeader>

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-elevated rounded-lg p-0.5 w-fit">
		<button onclick={() => tab = 'endpoints'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'endpoints' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Endpoints
		</button>
		<button onclick={() => { tab = 'history'; loadHistory(); }}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'history' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			History
		</button>
	</div>

	<!-- ===================== ENDPOINTS TAB ===================== -->
	{#if tab === 'endpoints'}

		<!-- Add/Edit Form -->
		{#if showForm}
			<div class="bg-surface rounded-xl border border-border p-5 mb-6">
				<h3 class="text-sm font-medium text-fg mb-4">{editingId ? 'Edit' : 'Add'} Webhook Endpoint</h3>

				<div class="space-y-4">
					<!-- Name -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Name</label>
						<input type="text" bind:value={name} placeholder="e.g. github-deploys"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>

					<!-- Path -->
					<div>
						<div class="flex items-center justify-between mb-1">
							<label class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Path</label>
							<button onclick={() => pathManual = !pathManual}
								class="text-[10px] text-fg-muted hover:text-fg-secondary">
								{pathManual ? 'Auto-generate' : 'Custom'}
							</button>
						</div>
						{#if pathManual}
							<input type="text" bind:value={path} placeholder="/hooks/my-endpoint"
								class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						{:else}
							<p class="px-3 py-2 text-sm font-mono bg-surface border border-border rounded-lg text-fg-secondary">{autoPath}</p>
						{/if}
					</div>

					<!-- Auth Type -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Authentication</label>
						<select bind:value={authType}
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 bg-surface">
							<option value="none">None</option>
							<option value="hmac_sha256">HMAC SHA-256</option>
							<option value="bearer">Bearer Token</option>
							<option value="header_match">Header Match</option>
						</select>

						{#if authType === 'hmac_sha256'}
							<input type="text" bind:value={authSecret} placeholder="HMAC secret"
								class="w-full mt-2 px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						{:else if authType === 'bearer'}
							<input type="text" bind:value={authToken} placeholder="Bearer token"
								class="w-full mt-2 px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						{:else if authType === 'header_match'}
							<div class="flex gap-2 mt-2">
								<input type="text" bind:value={authHeaderName} placeholder="Header name"
									class="flex-1 px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								<input type="text" bind:value={authHeaderValue} placeholder="Expected value"
									class="flex-1 px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
							</div>
						{/if}
					</div>

					<!-- Transform Type -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Transform</label>
						<select bind:value={transformType}
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 bg-surface">
							<option value="raw_json">Raw JSON (passthrough)</option>
							<option value="json_path">JSON Path (extract fields)</option>
							<option value="template">Handlebars Template</option>
							<option value="agent_prompt">Agent Prompt</option>
							<option value="skill_invocation">Skill Invocation</option>
						</select>

						{#if transformType === 'json_path'}
							<div class="grid grid-cols-2 gap-2 mt-2">
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Message path</label>
									<input type="text" bind:value={messagePath} placeholder="$.body.message"
										class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								</div>
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Title path (optional)</label>
									<input type="text" bind:value={titlePath} placeholder="$.body.title"
										class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								</div>
							</div>
						{:else if transformType === 'template'}
							<div class="mt-2">
								<label class="block text-[10px] text-fg-muted mb-0.5">Handlebars template</label>
								<textarea bind:value={templateBody} rows={4} placeholder="{{event}}: {{message}}"
									class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y"></textarea>
							</div>
						{:else if transformType === 'agent_prompt'}
							<div class="space-y-2 mt-2">
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Prompt template</label>
									<textarea bind:value={promptTemplate} rows={3} placeholder="Summarize this event: {{payload}}"
										class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y"></textarea>
								</div>
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Agent</label>
									<input type="text" bind:value={agent} placeholder="default"
										class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								</div>
							</div>
						{:else if transformType === 'skill_invocation'}
							<div class="grid grid-cols-2 gap-2 mt-2">
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Skill name</label>
									<input type="text" bind:value={skill} placeholder="e.g. solat"
										class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								</div>
								<div>
									<label class="block text-[10px] text-fg-muted mb-0.5">Input template</label>
									<input type="text" bind:value={inputTemplate} placeholder="input template JSON"
										class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
								</div>
							</div>
						{/if}
					</div>

					<!-- Targets -->
					<div>
						<div class="flex items-center justify-between mb-1">
							<label class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Targets</label>
							<button onclick={addTarget} class="text-xs text-fg-muted hover:text-fg">+ Add</button>
						</div>
						{#if targets.length === 0}
							<p class="text-xs text-fg-muted py-2">No targets configured. Add at least one delivery target.</p>
						{/if}
						{#each targets as target, i}
							<div class="flex gap-2 mb-2">
								<input type="text" bind:value={target.platform} placeholder="Platform"
									class="w-1/4 px-2 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<input type="text" bind:value={target.chat_id} placeholder="Chat ID"
									class="flex-1 px-2 py-1.5 text-xs font-mono border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<input type="text" bind:value={target.topic_id} placeholder="Topic (optional)"
									class="w-1/5 px-2 py-1.5 text-xs font-mono border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<button onclick={() => removeTarget(i)} class="text-xs text-error hover:text-error px-1">x</button>
							</div>
						{/each}
					</div>

					<!-- Rate Limit & Enabled -->
					<div class="flex items-end gap-4">
						<div class="flex-1">
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Rate Limit (req/min)</label>
							<input type="text" bind:value={rateLimit} placeholder="Optional"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
						<label class="flex items-center gap-2 pb-2 cursor-pointer">
							<input type="checkbox" bind:checked={enabled}
								class="w-4 h-4 rounded border-border text-fg focus:ring-primary-500">
							<span class="text-xs text-fg-secondary">Enabled</span>
						</label>
					</div>
				</div>

				<div class="flex gap-2 mt-5">
					<button onclick={saveEndpoint} disabled={saving || !name.trim()}
						class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
						{saving ? 'Saving...' : editingId ? 'Update' : 'Save'}
					</button>
					<button onclick={resetForm}
						class="px-4 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
						Cancel
					</button>
				</div>
			</div>
		{/if}

		<!-- Endpoints List -->
		{#if loading}
			<p class="text-sm text-fg-muted">Loading...</p>
		{:else if endpoints.length === 0 && !showForm}
			<div class="text-center py-16 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted mb-1">No webhook endpoints configured</p>
				<p class="text-xs text-fg-muted">Add endpoints to receive external events and route them to chat targets</p>
			</div>
		{:else if endpoints.length > 0}
			<div class="bg-surface rounded-xl border border-border overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-border">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Name</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Path</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Auth</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Transform</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Targets</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Enabled</th>
							<th class="px-4 py-2.5"></th>
						</tr>
					</thead>
					<tbody>
						{#each endpoints as ep}
							<tr class="border-b border-border hover:bg-[var(--color-elevated-50)]">
								<td class="px-4 py-3 font-medium text-fg">{ep.name}</td>
								<td class="px-4 py-3 font-mono text-xs text-fg-secondary">{ep.path || `/hooks/${ep.id}`}</td>
								<td class="px-4 py-3">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {authBadgeClass(ep.auth?.type || 'none')}">
										{ep.auth?.type || 'none'}
									</span>
								</td>
								<td class="px-4 py-3">
									<span class="text-xs text-fg-secondary">{ep.transform?.type || 'raw_json'}</span>
								</td>
								<td class="px-4 py-3">
									<span class="text-xs text-fg-secondary">{(ep.targets || []).length}</span>
								</td>
								<td class="px-4 py-3">
									<button onclick={() => toggleEnabled(ep)}
										class="w-8 h-4 rounded-full relative transition-colors {ep.enabled !== false ? 'bg-primary-500' : 'bg-border'}">
										<span class="absolute top-0.5 w-3 h-3 rounded-full bg-surface transition-transform {ep.enabled !== false ? 'left-4' : 'left-0.5'}"></span>
									</button>
								</td>
								<td class="px-4 py-3 text-right">
									<div class="flex gap-2 justify-end">
										<button onclick={() => editEndpoint(ep)}
											class="text-xs text-fg-muted hover:text-fg font-medium">Edit</button>
										<button onclick={() => deleteEndpoint(ep.id)}
											class="text-xs text-error hover:text-error font-medium">Delete</button>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}

	<!-- ===================== HISTORY TAB ===================== -->
	{:else}
		{#if history.length === 0}
			<div class="text-center py-16 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted mb-1">No webhook deliveries yet</p>
				<p class="text-xs text-fg-muted">History will appear here when endpoints receive requests</p>
			</div>
		{:else}
			<div class="bg-surface rounded-xl border border-border overflow-hidden">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-border">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Webhook</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Status</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Source IP</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Payload</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Error</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Duration</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Received</th>
						</tr>
					</thead>
					<tbody>
						{#each history as entry}
							<tr class="border-b border-border hover:bg-[var(--color-elevated-50)]">
								<td class="px-4 py-3 text-xs font-mono text-fg-secondary">{entry.webhook_id}</td>
								<td class="px-4 py-3">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded {statusBadgeClass(entry.status)}">
										{entry.status}
									</span>
								</td>
								<td class="px-4 py-3 text-xs text-fg-secondary font-mono">{entry.source_ip || '-'}</td>
								<td class="px-4 py-3 text-xs text-fg-muted font-mono max-w-[200px] truncate" title={entry.payload_preview}>
									{truncate(entry.payload_preview || '', 60)}
								</td>
								<td class="px-4 py-3 text-xs text-error max-w-[150px] truncate" title={entry.error}>
									{entry.error || '-'}
								</td>
								<td class="px-4 py-3 text-xs text-fg-muted">
									{entry.duration_ms != null ? `${entry.duration_ms}ms` : '-'}
								</td>
								<td class="px-4 py-3 text-xs text-fg-muted">{formatTime(entry.received_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="text-[11px] text-fg-muted mt-3">Auto-refreshes every 10 seconds</p>
		{/if}
	{/if}
</div>
