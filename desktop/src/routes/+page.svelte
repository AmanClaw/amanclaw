<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { botStatus, currentPage } from '$lib/stores/app';
	import Communities from '$lib/pages/Communities.svelte';
	import Skills from '$lib/pages/Skills.svelte';
	import Users from '$lib/pages/Users.svelte';
	import Settings from '$lib/pages/Settings.svelte';

	onMount(async () => {
		try {
			const status = await api.getStatus();
			botStatus.set(status as any);
		} catch (e) {
			// Not connected yet
		}
	});
</script>

{#if $currentPage === 'communities'}
	<Communities />
{:else if $currentPage === 'skills'}
	<Skills />
{:else if $currentPage === 'users'}
	<Users />
{:else if $currentPage === 'settings'}
	<Settings />
{:else}
	<!-- Dashboard -->
	<div class="p-8 max-w-4xl">
		<div class="mb-8">
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Dashboard</h2>
			<p class="text-sm text-gray-500 mt-1">Overview of your AmanClaw instance</p>
		</div>

		<div class="grid grid-cols-3 gap-4 mb-8">
			<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
				<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Communities</p>
				<p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.communities}</p>
			</div>
			<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
				<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Active Skills</p>
				<p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.skills}</p>
			</div>
			<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
				<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Users</p>
				<p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.users}</p>
			</div>
		</div>

		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<span class="w-3 h-3 rounded-full {$botStatus.running ? 'bg-green-500' : 'bg-red-500'}"></span>
					<div>
						<p class="text-sm font-medium text-gray-900">
							{$botStatus.running ? 'Bot Running' : 'Bot Stopped'}
						</p>
						<p class="text-xs text-gray-500">{$botStatus.mode === 'local' ? 'Local Mode' : 'Remote Mode'}</p>
					</div>
				</div>
				<button class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100 transition-colors">
					{$botStatus.running ? 'Stop' : 'Start'}
				</button>
			</div>
		</div>
	</div>
{/if}
