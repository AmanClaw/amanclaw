<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { botStatus, currentPage, isFirstRun } from '$lib/stores/app';
	import Dashboard from '$lib/pages/Dashboard.svelte';
	import Communities from '$lib/pages/Communities.svelte';
	import Agents from '$lib/pages/Agents.svelte';
	import Skills from '$lib/pages/Skills.svelte';
	import Marketplace from '$lib/pages/Marketplace.svelte';
	import CronJobs from '$lib/pages/CronJobs.svelte';
	import Webhooks from '$lib/pages/Webhooks.svelte';
	import Gateway from '$lib/pages/Gateway.svelte';
	import SubAgents from '$lib/pages/SubAgents.svelte';
	import KnowledgeBases from '$lib/pages/KnowledgeBases.svelte';
	import Users from '$lib/pages/Users.svelte';
	import Settings from '$lib/pages/Settings.svelte';
	import Logs from '$lib/pages/Logs.svelte';
	import Content from '$lib/pages/Content.svelte';
	import McpServers from '$lib/pages/McpServers.svelte';
	import Channels from '$lib/pages/Channels.svelte';
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

</script>

{#if !loaded}
	<div class="flex items-center justify-center h-full">
		<p class="text-sm text-fg-muted">Loading...</p>
	</div>
{:else if $isFirstRun}
	<Wizard />
{:else if $currentPage === 'channels'}
	<Channels />
{:else if $currentPage === 'communities'}
	<Communities />
{:else if $currentPage === 'agents'}
	<Agents />
{:else if $currentPage === 'skills'}
	<Skills />
{:else if $currentPage === 'marketplace'}
	<Marketplace />
{:else if $currentPage === 'cron'}
	<CronJobs />
{:else if $currentPage === 'webhooks'}
	<Webhooks />
{:else if $currentPage === 'gateway'}
	<Gateway />
{:else if $currentPage === 'subagents'}
	<SubAgents />
{:else if $currentPage === 'knowledgebases'}
	<KnowledgeBases />
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
	<Dashboard />
{/if}
