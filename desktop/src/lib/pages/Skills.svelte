<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let skills: any[] = $state([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const data = await api.getSkills() as any;
			skills = data.skills || [];
		} catch (e) {
			// Not connected
		}
		loading = false;
	});
</script>

<div class="p-8 max-w-4xl">
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Skills</h2>
		<p class="text-sm text-gray-500 mt-1">Manage bot capabilities</p>
	</div>

	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else if skills.length === 0}
		<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
			<p class="text-sm text-gray-500">No skills registered</p>
			<p class="text-xs text-gray-400 mt-1">Connect to a bot instance to see skills</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each skills as skill}
				<div class="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-200">
					<div>
						<p class="text-sm font-medium text-gray-900">{skill.name}</p>
						<p class="text-xs text-gray-500 mt-0.5">{skill.description}</p>
					</div>
					<label class="relative inline-flex items-center cursor-pointer">
						<input type="checkbox" checked class="sr-only peer">
						<div class="w-9 h-5 bg-gray-300 peer-checked:bg-gray-900 rounded-full transition-colors
							after:content-[''] after:absolute after:top-[2px] after:start-[2px]
							after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all
							peer-checked:after:translate-x-full"></div>
					</label>
				</div>
			{/each}
		</div>
	{/if}
</div>
