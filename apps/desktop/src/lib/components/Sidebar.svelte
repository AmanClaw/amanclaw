<script lang="ts">
	import { currentPage, botStatus } from '$lib/stores/app';
	import {
		LayoutDashboard, Users, Hash, Bot, Zap, Globe, Clock,
		Webhook, Radio, GitBranch, BookOpen, FileText,
		User, Server, ScrollText, Settings
	} from '@amanclaw/ui';

	const pages = [
		{ id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
		{ id: 'communities', label: 'Communities', icon: Users },
		{ id: 'channels', label: 'Channels', icon: Hash },
		{ id: 'agents', label: 'Agents', icon: Bot },
		{ id: 'skills', label: 'Skills', icon: Zap },
		{ id: 'marketplace', label: 'Marketplace', icon: Globe },
		{ id: 'cron', label: 'Cron Jobs', icon: Clock },
		{ id: 'webhooks', label: 'Webhooks', icon: Webhook },
		{ id: 'gateway', label: 'Gateway', icon: Radio },
		{ id: 'subagents', label: 'Sub-Agents', icon: GitBranch },
		{ id: 'knowledgebases', label: 'Knowledge Bases', icon: BookOpen },
		{ id: 'content', label: 'Content', icon: FileText },
		{ id: 'users', label: 'Users', icon: User },
		{ id: 'mcp', label: 'MCP Servers', icon: Server },
		{ id: 'logs', label: 'Logs', icon: ScrollText },
	];

	const bottomPages = [
		{ id: 'settings', label: 'Settings', icon: Settings },
	];

	const statusColor = $derived(
		$botStatus.engine_status === 'running' ? 'bg-green-500' :
		$botStatus.engine_status === 'starting' ? 'bg-yellow-500 animate-pulse' :
		$botStatus.engine_status === 'error' ? 'bg-red-500' :
		'bg-gray-400'
	);

	const statusText = $derived(
		$botStatus.engine_status === 'running' ? 'Engine Running' :
		$botStatus.engine_status === 'starting' ? 'Starting...' :
		$botStatus.engine_status === 'error' ? 'Engine Error' :
		'Engine Stopped'
	);
</script>

<aside class="w-56 h-screen bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-xl border-r border-gray-200 dark:border-gray-700 flex flex-col justify-between p-3">
	<div>
		<div class="px-3 py-4 mb-2">
			<h1 class="text-sm font-semibold text-gray-900 dark:text-white tracking-tight">AmanClaw</h1>
		</div>
		<nav class="space-y-0.5">
			{#each pages as page}
				<button
					class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] transition-colors
						{$currentPage === page.id
							? 'bg-gray-200/80 dark:bg-gray-700/80 text-gray-900 dark:text-white font-medium'
							: 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-white'}"
					onclick={() => currentPage.set(page.id)}
				>
					<page.icon size={16} class="shrink-0" />
					{page.label}
				</button>
			{/each}
		</nav>
	</div>

	<div>
		<div class="border-t border-gray-200 dark:border-gray-700 pt-2 mb-2">
			{#each bottomPages as page}
				<button
					class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-white transition-colors"
					onclick={() => currentPage.set(page.id)}
				>
					<page.icon size={16} class="shrink-0" />
					{page.label}
				</button>
			{/each}
		</div>
		<div class="mx-2 p-2.5 bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm">
			<div class="flex items-center gap-2">
				<span class="w-2 h-2 rounded-full {statusColor}"></span>
				<span class="text-[11px] font-medium text-gray-700 dark:text-gray-300">{statusText}</span>
			</div>
		</div>
	</div>
</aside>
