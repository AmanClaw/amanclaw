<script lang="ts">
	import '../app.css';
	import { isFirstRun, currentPage } from '$lib/stores/app';
	import { Sidebar as SharedSidebar, TopBar, BottomNav } from '@amanclaw/ui';
	import {
		LayoutDashboard, Bot, Users, Zap, Globe, Clock,
		Webhook, Radio, GitBranch, BookOpen, FileText,
		User, Hash, Server, ScrollText, Settings
	} from '@amanclaw/ui';

	let { children } = $props();
	let collapsed = $state(false);

	const navGroups = [
		{
			label: 'Main',
			items: [
				{ id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
				{ id: 'agents', label: 'Agents', icon: Bot },
				{ id: 'communities', label: 'Communities', icon: Users },
				{ id: 'skills', label: 'Skills', icon: Zap },
				{ id: 'marketplace', label: 'Marketplace', icon: Globe, badge: 'New' },
			]
		},
		{
			label: 'System',
			items: [
				{ id: 'cron', label: 'Cron Jobs', icon: Clock },
				{ id: 'webhooks', label: 'Webhooks', icon: Webhook },
				{ id: 'gateway', label: 'Gateway', icon: Radio },
				{ id: 'subagents', label: 'Sub-Agents', icon: GitBranch },
				{ id: 'knowledgebases', label: 'Knowledge Bases', icon: BookOpen },
				{ id: 'content', label: 'Content', icon: FileText },
				{ id: 'users', label: 'Users', icon: User },
				{ id: 'channels', label: 'Channels', icon: Hash },
				{ id: 'mcp', label: 'MCP Servers', icon: Server },
				{ id: 'logs', label: 'Logs', icon: ScrollText },
				{ id: 'settings', label: 'Settings', icon: Settings },
			]
		}
	];

	const mobileItems = [
		{ id: 'dashboard', label: 'Home', icon: LayoutDashboard },
		{ id: 'communities', label: 'Groups', icon: Users },
		{ id: 'skills', label: 'Skills', icon: Zap },
		{ id: 'settings', label: 'Settings', icon: Settings },
	];

	const moreItems = [
		{ id: 'agents', label: 'Agents', icon: Bot },
		{ id: 'cron', label: 'Cron Jobs', icon: Clock },
		{ id: 'webhooks', label: 'Webhooks', icon: Webhook },
		{ id: 'gateway', label: 'Gateway', icon: Radio },
		{ id: 'mcp', label: 'MCP Servers', icon: Server },
		{ id: 'logs', label: 'Logs', icon: ScrollText },
		{ id: 'channels', label: 'Channels', icon: Hash },
		{ id: 'content', label: 'Content', icon: FileText },
	];

	function handleNavigate(id: string) {
		currentPage.set(id);
	}

	const pageTitles: Record<string, string> = {
		dashboard: 'Dashboard', agents: 'Agents', communities: 'Communities',
		skills: 'Skills', marketplace: 'Marketplace', cron: 'Cron Jobs',
		webhooks: 'Webhooks', gateway: 'Gateway', subagents: 'Sub-Agents',
		knowledgebases: 'Knowledge Bases', content: 'Content', users: 'Users',
		channels: 'Channels', mcp: 'MCP Servers', logs: 'Logs', settings: 'Settings',
	};
</script>

{#if $isFirstRun}
	<main class="h-screen overflow-y-auto bg-base">
		{@render children()}
	</main>
{:else}
	<div class="flex h-screen bg-base select-none">
		<div class="hidden md:block">
			<SharedSidebar
				groups={navGroups}
				activePage={$currentPage}
				onNavigate={handleNavigate}
				{collapsed}
				onToggleCollapse={() => collapsed = !collapsed}
				userName="Admin"
				userInitials="AM"
				logoUrl="/logo.png"
			/>
		</div>
		<div class="flex-1 flex flex-col overflow-hidden">
			<TopBar
				breadcrumbs={[
					{ label: navGroups.find(g => g.items.some(i => i.id === $currentPage))?.label ?? 'Main' },
					{ label: pageTitles[$currentPage] ?? $currentPage, active: true }
				]}
			/>
			<main class="flex-1 overflow-y-auto p-6">
				{@render children()}
			</main>
		</div>
	</div>
	<div class="md:hidden">
		<BottomNav items={mobileItems} {moreItems} activePage={$currentPage} onNavigate={handleNavigate} />
	</div>
{/if}
