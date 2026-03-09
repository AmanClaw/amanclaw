<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

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
					enabled_skills: skills,
				});
			} else {
				await api.createCommunity({
					name: formName,
					platform: formPlatform,
					platform_group_id: formGroupId,
					zone: formZone,
					language: formLanguage,
					enabled_skills: skills,
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

<div class="p-8 max-w-4xl">
	<div class="flex items-center justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Communities</h2>
			<p class="text-sm text-gray-500 mt-1">{communities.length} connected group{communities.length !== 1 ? 's' : ''}</p>
		</div>
		<button onclick={() => { resetForm(); showForm = true; }}
			class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
			Add Community
		</button>
	</div>

	{#if showForm}
		<div class="bg-white rounded-xl border-2 border-gray-900 p-5 mb-6">
			<h3 class="text-sm font-semibold text-gray-900 mb-4">{editing ? 'Edit' : 'Add'} Community</h3>
			<div class="space-y-3">
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label class="block text-xs font-medium text-gray-700 mb-1">Name</label>
						<input type="text" bind:value={formName} placeholder="My Community"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
					<div>
						<label class="block text-xs font-medium text-gray-700 mb-1">Platform</label>
						<select bind:value={formPlatform} disabled={!!editing}
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 disabled:opacity-50">
							{#each platforms as p}
								<option value={p}>{p}</option>
							{/each}
						</select>
					</div>
				</div>
				{#if !editing}
					<div>
						<label class="block text-xs font-medium text-gray-700 mb-1">Platform Group ID</label>
						<input type="text" bind:value={formGroupId} placeholder="-100123456789"
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
					</div>
				{/if}
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label class="block text-xs font-medium text-gray-700 mb-1">JAKIM Zone</label>
						<select bind:value={formZone}
							class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
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
						<label class="block text-xs font-medium text-gray-700 mb-1">Language</label>
						<div class="flex gap-3 mt-1">
							{#each languages as lang}
								<label class="flex items-center gap-1.5 cursor-pointer">
									<input type="radio" bind:group={formLanguage} value={lang.value} class="accent-gray-900">
									<span class="text-xs text-gray-700">{lang.label}</span>
								</label>
							{/each}
						</div>
					</div>
				</div>
				<div>
					<label class="block text-xs font-medium text-gray-700 mb-1">Enabled Skills (comma-separated)</label>
					<input type="text" bind:value={formSkills} placeholder="solat, doa, quran"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>
			</div>
			<div class="flex gap-2 mt-4">
				<button onclick={handleSave}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					{editing ? 'Update' : 'Create'}
				</button>
				<button onclick={resetForm}
					class="px-4 py-1.5 text-xs font-medium rounded-md border border-gray-200 text-gray-600 hover:bg-gray-50 transition-colors">
					Cancel
				</button>
			</div>
		</div>
	{/if}

	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else if communities.length === 0}
		<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
			<p class="text-sm text-gray-500">No communities yet</p>
			<p class="text-xs text-gray-400 mt-1">Add your first community to get started</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each communities as community}
				<div class="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-200 hover:border-gray-300 transition-colors">
					<div>
						<p class="text-sm font-medium text-gray-900">{community.name}</p>
						<p class="text-xs text-gray-500 mt-0.5">
							{community.platform} · {community.zone} · {community.language}
						</p>
					</div>
					<div class="flex items-center gap-3">
						<span class="text-xs text-gray-400">{community.enabled_skills?.length || 0} skills</span>
						<button onclick={() => startEdit(community)}
							class="text-xs text-gray-500 hover:text-gray-900 font-medium">Edit</button>
						{#if confirmDelete === community.id}
							<button onclick={() => handleDelete(community.id)}
								class="text-xs text-red-600 font-medium">Confirm</button>
							<button onclick={() => confirmDelete = null}
								class="text-xs text-gray-400">Cancel</button>
						{:else}
							<button onclick={() => confirmDelete = community.id}
								class="text-xs text-red-400 hover:text-red-600">Delete</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
