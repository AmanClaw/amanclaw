<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	let activeTab = $state('doa');
	let loading = $state(false);

	// Doa state
	let doas: any[] = $state([]);
	let doaCategory = $state('');
	let doaSearch = $state('');
	const categories = [
		{ value: '', label: 'All' },
		{ value: 'harian', label: 'Harian (Daily)' },
		{ value: 'pagi', label: 'Pagi (Morning)' },
		{ value: 'petang', label: 'Petang (Evening)' },
		{ value: 'solat', label: 'Solat (Prayer)' },
		{ value: 'musafir', label: 'Musafir (Travel)' },
		{ value: 'makan', label: 'Makan (Eating)' },
		{ value: 'tidur', label: 'Tidur (Sleep)' },
		{ value: 'wudhu', label: 'Wudhu' },
		{ value: 'masjid', label: 'Masjid (Mosque)' },
	];

	// Zakat state
	let zakatRates: any = $state(null);

	// Khutbah state
	let khutbah: any = $state(null);
	let fetchingKhutbah = $state(false);

	async function loadDoas() {
		loading = true;
		try {
			if (doaSearch.trim()) {
				const data = await api.searchDoa(doaSearch) as any;
				doas = data.doas || [];
			} else {
				const data = await api.getDoaCollection(doaCategory || undefined) as any;
				doas = data.doas || [];
			}
		} catch (_) {}
		loading = false;
	}

	async function loadZakat() {
		try {
			zakatRates = await api.getZakatRates();
		} catch (_) {}
	}

	async function fetchKhutbah() {
		fetchingKhutbah = true;
		try {
			khutbah = await api.getLatestKhutbah();
		} catch (_) {}
		fetchingKhutbah = false;
	}

	onMount(() => {
		loadDoas();
	});

	// Reload doas when category changes
	$effect(() => {
		doaCategory;
		if (activeTab === 'doa' && !doaSearch.trim()) loadDoas();
	});
</script>

<div class="max-w-4xl">
	<PageHeader title="Content" subtitle="Browse Islamic content and reference data" />

	<div class="flex gap-1 mb-6 bg-elevated p-1 rounded-lg w-fit">
		{#each ['doa', 'zakat', 'khutbah'] as tab}
			<button
				class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
					{activeTab === tab ? 'bg-surface text-fg shadow-sm' : 'text-fg-secondary hover:text-fg'}"
				onclick={() => { activeTab = tab; if (tab === 'zakat' && !zakatRates) loadZakat(); }}
			>
				{tab.charAt(0).toUpperCase() + tab.slice(1)}
			</button>
		{/each}
	</div>

	{#if activeTab === 'doa'}
		<div class="flex items-center gap-3 mb-4">
			<select bind:value={doaCategory} onchange={() => { doaSearch = ''; loadDoas(); }}
				class="px-3 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500">
				{#each categories as cat}
					<option value={cat.value}>{cat.label}</option>
				{/each}
			</select>
			<div class="flex gap-1 flex-1">
				<input type="text" bind:value={doaSearch} placeholder="Search doas..."
					class="flex-1 px-3 py-1.5 text-xs border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
					onkeydown={(e) => { if (e.key === 'Enter') loadDoas(); }}>
				<button onclick={loadDoas}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600">
					Search
				</button>
			</div>
			<span class="text-xs text-fg-muted">{doas.length} result{doas.length !== 1 ? 's' : ''}</span>
		</div>

		{#if loading}
			<p class="text-sm text-fg-muted">Loading...</p>
		{:else if doas.length === 0}
			<div class="text-center py-12 bg-surface rounded-xl border border-border">
				<p class="text-sm text-fg-muted">No doas found</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each doas as doa}
					<div class="bg-surface rounded-xl border border-border p-5">
						<div class="flex items-start justify-between mb-3">
							<div>
								<p class="text-sm font-medium text-fg">{doa.title_ms}</p>
								<p class="text-xs text-fg-muted">{doa.title_en}</p>
							</div>
							<span class="text-[10px] font-medium px-2 py-0.5 rounded bg-elevated text-fg-secondary">{doa.category}</span>
						</div>
						<p class="text-xl text-right leading-loose text-fg font-serif mb-3" dir="rtl">{doa.arabic}</p>
						<p class="text-xs text-fg-muted italic mb-2">{doa.transliteration}</p>
						<div class="grid grid-cols-2 gap-3 text-xs">
							<div>
								<span class="text-fg-muted block mb-0.5">BM</span>
								<p class="text-fg-secondary">{doa.translation_ms}</p>
							</div>
							<div>
								<span class="text-fg-muted block mb-0.5">EN</span>
								<p class="text-fg-secondary">{doa.translation_en}</p>
							</div>
						</div>
						{#if doa.source}
							<p class="text-[10px] text-fg-muted mt-2">{doa.source}</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

	{:else if activeTab === 'zakat'}
		<div class="bg-surface rounded-xl border border-border p-5">
			<p class="text-sm font-medium text-fg mb-4">Zakat Fitrah Rates</p>
			{#if zakatRates}
				<div class="bg-surface rounded-lg border border-border p-4 mb-3">
					<div class="grid grid-cols-3 gap-4 text-xs">
						<div>
							<span class="text-fg-muted block">Rate</span>
							<span class="text-lg font-semibold text-fg">RM {zakatRates.fitrah?.rate?.toFixed(2)}</span>
						</div>
						<div>
							<span class="text-fg-muted block">Currency</span>
							<span class="text-fg-secondary">{zakatRates.fitrah?.currency}</span>
						</div>
						<div>
							<span class="text-fg-muted block">Year</span>
							<span class="text-fg-secondary">{zakatRates.fitrah?.year}</span>
						</div>
					</div>
				</div>
				<p class="text-xs text-fg-muted">{zakatRates.note}</p>
			{:else}
				<p class="text-xs text-fg-muted">Loading rates...</p>
			{/if}
		</div>

	{:else if activeTab === 'khutbah'}
		<div class="bg-surface rounded-xl border border-border p-5">
			<div class="flex items-center justify-between mb-4">
				<p class="text-sm font-medium text-fg">Latest Khutbah</p>
				<button onclick={fetchKhutbah} disabled={fetchingKhutbah}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 disabled:opacity-50">
					{fetchingKhutbah ? 'Fetching...' : 'Fetch Latest'}
				</button>
			</div>
			{#if khutbah}
				{#if khutbah.available}
					<div class="bg-surface rounded-lg border border-border p-4">
						<p class="text-sm text-fg">{khutbah.title || 'Untitled'}</p>
						<p class="text-xs text-fg-muted mt-2">{khutbah.content || ''}</p>
					</div>
				{:else}
					<p class="text-xs text-fg-muted">{khutbah.note}</p>
				{/if}
			{:else}
				<p class="text-xs text-fg-muted">Click "Fetch Latest" to load khutbah data.</p>
			{/if}
		</div>
	{/if}
</div>
