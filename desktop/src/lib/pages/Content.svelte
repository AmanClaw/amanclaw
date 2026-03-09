<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

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

<div class="p-8 max-w-4xl">
	<div class="mb-6">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Content</h2>
		<p class="text-sm text-gray-500 mt-1">Browse Islamic content and reference data</p>
	</div>

	<div class="flex gap-1 mb-6 bg-gray-100 p-1 rounded-lg w-fit">
		{#each ['doa', 'zakat', 'khutbah'] as tab}
			<button
				class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
					{activeTab === tab ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
				onclick={() => { activeTab = tab; if (tab === 'zakat' && !zakatRates) loadZakat(); }}
			>
				{tab.charAt(0).toUpperCase() + tab.slice(1)}
			</button>
		{/each}
	</div>

	{#if activeTab === 'doa'}
		<div class="flex items-center gap-3 mb-4">
			<select bind:value={doaCategory} onchange={() => { doaSearch = ''; loadDoas(); }}
				class="px-3 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900">
				{#each categories as cat}
					<option value={cat.value}>{cat.label}</option>
				{/each}
			</select>
			<div class="flex gap-1 flex-1">
				<input type="text" bind:value={doaSearch} placeholder="Search doas..."
					class="flex-1 px-3 py-1.5 text-xs border border-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-gray-900"
					onkeydown={(e) => { if (e.key === 'Enter') loadDoas(); }}>
				<button onclick={loadDoas}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800">
					Search
				</button>
			</div>
			<span class="text-xs text-gray-400">{doas.length} result{doas.length !== 1 ? 's' : ''}</span>
		</div>

		{#if loading}
			<p class="text-sm text-gray-500">Loading...</p>
		{:else if doas.length === 0}
			<div class="text-center py-12 bg-gray-50 rounded-xl border border-gray-200">
				<p class="text-sm text-gray-500">No doas found</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each doas as doa}
					<div class="bg-white rounded-xl border border-gray-200 p-5">
						<div class="flex items-start justify-between mb-3">
							<div>
								<p class="text-sm font-medium text-gray-900">{doa.title_ms}</p>
								<p class="text-xs text-gray-500">{doa.title_en}</p>
							</div>
							<span class="text-[10px] font-medium px-2 py-0.5 rounded bg-gray-100 text-gray-600">{doa.category}</span>
						</div>
						<p class="text-xl text-right leading-loose text-gray-800 font-serif mb-3" dir="rtl">{doa.arabic}</p>
						<p class="text-xs text-gray-500 italic mb-2">{doa.transliteration}</p>
						<div class="grid grid-cols-2 gap-3 text-xs">
							<div>
								<span class="text-gray-400 block mb-0.5">BM</span>
								<p class="text-gray-700">{doa.translation_ms}</p>
							</div>
							<div>
								<span class="text-gray-400 block mb-0.5">EN</span>
								<p class="text-gray-700">{doa.translation_en}</p>
							</div>
						</div>
						{#if doa.source}
							<p class="text-[10px] text-gray-400 mt-2">{doa.source}</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

	{:else if activeTab === 'zakat'}
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<p class="text-sm font-medium text-gray-900 mb-4">Zakat Fitrah Rates</p>
			{#if zakatRates}
				<div class="bg-white rounded-lg border border-gray-200 p-4 mb-3">
					<div class="grid grid-cols-3 gap-4 text-xs">
						<div>
							<span class="text-gray-400 block">Rate</span>
							<span class="text-lg font-semibold text-gray-900">RM {zakatRates.fitrah?.rate?.toFixed(2)}</span>
						</div>
						<div>
							<span class="text-gray-400 block">Currency</span>
							<span class="text-gray-700">{zakatRates.fitrah?.currency}</span>
						</div>
						<div>
							<span class="text-gray-400 block">Year</span>
							<span class="text-gray-700">{zakatRates.fitrah?.year}</span>
						</div>
					</div>
				</div>
				<p class="text-xs text-gray-500">{zakatRates.note}</p>
			{:else}
				<p class="text-xs text-gray-500">Loading rates...</p>
			{/if}
		</div>

	{:else if activeTab === 'khutbah'}
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<div class="flex items-center justify-between mb-4">
				<p class="text-sm font-medium text-gray-900">Latest Khutbah</p>
				<button onclick={fetchKhutbah} disabled={fetchingKhutbah}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 disabled:opacity-50">
					{fetchingKhutbah ? 'Fetching...' : 'Fetch Latest'}
				</button>
			</div>
			{#if khutbah}
				{#if khutbah.available}
					<div class="bg-white rounded-lg border border-gray-200 p-4">
						<p class="text-sm text-gray-900">{khutbah.title || 'Untitled'}</p>
						<p class="text-xs text-gray-500 mt-2">{khutbah.content || ''}</p>
					</div>
				{:else}
					<p class="text-xs text-gray-500">{khutbah.note}</p>
				{/if}
			{:else}
				<p class="text-xs text-gray-500">Click "Fetch Latest" to load khutbah data.</p>
			{/if}
		</div>
	{/if}
</div>
