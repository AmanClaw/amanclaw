<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let users: any[] = $state([]);
	let stats: any = $state(null);
	let loading = $state(true);
	let search = $state('');
	let platformFilter = $state('');
	let statusFilter = $state('');
	let selectedUser: any = $state(null);
	let userHistory: any[] = $state([]);
	let historyTotal = $state(0);
	let historyLoading = $state(false);

	const platforms = ['telegram', 'discord', 'whatsapp', 'whatsapp-web', 'slack'];
	const statuses = ['pending', 'approved', 'blocked'];
	const platformIcons: Record<string, string> = {
		telegram: 'TG',
		discord: 'DC',
		whatsapp: 'WA',
		'whatsapp-web': 'WA',
		slack: 'SL',
	};

	async function loadUsers() {
		try {
			const params: any = {};
			if (platformFilter) params.platform = platformFilter;
			if (statusFilter) params.status = statusFilter;
			if (search) params.search = search;
			const data = await api.getUsers(params) as any;
			users = data.users || [];
		} catch (e) {
			// Not connected
		}
		loading = false;
	}

	async function loadStats() {
		try {
			stats = await api.getUserStats() as any;
		} catch (_) {}
	}

	async function showUser(userId: string, platform: string) {
		try {
			selectedUser = await api.getUserDetail(userId, platform) as any;
			await loadHistory(userId, platform);
		} catch (e) {
			// Not connected
		}
	}

	async function loadHistory(userId: string, platform: string, offset = 0) {
		historyLoading = true;
		try {
			const data = await api.getUserHistory(userId, platform, 20, offset) as any;
			userHistory = data.messages || [];
			historyTotal = data.total || 0;
		} catch (_) {}
		historyLoading = false;
	}

	async function approve(userId: string, platform: string) {
		try {
			await api.approveUser(userId, platform);
			await loadUsers();
			await loadStats();
			if (selectedUser?.user_id === userId) selectedUser.state = 'approved';
		} catch (_) {}
	}

	async function block(userId: string, platform: string) {
		try {
			await api.blockUser(userId, platform);
			await loadUsers();
			await loadStats();
			if (selectedUser?.user_id === userId) selectedUser.state = 'blocked';
		} catch (_) {}
	}

	async function unblock(userId: string, platform: string) {
		try {
			await api.unblockUser(userId, platform);
			await loadUsers();
			await loadStats();
			if (selectedUser?.user_id === userId) selectedUser.state = 'pending';
		} catch (_) {}
	}

	function closeDetail() {
		selectedUser = null;
		userHistory = [];
	}

	function formatDate(d: string | null) {
		if (!d) return '-';
		return new Date(d).toLocaleString();
	}

	let searchTimeout: ReturnType<typeof setTimeout>;
	function onSearchInput() {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(loadUsers, 300);
	}

	onMount(() => {
		loadUsers();
		loadStats();
		const interval = setInterval(() => { loadUsers(); loadStats(); }, 5000);
		return () => clearInterval(interval);
	});
</script>

<div class="p-8 max-w-5xl">
	<div class="mb-6">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Users</h2>
		<p class="text-sm text-gray-500 mt-1">Manage bot users and permissions</p>
	</div>

	<!-- Stats -->
	{#if stats}
		<div class="grid grid-cols-4 gap-3 mb-6">
			<div class="bg-gray-50 rounded-xl border border-gray-200 p-4">
				<p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Total</p>
				<p class="text-2xl font-semibold text-gray-900 mt-1">{stats.total}</p>
			</div>
			<div class="bg-yellow-50 rounded-xl border border-yellow-200 p-4">
				<p class="text-[11px] font-medium text-yellow-600 uppercase tracking-wider">Pending</p>
				<p class="text-2xl font-semibold text-yellow-700 mt-1">{stats.pending}</p>
			</div>
			<div class="bg-green-50 rounded-xl border border-green-200 p-4">
				<p class="text-[11px] font-medium text-green-600 uppercase tracking-wider">Approved</p>
				<p class="text-2xl font-semibold text-green-700 mt-1">{stats.approved}</p>
			</div>
			<div class="bg-red-50 rounded-xl border border-red-200 p-4">
				<p class="text-[11px] font-medium text-red-600 uppercase tracking-wider">Blocked</p>
				<p class="text-2xl font-semibold text-red-700 mt-1">{stats.blocked}</p>
			</div>
		</div>

		{#if stats.by_platform && Object.keys(stats.by_platform).length > 0}
			<div class="flex flex-wrap gap-2 mb-6">
				{#each Object.entries(stats.by_platform) as [plat, count]}
					<span class="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-medium bg-gray-100 text-gray-700 rounded-full">
						<span class="text-[10px] font-bold text-gray-500">{platformIcons[plat] || plat}</span>
						{plat}: {count}
					</span>
				{/each}
			</div>
		{/if}
	{/if}

	<!-- Filters -->
	<div class="flex gap-3 mb-5">
		<input
			type="text"
			bind:value={search}
			oninput={onSearchInput}
			placeholder="Search users..."
			class="flex-1 px-3 py-2 text-sm rounded-lg border border-gray-200 bg-white text-gray-900 outline-none focus:border-gray-400 focus:ring-1 focus:ring-gray-400"
		/>
		<select
			bind:value={platformFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-gray-200 bg-white text-gray-700"
		>
			<option value="">All platforms</option>
			{#each platforms as p}
				<option value={p}>{p}</option>
			{/each}
		</select>
		<select
			bind:value={statusFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-gray-200 bg-white text-gray-700"
		>
			<option value="">All statuses</option>
			{#each statuses as s}
				<option value={s}>{s}</option>
			{/each}
		</select>
	</div>

	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else if users.length === 0}
		<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
			<p class="text-sm text-gray-500">No users found</p>
		</div>
	{:else}
		<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-gray-100">
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Platform</th>
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">User</th>
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Status</th>
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Last Seen</th>
						<th class="text-right px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each users as user}
						<tr class="border-b border-gray-50 hover:bg-gray-50 transition-colors cursor-pointer"
							onclick={() => showUser(user.user_id, user.platform)}>
							<td class="px-4 py-3">
								<span class="inline-flex items-center gap-1.5">
									<span class="text-[10px] font-bold text-gray-400">{platformIcons[user.platform] || '??'}</span>
									<span class="text-gray-500">{user.platform}</span>
								</span>
							</td>
							<td class="px-4 py-3">
								<div class="text-gray-900 font-mono text-xs">{user.user_id}</div>
								{#if user.username || user.first_name}
									<div class="text-[11px] text-gray-400">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
								{/if}
							</td>
							<td class="px-4 py-3">
								<span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full
									{user.state === 'approved' ? 'bg-green-100 text-green-700' :
									 user.state === 'pending' ? 'bg-yellow-100 text-yellow-700' :
									 user.state === 'blocked' ? 'bg-red-100 text-red-700' :
									 'bg-gray-100 text-gray-700'}">
									{user.state}
								</span>
							</td>
							<td class="px-4 py-3 text-xs text-gray-400">{formatDate(user.last_seen)}</td>
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<td class="px-4 py-3 text-right" onclick={(e) => e.stopPropagation()}>
								{#if user.state === 'pending'}
									<button onclick={() => approve(user.user_id, user.platform)}
										class="text-xs text-green-600 hover:text-green-800 font-medium mr-2">Approve</button>
								{/if}
								{#if user.state === 'blocked'}
									<button onclick={() => unblock(user.user_id, user.platform)}
										class="text-xs text-yellow-600 hover:text-yellow-800 font-medium mr-2">Unblock</button>
								{/if}
								{#if user.state !== 'blocked'}
									<button onclick={() => block(user.user_id, user.platform)}
										class="text-xs text-red-600 hover:text-red-800 font-medium">Block</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="text-xs text-gray-400 mt-2">{users.length} users</p>
	{/if}
</div>

<!-- User Detail Modal -->
{#if selectedUser}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="fixed inset-0 bg-black/40 z-50 flex items-start justify-center pt-12 px-4" onclick={closeDetail}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="bg-white rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
			onclick={(e) => e.stopPropagation()}>
			<div class="p-6">
				<!-- Header -->
				<div class="flex justify-between items-start mb-5">
					<div>
						<h3 class="text-lg font-semibold text-gray-900">
							<span class="text-[11px] font-bold text-gray-400 mr-1">{platformIcons[selectedUser.platform] || '??'}</span>
							{selectedUser.first_name || selectedUser.user_id}
						</h3>
						<p class="text-xs text-gray-500 mt-0.5">
							{selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
							{#if selectedUser.username} &middot; @{selectedUser.username}{/if}
						</p>
					</div>
					<button onclick={closeDetail} class="text-gray-400 hover:text-gray-600 text-lg leading-none">&times;</button>
				</div>

				<!-- Info Grid -->
				<div class="grid grid-cols-4 gap-3 mb-5">
					<div class="bg-gray-50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 uppercase">Status</p>
						<p class="text-sm font-semibold text-gray-900 capitalize">{selectedUser.state}</p>
					</div>
					<div class="bg-gray-50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 uppercase">Messages</p>
						<p class="text-sm font-semibold text-gray-900">{selectedUser.message_count}</p>
					</div>
					<div class="bg-gray-50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 uppercase">First Seen</p>
						<p class="text-[11px] font-semibold text-gray-900">{formatDate(selectedUser.first_seen)}</p>
					</div>
					<div class="bg-gray-50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 uppercase">Last Seen</p>
						<p class="text-[11px] font-semibold text-gray-900">{formatDate(selectedUser.last_seen)}</p>
					</div>
				</div>

				<!-- Facts -->
				{#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
					<div class="mb-5">
						<h4 class="text-xs font-semibold text-gray-700 mb-2">Learned Facts</h4>
						<div class="bg-gray-50 rounded-lg p-3">
							{#each Object.entries(selectedUser.facts) as [key, value]}
								<div class="flex justify-between py-1 text-xs">
									<span class="text-gray-500">{key}</span>
									<span class="text-gray-900">{value}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Conversation History -->
				<div>
					<h4 class="text-xs font-semibold text-gray-700 mb-2">
						Recent Conversations ({historyTotal} total)
					</h4>
					{#if historyLoading}
						<p class="text-gray-400 text-xs">Loading...</p>
					{:else}
						<div class="space-y-1.5 max-h-56 overflow-auto">
							{#each userHistory as msg}
								<div class="text-xs p-2 rounded-lg {msg.role === 'user' ? 'bg-blue-50 text-blue-900' : 'bg-gray-50 text-gray-900'}">
									<span class="text-[10px] font-medium text-gray-400 uppercase">{msg.role}</span>
									<p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
								</div>
							{/each}
							{#if userHistory.length === 0}
								<p class="text-gray-400 text-xs">No conversation history</p>
							{/if}
						</div>
					{/if}
				</div>

				<!-- Actions -->
				<div class="flex gap-2 mt-5 pt-4 border-t border-gray-100">
					{#if selectedUser.state !== 'approved'}
						<button onclick={() => approve(selectedUser.user_id, selectedUser.platform)}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 hover:bg-green-700 text-white transition-colors">
							Approve
						</button>
					{/if}
					{#if selectedUser.state === 'blocked'}
						<button onclick={() => unblock(selectedUser.user_id, selectedUser.platform)}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-yellow-600 hover:bg-yellow-700 text-white transition-colors">
							Unblock
						</button>
					{:else if selectedUser.state !== 'blocked'}
						<button onclick={() => block(selectedUser.user_id, selectedUser.platform)}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-red-600 hover:bg-red-700 text-white transition-colors">
							Block
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
