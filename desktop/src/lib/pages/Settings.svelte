<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let mode = $state('local');
	let remoteUrl = $state('');
	let remoteToken = $state('');
	let saved = $state(false);

	onMount(async () => {
		try {
			const m = await api.getMode() as string;
			if (m.startsWith('remote:')) {
				mode = 'remote';
				remoteUrl = m.replace('remote:', '');
			}
		} catch (e) {
			// Not connected
		}
	});

	async function saveMode() {
		try {
			await api.setMode(mode, remoteUrl, remoteToken);
			saved = true;
			setTimeout(() => saved = false, 2000);
		} catch (e) {
			// Handle error
		}
	}
</script>

<div class="p-8 max-w-2xl">
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Settings</h2>
		<p class="text-sm text-gray-500 mt-1">Configure your AmanClaw instance</p>
	</div>

	<div class="mb-8">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Connection Mode</h3>
		<div class="space-y-2">
			<label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
				{mode === 'local' ? 'border-gray-900 bg-gray-50' : ''}">
				<input type="radio" bind:group={mode} value="local" class="accent-gray-900">
				<div>
					<p class="text-sm font-medium text-gray-900">Local Mode</p>
					<p class="text-xs text-gray-500">Bot runs on this machine</p>
				</div>
			</label>
			<label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
				{mode === 'remote' ? 'border-gray-900 bg-gray-50' : ''}">
				<input type="radio" bind:group={mode} value="remote" class="accent-gray-900">
				<div>
					<p class="text-sm font-medium text-gray-900">Remote Mode</p>
					<p class="text-xs text-gray-500">Connect to a remote AmanClaw server</p>
				</div>
			</label>
		</div>

		{#if mode === 'remote'}
			<div class="mt-4 space-y-3">
				<div>
					<label class="block text-xs font-medium text-gray-700 mb-1">Server URL</label>
					<input type="text" bind:value={remoteUrl} placeholder="https://your-server.com"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label class="block text-xs font-medium text-gray-700 mb-1">API Token</label>
					<input type="password" bind:value={remoteToken} placeholder="Bearer token"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		{/if}

		<button onclick={saveMode}
			class="mt-4 px-4 py-2 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
			{saved ? 'Saved' : 'Save'}
		</button>
	</div>

	<div class="border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-2">About</h3>
		<p class="text-xs text-gray-500">AmanClaw Desktop v0.1.0</p>
		<p class="text-xs text-gray-500">Built in Malaysia</p>
	</div>
</div>
