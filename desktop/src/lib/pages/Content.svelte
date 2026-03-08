<script lang="ts">
	let activeTab = $state('doa');
</script>

<div class="p-8 max-w-4xl">
	<div class="mb-6">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Content</h2>
		<p class="text-sm text-gray-500 mt-1">Manage Islamic content and data</p>
	</div>

	<div class="flex gap-1 mb-6 bg-gray-100 p-1 rounded-lg w-fit">
		{#each ['doa', 'zakat', 'khutbah'] as tab}
			<button
				class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
					{activeTab === tab ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
				onclick={() => activeTab = tab}
			>
				{tab.charAt(0).toUpperCase() + tab.slice(1)}
			</button>
		{/each}
	</div>

	{#if activeTab === 'doa'}
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<div class="flex items-center justify-between mb-4">
				<p class="text-sm font-medium text-gray-900">Doa Collection</p>
				<button class="text-xs text-gray-500 hover:text-gray-900">Add Doa</button>
			</div>
			<p class="text-xs text-gray-500">20 doas across 9 categories. Edit via the collection manager.</p>
		</div>
	{:else if activeTab === 'zakat'}
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<p class="text-sm font-medium text-gray-900 mb-4">Zakat Fitrah Rates (2026)</p>
			<p class="text-xs text-gray-500">Update yearly rates per state from JAKIM.</p>
		</div>
	{:else if activeTab === 'khutbah'}
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<p class="text-sm font-medium text-gray-900 mb-4">Khutbah Cache</p>
			<p class="text-xs text-gray-500">Cached weekly khutbah from JAKIM portal.</p>
		</div>
	{/if}
</div>
