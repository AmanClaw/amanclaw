<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	// --- State ---
	let tab = $state<'jobs' | 'history'>('jobs');
	let jobs: any[] = $state([]);
	let history: any[] = $state([]);
	let timezone = $state('');
	let loading = $state(true);
	let showForm = $state(false);
	let saving = $state(false);
	let editingId = $state<string | null>(null);

	// Form fields
	let jobName = $state('');
	let schedule = $state('');
	let schedulePreset = $state('custom');
	let jobType = $state<'direct_message' | 'skill_invocation' | 'agent_prompt'>('direct_message');
	let template = $state('');
	let skillName = $state('');
	let skillInput = $state('{}');
	let agentName = $state('');
	let agentPrompt = $state('');
	let targets = $state<{ platform: string; chat_id: string; topic_id: string }[]>([]);
	let agentOverride = $state('');
	let enabled = $state(true);

	const presets: { label: string; value: string; cron: string }[] = [
		{ label: 'Hourly', value: 'hourly', cron: '0 * * * *' },
		{ label: 'Daily 6am', value: 'daily', cron: '0 6 * * *' },
		{ label: 'Weekly Friday', value: 'weekly', cron: '0 6 * * 5' },
		{ label: 'Custom', value: 'custom', cron: '' },
	];

	// --- Derived ---
	const jobCount = $derived(jobs.length);

	// --- Form helpers ---
	function resetForm() {
		jobName = '';
		schedule = '';
		schedulePreset = 'custom';
		jobType = 'direct_message';
		template = '';
		skillName = '';
		skillInput = '{}';
		agentName = '';
		agentPrompt = '';
		targets = [];
		agentOverride = '';
		enabled = true;
		editingId = null;
		showForm = false;
	}

	function onPresetChange() {
		const preset = presets.find(p => p.value === schedulePreset);
		if (preset && preset.cron) {
			schedule = preset.cron;
		}
	}

	function addTarget() {
		targets = [...targets, { platform: '', chat_id: '', topic_id: '' }];
	}

	function removeTarget(index: number) {
		targets = targets.filter((_, i) => i !== index);
	}

	function editJob(job: any) {
		editingId = job.id;
		jobName = job.name || '';
		schedule = job.schedule || '';
		jobType = job.type || 'direct_message';
		template = job.template || '';
		skillName = job.skill_name || '';
		skillInput = job.skill_input ? JSON.stringify(job.skill_input, null, 2) : '{}';
		agentName = job.agent_name || '';
		agentPrompt = job.prompt || '';
		targets = (job.targets || []).map((t: any) => ({
			platform: t.platform || '',
			chat_id: t.chat_id || '',
			topic_id: t.topic_id || '',
		}));
		agentOverride = job.agent_override || '';
		enabled = job.enabled !== false;

		// Match preset
		const matched = presets.find(p => p.cron === schedule);
		schedulePreset = matched ? matched.value : 'custom';

		showForm = true;
	}

	function buildJobPayload() {
		const base: any = {
			name: jobName.trim(),
			schedule: schedule.trim(),
			type: jobType,
			targets: targets.filter(t => t.platform.trim() || t.chat_id.trim()),
			enabled,
		};

		if (agentOverride.trim()) base.agent_override = agentOverride.trim();

		if (jobType === 'direct_message') {
			base.template = template;
		} else if (jobType === 'skill_invocation') {
			base.skill_name = skillName.trim();
			try { base.skill_input = JSON.parse(skillInput); } catch { base.skill_input = {}; }
		} else if (jobType === 'agent_prompt') {
			base.agent_name = agentName.trim();
			base.prompt = agentPrompt;
		}

		return base;
	}

	async function saveJob() {
		if (!jobName.trim() || !schedule.trim()) return;
		saving = true;
		try {
			const payload = buildJobPayload();
			await api.saveCronJob(editingId, payload);
			resetForm();
			await loadJobs();
		} catch (_) {}
		saving = false;
	}

	async function deleteJob(id: string) {
		try {
			await api.deleteCronJob(id);
			await loadJobs();
		} catch (_) {}
	}

	async function toggleJob(job: any) {
		try {
			await api.saveCronJob(job.id, { ...job, enabled: !job.enabled });
			await loadJobs();
		} catch (_) {}
	}

	// --- Data loading ---
	async function loadJobs() {
		try {
			const data = await api.listCronJobs() as any;
			jobs = data.jobs || [];
			timezone = data.timezone || '';
		} catch (_) {}
		loading = false;
	}

	async function loadHistory() {
		try {
			const data = await api.getCronHistory() as any;
			history = data.entries || [];
		} catch (_) {}
	}

	function truncate(text: string, max: number): string {
		if (!text) return '-';
		return text.length > max ? text.slice(0, max) + '...' : text;
	}

	function formatDate(iso: string): string {
		if (!iso) return '-';
		try {
			const d = new Date(iso);
			return d.toLocaleString();
		} catch { return iso; }
	}

	onMount(() => {
		loadJobs();
		loadHistory();
		const interval = setInterval(() => {
			if (tab === 'history') loadHistory();
		}, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="max-w-4xl">
	<PageHeader title="Cron Jobs" subtitle={tab === 'jobs' ? `${jobCount} job${jobCount !== 1 ? 's' : ''}${timezone ? ` - ${timezone}` : ''}` : 'Execution history'}>
		{#snippet action()}
			{#if tab === 'jobs' && !showForm}
				<Button size="sm" onclick={() => { resetForm(); showForm = true; }}>Add Job</Button>
			{/if}
		{/snippet}
	</PageHeader>

	<!-- Tabs -->
	<div class="flex gap-1 mb-6 bg-elevated rounded-lg p-0.5 w-fit">
		<button onclick={() => tab = 'jobs'}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'jobs' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			Jobs
		</button>
		<button onclick={() => { tab = 'history'; loadHistory(); }}
			class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors
				{tab === 'history' ? 'bg-surface text-fg shadow-sm' : 'text-fg-muted hover:text-fg-secondary'}">
			History
		</button>
	</div>

	<!-- ===================== JOBS TAB ===================== -->
	{#if tab === 'jobs'}
		<!-- Inline form -->
		{#if showForm}
			<div class="bg-surface rounded-xl border border-border p-5 mb-6">
				<h3 class="text-sm font-medium text-fg mb-4">{editingId ? 'Edit' : 'Add'} Cron Job</h3>

				<div class="space-y-4">
					<!-- Name -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Name</label>
						<input type="text" bind:value={jobName} placeholder="e.g. daily-reminder"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>

					<!-- Schedule -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Schedule</label>
						<div class="flex gap-2 mb-2">
							{#each presets as preset}
								<button onclick={() => { schedulePreset = preset.value; onPresetChange(); }}
									class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
										{schedulePreset === preset.value ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
									{preset.label}
								</button>
							{/each}
						</div>
						<input type="text" bind:value={schedule} placeholder="* * * * *"
							class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						<p class="text-[10px] text-fg-muted mt-1">min hour day month weekday</p>
					</div>

					<!-- Type -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Type</label>
						<div class="flex gap-2">
							<button onclick={() => jobType = 'direct_message'}
								class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
									{jobType === 'direct_message' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
								Direct Message
							</button>
							<button onclick={() => jobType = 'skill_invocation'}
								class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
									{jobType === 'skill_invocation' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
								Skill Invocation
							</button>
							<button onclick={() => jobType = 'agent_prompt'}
								class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
									{jobType === 'agent_prompt' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
								Agent Prompt
							</button>
						</div>
					</div>

					<!-- Type-specific fields -->
					{#if jobType === 'direct_message'}
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Message Template</label>
							<textarea bind:value={template} rows={3} placeholder="Message to send..."
								class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y"></textarea>
						</div>
					{:else if jobType === 'skill_invocation'}
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Skill Name</label>
							<input type="text" bind:value={skillName} placeholder="e.g. solat"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Input JSON</label>
							<textarea bind:value={skillInput} rows={3} placeholder="JSON input for skill"
								class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y"></textarea>
						</div>
					{:else if jobType === 'agent_prompt'}
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Agent Name</label>
							<input type="text" bind:value={agentName} placeholder="e.g. default"
								class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						</div>
						<div>
							<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Prompt</label>
							<textarea bind:value={agentPrompt} rows={3} placeholder="Prompt to send to the agent..."
								class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-y"></textarea>
						</div>
					{/if}

					<!-- Targets -->
					<div>
						<div class="flex items-center justify-between mb-1">
							<label class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Targets</label>
							<button onclick={addTarget} class="text-xs text-fg-muted hover:text-fg">+ Add</button>
						</div>
						{#if targets.length === 0}
							<p class="text-xs text-fg-muted">No targets. Click + Add to add a target.</p>
						{/if}
						{#each targets as target, i}
							<div class="flex gap-2 mb-2">
								<input type="text" bind:value={target.platform} placeholder="platform"
									class="w-1/4 px-2 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<input type="text" bind:value={target.chat_id} placeholder="chat_id"
									class="flex-1 px-2 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<input type="text" bind:value={target.topic_id} placeholder="topic_id"
									class="w-1/4 px-2 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
								<button onclick={() => removeTarget(i)} class="text-xs text-error hover:text-error px-1">x</button>
							</div>
						{/each}
					</div>

					<!-- Agent override -->
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">
							Agent Override <span class="normal-case text-fg-muted">(optional)</span>
						</label>
						<input type="text" bind:value={agentOverride} placeholder="Override agent for this job"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>

					<!-- Enabled toggle -->
					<div class="flex items-center gap-2">
						<button onclick={() => enabled = !enabled}
							class="relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors cursor-pointer
								{enabled ? 'bg-primary-500' : 'bg-border'}">
							<span class="inline-block h-4 w-4 rounded-full bg-surface transition-transform
								{enabled ? 'translate-x-4' : 'translate-x-0'}"></span>
						</button>
						<span class="text-xs text-fg-secondary">{enabled ? 'Enabled' : 'Disabled'}</span>
					</div>
				</div>

				<div class="flex gap-2 mt-5">
					<button onclick={saveJob} disabled={saving || !jobName.trim() || !schedule.trim()}
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

		<!-- Jobs table -->
		{#if loading}
			<p class="text-sm text-fg-muted">Loading...</p>
		{:else if jobs.length === 0 && !showForm}
			<div class="text-center py-16 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted mb-1">No cron jobs configured</p>
				<p class="text-xs text-fg-muted">Schedule recurring tasks for your bot</p>
			</div>
		{:else if jobs.length > 0}
			<div class="bg-surface rounded-xl border border-border overflow-hidden">
				<table class="w-full text-xs">
					<thead>
						<tr class="border-b border-border">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Name</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Schedule</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Type</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Targets</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Enabled</th>
							<th class="text-right px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each jobs as job}
							<tr class="border-b border-border hover:bg-[var(--color-elevated-50)] transition-colors">
								<td class="px-4 py-2.5 text-sm font-medium text-fg">{job.name}</td>
								<td class="px-4 py-2.5 font-mono text-fg-secondary">{job.schedule}</td>
								<td class="px-4 py-2.5">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded
										{job.type === 'direct_message' ? 'bg-[var(--color-info-15)] text-info' :
										 job.type === 'skill_invocation' ? 'bg-[var(--color-accent-500-15)] text-[var(--color-accent-500)]' :
										 'bg-[var(--color-warning-15)] text-warning'}">
										{job.type.replace('_', ' ')}
									</span>
								</td>
								<td class="px-4 py-2.5 text-fg-secondary">{(job.targets || []).length}</td>
								<td class="px-4 py-2.5">
									<button onclick={() => toggleJob(job)}
										class="relative inline-flex h-4 w-7 shrink-0 rounded-full border-2 border-transparent transition-colors cursor-pointer
											{job.enabled ? 'bg-primary-500' : 'bg-border'}">
										<span class="inline-block h-3 w-3 rounded-full bg-surface transition-transform
											{job.enabled ? 'translate-x-3' : 'translate-x-0'}"></span>
									</button>
								</td>
								<td class="px-4 py-2.5 text-right">
									<button onclick={() => editJob(job)}
										class="text-xs text-fg-muted hover:text-fg font-medium mr-2">Edit</button>
									<button onclick={() => deleteJob(job.id)}
										class="text-xs text-error hover:text-error font-medium">Delete</button>
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
				<p class="text-sm text-fg-muted">No execution history yet</p>
				<p class="text-xs text-fg-muted mt-1">History will appear here once jobs start running</p>
			</div>
		{:else}
			<div class="bg-surface rounded-xl border border-border overflow-hidden">
				<table class="w-full text-xs">
					<thead>
						<tr class="border-b border-border">
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Job</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Status</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Output</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Duration</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase">Executed At</th>
						</tr>
					</thead>
					<tbody>
						{#each history as entry}
							<tr class="border-b border-border hover:bg-[var(--color-elevated-50)] transition-colors">
								<td class="px-4 py-2.5 text-sm font-medium text-fg">{entry.job_id}</td>
								<td class="px-4 py-2.5">
									<span class="inline-flex px-1.5 py-0.5 text-[10px] font-medium rounded
										{entry.status === 'success' ? 'bg-[var(--color-success-15)] text-success' :
										 entry.status === 'failed' ? 'bg-[var(--color-error-15)] text-error' :
										 'bg-elevated text-fg-secondary'}">
										{entry.status}
									</span>
								</td>
								<td class="px-4 py-2.5 text-fg-secondary font-mono max-w-xs truncate" title={entry.output}>
									{truncate(entry.output, 60)}
								</td>
								<td class="px-4 py-2.5 text-fg-secondary">
									{entry.duration_ms != null ? `${entry.duration_ms}ms` : '-'}
								</td>
								<td class="px-4 py-2.5 text-fg-muted">{formatDate(entry.executed_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{/if}
</div>
