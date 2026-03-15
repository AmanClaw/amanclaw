<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';
	import { Plus, Users, Edit3, Trash2 } from '@amanclaw/ui';

	let communities: any[] = $state([]);
	let loading = $state(true);
	let showForm = $state(false);
	let editing = $state<any>(null);
	let confirmDelete = $state<string | null>(null);

	// Form fields
	let formName = $state('');
	let formPlatform = $state('telegram');
	let formGroupId = $state('');
	let formZone = $state('WP01');
	let formLanguage = $state('bm');
	let formSkills = $state('');

	const platforms = ['telegram', 'discord', 'whatsapp', 'slack'];
	const languages = [
		{ value: 'bm', label: 'Bahasa Melayu' },
		{ value: 'en', label: 'English' },
		{ value: 'rojak', label: 'Rojak' },
	];
	const zones = [
		{ group: 'WP & Selangor', items: ['WP01', 'WP02', 'WP03', 'SGR01', 'SGR02', 'SGR03'] },
		{ group: 'Johor', items: ['JHR01', 'JHR02', 'JHR03', 'JHR04'] },
		{ group: 'Kedah', items: ['KDH01', 'KDH02', 'KDH03', 'KDH04', 'KDH05', 'KDH06', 'KDH07'] },
		{ group: 'Kelantan', items: ['KTN01', 'KTN02'] },
		{ group: 'Melaka', items: ['MLK01'] },
		{ group: 'N. Sembilan', items: ['NGS01', 'NGS02'] },
		{ group: 'Pahang', items: ['PHG01', 'PHG02', 'PHG03', 'PHG04', 'PHG05', 'PHG06'] },
		{ group: 'Perak', items: ['PRK01', 'PRK02', 'PRK03', 'PRK04', 'PRK05', 'PRK06', 'PRK07'] },
		{ group: 'Perlis', items: ['PLS01'] },
		{ group: 'Pulau Pinang', items: ['PNG01'] },
		{ group: 'Sabah', items: ['SBH01', 'SBH02', 'SBH03', 'SBH04', 'SBH05', 'SBH06', 'SBH07', 'SBH08', 'SBH09'] },
		{ group: 'Sarawak', items: ['SWK01', 'SWK02', 'SWK03', 'SWK04', 'SWK05', 'SWK06', 'SWK07', 'SWK08', 'SWK09'] },
		{ group: 'Terengganu', items: ['TRG01', 'TRG02', 'TRG03', 'TRG04'] },
	];

	function resetForm() {
		formName = '';
		formPlatform = 'telegram';
		formGroupId = '';
		formZone = 'WP01';
		formLanguage = 'bm';
		formSkills = '';
		editing = null;
		showForm = false;
	}

	function startEdit(c: any) {
		editing = c;
		formName = c.name;
		formPlatform = c.platform;
		formGroupId = c.platform_group_id || '';
		formZone = c.zone || 'WP01';
		formLanguage = c.language || 'bm';
		formSkills = (c.enabled_skills || []).join(', ');
		showForm = true;
	}

	async function handleSave() {
		const skills = formSkills.split(',').map((s: string) => s.trim()).filter(Boolean);
		try {
			if (editing) {
				await api.updateCommunity({
					id: editing.id,
					name: formName,
					zone: formZone,
					language: formLanguage,
					enabledSkills: skills,
				});
			} else {
				await api.createCommunity({
					name: formName,
					platform: formPlatform,
					platformGroupId: formGroupId,
					zone: formZone,
					language: formLanguage,
					enabledSkills: skills,
				});
			}
			resetForm();
			await loadCommunities();
		} catch (_) {}
	}

	async function handleDelete(id: string) {
		try {
			await api.deleteCommunity(id);
			confirmDelete = null;
			await loadCommunities();
		} catch (_) {}
	}

	async function loadCommunities() {
		try {
			const data = await api.getCommunities() as any;
			communities = data.communities || [];
		} catch (_) {}
		loading = false;
	}

	onMount(() => {
		loadCommunities();
	});
</script>

<div class="max-w-4xl">
	<PageHeader title="Communities" subtitle="{communities.length} connected group{communities.length !== 1 ? 's' : ''}">
		{#snippet action()}
			<Button size="sm" onclick={() => { resetForm(); showForm = true; }}>
				<Plus size={14} />
				Add Community
			</Button>
		{/snippet}
	</PageHeader>

	{#if showForm}
		<Card class="mb-6 !border-2 !border-primary-500">
			<h3 class="text-sm font-semibold text-fg mb-4">{editing ? 'Edit' : 'Add'} Community</h3>
			<div class="space-y-3">
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label class="block text-xs font-medium text-fg-secondary mb-1">Name</label>
						<input type="text" bind:value={formName} placeholder="My Community"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
					<div>
						<label class="block text-xs font-medium text-fg-secondary mb-1">Platform</label>
						<select bind:value={formPlatform} disabled={!!editing}
							class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500 disabled:opacity-50">
							{#each platforms as p}
								<option value={p}>{p}</option>
							{/each}
						</select>
					</div>
				</div>
				{#if !editing}
					<div>
						<label class="block text-xs font-medium text-fg-secondary mb-1">Platform Group ID</label>
						<input type="text" bind:value={formGroupId} placeholder="-100123456789"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
				{/if}
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label class="block text-xs font-medium text-fg-secondary mb-1">JAKIM Zone</label>
						<select bind:value={formZone}
							class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
							{#each zones as group}
								<optgroup label={group.group}>
									{#each group.items as z}
										<option value={z}>{z}</option>
									{/each}
								</optgroup>
							{/each}
						</select>
					</div>
					<div>
						<label class="block text-xs font-medium text-fg-secondary mb-1">Language</label>
						<div class="flex gap-3 mt-1">
							{#each languages as lang}
								<label class="flex items-center gap-1.5 cursor-pointer">
									<input type="radio" bind:group={formLanguage} value={lang.value} class="accent-primary-500">
									<span class="text-xs text-fg-secondary">{lang.label}</span>
								</label>
							{/each}
						</div>
					</div>
				</div>
				<div>
					<label class="block text-xs font-medium text-fg-secondary mb-1">Enabled Skills (comma-separated)</label>
					<input type="text" bind:value={formSkills} placeholder="solat, doa, quran"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-elevated text-fg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>
			</div>
			<div class="flex gap-2 mt-4">
				<Button size="sm" onclick={handleSave}>
					{editing ? 'Update' : 'Create'}
				</Button>
				<Button variant="secondary" size="sm" onclick={resetForm}>
					Cancel
				</Button>
			</div>
		</Card>
	{/if}

	{#if loading}
		<p class="text-sm text-fg-muted">Loading...</p>
	{:else if communities.length === 0}
		<Card>
			<EmptyState icon={Users} title="No communities yet" description="Add your first community to get started" />
		</Card>
	{:else}
		<div class="space-y-2">
			{#each communities as community}
				<div class="flex items-center justify-between p-4 bg-base rounded-xl border border-border hover:border-[var(--color-primary-500-10)] transition-colors">
					<div>
						<p class="text-[13px] font-medium text-fg">{community.name}</p>
						<p class="text-xs text-fg-muted mt-0.5">
							{community.platform} · {community.zone} · {community.language}
						</p>
					</div>
					<div class="flex items-center gap-3">
						<span class="text-xs text-fg-muted">{community.enabled_skills?.length || 0} skills</span>
						<button onclick={() => startEdit(community)}
							class="text-xs text-fg-secondary hover:text-fg font-medium">Edit</button>
						{#if confirmDelete === community.id}
							<button onclick={() => handleDelete(community.id)}
								class="text-xs text-error font-medium">Confirm</button>
							<button onclick={() => confirmDelete = null}
								class="text-xs text-fg-muted">Cancel</button>
						{:else}
							<button onclick={() => confirmDelete = community.id}
								class="text-xs text-error/70 hover:text-error">Delete</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
