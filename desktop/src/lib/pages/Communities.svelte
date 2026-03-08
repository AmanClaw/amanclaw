<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let communities: any[] = $state([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const data = await api.getCommunities() as any;
			communities = data.communities || [];
		} catch (e) {
			// Not connected
		}
		loading = false;
	});
</script>

<div class="p-8 max-w-4xl">
	<div class="flex items-center justify-between mb-8">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Communities</h2>
			<p class="text-sm text-gray-500 mt-1">Manage your connected groups</p>
		</div>
		<button class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
			Add Community
		</button>
	</div>

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
					<div class="flex items-center gap-2">
						<span class="text-xs text-gray-400">{community.enabled_skills?.length || 0} skills</span>
						<button class="text-xs text-gray-500 hover:text-gray-900">Edit</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
