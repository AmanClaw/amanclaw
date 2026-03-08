<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { botStatus, currentPage, isFirstRun } from '$lib/stores/app';
	import Communities from '$lib/pages/Communities.svelte';
	import Skills from '$lib/pages/Skills.svelte';
	import Users from '$lib/pages/Users.svelte';
	import Settings from '$lib/pages/Settings.svelte';
	import Logs from '$lib/pages/Logs.svelte';
	import Content from '$lib/pages/Content.svelte';
	import McpServers from '$lib/pages/McpServers.svelte';
	import Wizard from '$lib/pages/Wizard.svelte';

	let loaded = $state(false);

	async function refreshStatus() {
		try {
			const [status, communities, skills, users] = await Promise.all([
				api.getStatus(),
				api.getCommunities().catch(() => ({ count: 0 })),
				api.getSkills().catch(() => ({ count: 0 })),
				api.getUsers().catch(() => ({ count: 0 })),
			]);
			botStatus.set({
				...$botStatus,
				...(status as any),
				communities: (communities as any).count ?? 0,
				skills: (skills as any).count ?? 0,
				users: (users as any).count ?? 0,
			});
		} catch (_) {}
	}

	onMount(async () => {
		try {
			const firstRun = await api.checkFirstRun();
			isFirstRun.set(firstRun);
			if (!firstRun) {
				await refreshStatus();
			}
		} catch (e) {
			// Not connected yet
		}
		loaded = true;

		// Poll status every 3 seconds to stay in sync with engine
		const interval = setInterval(refreshStatus, 3000);
		return () => clearInterval(interval);
	});

	async function handleStart() {
		try {
			await api.startEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}

	async function handleStop() {
		try {
			await api.stopEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e) {
			// ignore
		}
	}

	async function handleRestart() {
		try {
			botStatus.set({ ...$botStatus, engine_status: 'starting' });
			await api.restartEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}
</script>

{#if !loaded}
	<div class="flex items-center justify-center h-full">
		<p class="text-sm text-gray-400">Loading...</p>
	</div>
{:else if $isFirstRun}
	<Wizard />
{:else if $currentPage === 'communities'}
	<Communities />
{:else if $currentPage === 'skills'}
	<Skills />
{:else if $currentPage === 'users'}
	<Users />
{:else if $currentPage === 'settings'}
	<Settings />
{:else if $currentPage === 'logs'}
	<Logs />
{:else if $currentPage === 'mcp'}
	<McpServers />
{:else if $currentPage === 'content'}
	<Content />
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

		<!-- Engine Control -->
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5 mb-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<span class="w-3 h-3 rounded-full {
						$botStatus.engine_status === 'running' ? 'bg-green-500' :
						$botStatus.engine_status === 'starting' ? 'bg-yellow-500 animate-pulse' :
						$botStatus.engine_status === 'error' ? 'bg-red-500' :
						'bg-gray-400'
					}"></span>
					<div>
						<p class="text-sm font-medium text-gray-900">
							{$botStatus.engine_status === 'running' ? 'Engine Running' :
							 $botStatus.engine_status === 'starting' ? 'Engine Starting...' :
							 $botStatus.engine_status === 'error' ? 'Engine Error' :
							 'Engine Stopped'}
						</p>
						<p class="text-xs text-gray-500">
							{$botStatus.mode === 'local' ? 'Local Mode' : 'Remote Mode'}
							{#if $botStatus.uptime_seconds > 0}
								 · Uptime: {Math.floor($botStatus.uptime_seconds / 60)}m
							{/if}
						</p>
					</div>
				</div>
				<div class="flex gap-2">
					{#if $botStatus.engine_status === 'running'}
						<button onclick={handleRestart}
							class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100 transition-colors">
							Restart
						</button>
						<button onclick={handleStop}
							class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-300 text-red-700 hover:bg-red-50 transition-colors">
							Stop
						</button>
					{:else if $botStatus.engine_status !== 'starting'}
						<button onclick={handleStart}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
							Start
						</button>
					{/if}
				</div>
			</div>

			{#if $botStatus.error}
				<div class="mt-3 p-2 bg-red-50 rounded text-xs text-red-700">
					{$botStatus.error}
				</div>
			{/if}
		</div>
	</div>
{/if}
