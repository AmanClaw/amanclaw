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

	// Add user modal
	let showAddModal = $state(false);
	let newUserId = $state('');
	let newPlatform = $state('telegram');
	let newUsername = $state('');
	let newFirstName = $state('');
	let newStatus = $state('approved');
	let addSaving = $state(false);
	let addError = $state('');

	const platforms = ['telegram', 'discord', 'whatsapp', 'whatsapp-web', 'slack'];
	const statuses = ['admin', 'pending', 'approved', 'blocked'];
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

	async function makeAdmin(userId: string, platform: string) {
		try {
			await api.makeAdmin(userId, platform);
			await loadUsers();
			await loadStats();
			if (selectedUser?.user_id === userId) selectedUser.state = 'admin';
		} catch (_) {}
	}

	async function removeAdmin(userId: string, platform: string) {
		try {
			await api.removeAdmin(userId, platform);
			await loadUsers();
			await loadStats();
			if (selectedUser?.user_id === userId) selectedUser.state = 'approved';
		} catch (_) {}
	}

	async function addNewUser() {
		if (!newUserId.trim()) { addError = 'User ID is required'; return; }
		addSaving = true;
		addError = '';
		try {
			await api.addUser({
				userId: newUserId.trim(),
				platform: newPlatform,
				username: newUsername.trim() || undefined,
				firstName: newFirstName.trim() || undefined,
				status: newStatus,
			});
			showAddModal = false;
			newUserId = '';
			newUsername = '';
			newFirstName = '';
			newStatus = 'approved';
			await loadUsers();
			await loadStats();
		} catch (e: any) {
			addError = e?.toString() || 'Failed to add user';
		} finally {
			addSaving = false;
		}
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

<div class="p-6 md:p-8">
	<div class="flex items-start justify-between mb-6">
		<div>
			<h2 class="text-xl font-semibold text-gray-900 dark:text-white tracking-tight">Users</h2>
			<p class="text-sm text-gray-500 dark:text-gray-400 mt-1">Manage bot users and permissions</p>
		</div>
		<button onclick={() => showAddModal = true}
			class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 dark:bg-white text-white dark:text-gray-900 hover:bg-gray-800 dark:hover:bg-gray-100 transition-colors">
			+ Add User
		</button>
	</div>

	<!-- Stats -->
	{#if stats}
		<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-6">
			<div class="bg-gray-50 dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
				<p class="text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Total</p>
				<p class="text-2xl font-semibold text-gray-900 dark:text-white mt-1">{stats.total}</p>
			</div>
			<div class="bg-purple-50 dark:bg-purple-900/20 rounded-xl border border-purple-200 dark:border-purple-800 p-4">
				<p class="text-[11px] font-medium text-purple-600 dark:text-purple-400 uppercase tracking-wider">Admin</p>
				<p class="text-2xl font-semibold text-purple-700 dark:text-purple-300 mt-1">{stats.admin ?? 0}</p>
			</div>
			<div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-xl border border-yellow-200 dark:border-yellow-800 p-4">
				<p class="text-[11px] font-medium text-yellow-600 dark:text-yellow-400 uppercase tracking-wider">Pending</p>
				<p class="text-2xl font-semibold text-yellow-700 dark:text-yellow-300 mt-1">{stats.pending}</p>
			</div>
			<div class="bg-green-50 dark:bg-green-900/20 rounded-xl border border-green-200 dark:border-green-800 p-4">
				<p class="text-[11px] font-medium text-green-600 dark:text-green-400 uppercase tracking-wider">Approved</p>
				<p class="text-2xl font-semibold text-green-700 dark:text-green-300 mt-1">{stats.approved}</p>
			</div>
			<div class="bg-red-50 dark:bg-red-900/20 rounded-xl border border-red-200 dark:border-red-800 p-4">
				<p class="text-[11px] font-medium text-red-600 dark:text-red-400 uppercase tracking-wider">Blocked</p>
				<p class="text-2xl font-semibold text-red-700 dark:text-red-300 mt-1">{stats.blocked}</p>
			</div>
		</div>

		{#if stats.by_platform && Object.keys(stats.by_platform).length > 0}
			<div class="flex flex-wrap gap-2 mb-6">
				{#each Object.entries(stats.by_platform) as [plat, count]}
					<span class="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-medium bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 rounded-full">
						<span class="text-[10px] font-bold text-gray-500 dark:text-gray-400">{platformIcons[plat] || plat}</span>
						{plat}: {count}
					</span>
				{/each}
			</div>
		{/if}
	{/if}

	<!-- Filters -->
	<div class="flex flex-wrap gap-3 mb-5">
		<input
			type="text"
			bind:value={search}
			oninput={onSearchInput}
			placeholder="Search users..."
			class="flex-1 min-w-48 px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:border-gray-400 dark:focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
		/>
		<select
			bind:value={platformFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300"
		>
			<option value="">All platforms</option>
			{#each platforms as p}
				<option value={p}>{p}</option>
			{/each}
		</select>
		<select
			bind:value={statusFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300"
		>
			<option value="">All statuses</option>
			{#each statuses as s}
				<option value={s}>{s}</option>
			{/each}
		</select>
	</div>

	{#if loading}
		<p class="text-sm text-gray-500 dark:text-gray-400">Loading...</p>
	{:else if users.length === 0}
		<div class="text-center py-16 bg-gray-50 dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
			<p class="text-sm text-gray-500 dark:text-gray-400">No users found</p>
		</div>
	{:else}
		<div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-gray-100 dark:border-gray-700">
							<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Platform</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">User</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Last Seen</th>
							<th class="text-right px-4 py-3 text-[11px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each users as user}
							<tr class="border-b border-gray-50 dark:border-gray-700/50 hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors cursor-pointer"
								onclick={() => showUser(user.user_id, user.platform)}>
								<td class="px-4 py-3">
									<span class="inline-flex items-center gap-1.5">
										<span class="text-[10px] font-bold text-gray-400 dark:text-gray-500">{platformIcons[user.platform] || '??'}</span>
										<span class="text-gray-500 dark:text-gray-400">{user.platform}</span>
									</span>
								</td>
								<td class="px-4 py-3">
									<div class="text-gray-900 dark:text-white font-mono text-xs">{user.user_id}</div>
									{#if user.username || user.first_name}
										<div class="text-[11px] text-gray-400 dark:text-gray-500">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
									{/if}
								</td>
								<td class="px-4 py-3">
									<span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full
										{user.state === 'admin' ? 'bg-purple-100 dark:bg-purple-900/40 text-purple-700 dark:text-purple-300' :
										 user.state === 'approved' ? 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-300' :
										 user.state === 'pending' ? 'bg-yellow-100 dark:bg-yellow-900/40 text-yellow-700 dark:text-yellow-300' :
										 user.state === 'blocked' ? 'bg-red-100 dark:bg-red-900/40 text-red-700 dark:text-red-300' :
										 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300'}">
										{user.state}
									</span>
								</td>
								<td class="px-4 py-3 text-xs text-gray-400 dark:text-gray-500">{formatDate(user.last_seen)}</td>
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<td class="px-4 py-3 text-right" onclick={(e) => e.stopPropagation()}>
									{#if user.state === 'admin'}
										<button onclick={() => removeAdmin(user.user_id, user.platform)}
											class="text-xs text-purple-600 dark:text-purple-400 hover:text-purple-800 dark:hover:text-purple-300 font-medium mr-2">Remove Admin</button>
									{:else}
										{#if user.state === 'pending'}
											<button onclick={() => approve(user.user_id, user.platform)}
												class="text-xs text-green-600 dark:text-green-400 hover:text-green-800 font-medium mr-2">Approve</button>
										{/if}
										{#if user.state === 'blocked'}
											<button onclick={() => unblock(user.user_id, user.platform)}
												class="text-xs text-yellow-600 dark:text-yellow-400 hover:text-yellow-800 font-medium mr-2">Unblock</button>
										{/if}
										{#if user.state === 'approved'}
											<button onclick={() => makeAdmin(user.user_id, user.platform)}
												class="text-xs text-purple-600 dark:text-purple-400 hover:text-purple-800 font-medium mr-2">Make Admin</button>
										{/if}
										{#if user.state !== 'blocked'}
											<button onclick={() => block(user.user_id, user.platform)}
												class="text-xs text-red-600 dark:text-red-400 hover:text-red-800 font-medium">Block</button>
										{/if}
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
		<p class="text-xs text-gray-400 dark:text-gray-500 mt-2">{users.length} users</p>
	{/if}
</div>

<!-- User Detail Modal -->
{#if selectedUser}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="fixed inset-0 bg-black/40 z-50 flex items-start justify-center pt-12 px-4" onclick={closeDetail}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
			onclick={(e) => e.stopPropagation()}>
			<div class="p-6">
				<!-- Header -->
				<div class="flex justify-between items-start mb-5">
					<div>
						<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
							<span class="text-[11px] font-bold text-gray-400 dark:text-gray-500 mr-1">{platformIcons[selectedUser.platform] || '??'}</span>
							{selectedUser.first_name || selectedUser.user_id}
						</h3>
						<p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
							{selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
							{#if selectedUser.username} &middot; @{selectedUser.username}{/if}
						</p>
					</div>
					<button onclick={closeDetail} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-lg leading-none">&times;</button>
				</div>

				<!-- Info Grid -->
				<div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-5">
					<div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 dark:text-gray-400 uppercase">Status</p>
						<p class="text-sm font-semibold text-gray-900 dark:text-white capitalize">{selectedUser.state}</p>
					</div>
					<div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 dark:text-gray-400 uppercase">Messages</p>
						<p class="text-sm font-semibold text-gray-900 dark:text-white">{selectedUser.message_count}</p>
					</div>
					<div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 dark:text-gray-400 uppercase">First Seen</p>
						<p class="text-[11px] font-semibold text-gray-900 dark:text-white">{formatDate(selectedUser.first_seen)}</p>
					</div>
					<div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
						<p class="text-[10px] text-gray-500 dark:text-gray-400 uppercase">Last Seen</p>
						<p class="text-[11px] font-semibold text-gray-900 dark:text-white">{formatDate(selectedUser.last_seen)}</p>
					</div>
				</div>

				<!-- Facts -->
				{#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
					<div class="mb-5">
						<h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-2">Learned Facts</h4>
						<div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
							{#each Object.entries(selectedUser.facts) as [key, value]}
								<div class="flex justify-between py-1 text-xs">
									<span class="text-gray-500 dark:text-gray-400">{key}</span>
									<span class="text-gray-900 dark:text-white">{value}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Conversation History -->
				<div>
					<h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-2">
						Recent Conversations ({historyTotal} total)
					</h4>
					{#if historyLoading}
						<p class="text-gray-400 text-xs">Loading...</p>
					{:else}
						<div class="space-y-1.5 max-h-56 overflow-auto">
							{#each userHistory as msg}
								<div class="text-xs p-2 rounded-lg {msg.role === 'user' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-900 dark:text-blue-100' : 'bg-gray-50 dark:bg-gray-700/50 text-gray-900 dark:text-white'}">
									<span class="text-[10px] font-medium text-gray-400 uppercase">{msg.role}</span>
									<p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
								</div>
							{/each}
							{#if userHistory.length === 0}
								<p class="text-gray-400 dark:text-gray-500 text-xs">No conversation history</p>
							{/if}
						</div>
					{/if}
				</div>

				<!-- Actions -->
				<div class="flex gap-2 mt-5 pt-4 border-t border-gray-100 dark:border-gray-700">
					{#if selectedUser.state === 'admin'}
						<button onclick={() => removeAdmin(selectedUser.user_id, selectedUser.platform)}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-purple-600 hover:bg-purple-700 text-white transition-colors">
							Remove Admin
						</button>
					{:else}
						{#if selectedUser.state !== 'approved' && selectedUser.state !== 'admin'}
							<button onclick={() => approve(selectedUser.user_id, selectedUser.platform)}
								class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 hover:bg-green-700 text-white transition-colors">
								Approve
							</button>
						{/if}
						{#if selectedUser.state === 'approved'}
							<button onclick={() => makeAdmin(selectedUser.user_id, selectedUser.platform)}
								class="px-3 py-1.5 text-xs font-medium rounded-md bg-purple-600 hover:bg-purple-700 text-white transition-colors">
								Make Admin
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
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Add User Modal -->
{#if showAddModal}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="fixed inset-0 bg-black/40 z-50 flex items-start justify-center pt-16 px-4" onclick={() => showAddModal = false}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md" onclick={(e) => e.stopPropagation()}>
			<div class="p-6">
				<div class="flex justify-between items-center mb-5">
					<h3 class="text-lg font-semibold text-gray-900 dark:text-white">Add User</h3>
					<button onclick={() => showAddModal = false} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-lg leading-none">&times;</button>
				</div>

				<div class="space-y-3">
					<div>
						<label class="block text-[11px] font-medium text-gray-600 dark:text-gray-400 mb-1">Platform</label>
						<select bind:value={newPlatform}
							class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300">
							{#each platforms as p}
								<option value={p}>{p}</option>
							{/each}
						</select>
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-600 dark:text-gray-400 mb-1">User ID <span class="text-red-500">*</span></label>
						<input type="text" bind:value={newUserId} placeholder="e.g. 123456789"
							class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:border-gray-400 focus:ring-1 focus:ring-gray-400" />
						<p class="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">Telegram: numeric ID | Discord: snowflake | WhatsApp: phone number</p>
					</div>
					<div class="grid grid-cols-2 gap-3">
						<div>
							<label class="block text-[11px] font-medium text-gray-600 dark:text-gray-400 mb-1">Username</label>
							<input type="text" bind:value={newUsername} placeholder="Optional"
								class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:border-gray-400 focus:ring-1 focus:ring-gray-400" />
						</div>
						<div>
							<label class="block text-[11px] font-medium text-gray-600 dark:text-gray-400 mb-1">First Name</label>
							<input type="text" bind:value={newFirstName} placeholder="Optional"
								class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:border-gray-400 focus:ring-1 focus:ring-gray-400" />
						</div>
					</div>
					<div>
						<label class="block text-[11px] font-medium text-gray-600 dark:text-gray-400 mb-1">Initial Status</label>
						<select bind:value={newStatus}
							class="w-full px-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300">
							<option value="approved">Approved</option>
							<option value="pending">Pending</option>
						</select>
					</div>
				</div>

				{#if addError}
					<div class="mt-3 p-2 bg-red-50 dark:bg-red-900/20 rounded-lg text-xs text-red-700 dark:text-red-300">{addError}</div>
				{/if}

				<div class="flex gap-2 mt-5 pt-4 border-t border-gray-100 dark:border-gray-700">
					<button onclick={addNewUser} disabled={addSaving}
						class="px-4 py-2 text-xs font-medium rounded-md bg-gray-900 dark:bg-white text-white dark:text-gray-900 hover:bg-gray-800 dark:hover:bg-gray-100 disabled:opacity-50 transition-colors">
						{addSaving ? 'Adding...' : 'Add User'}
					</button>
					<button onclick={() => showAddModal = false}
						class="px-4 py-2 text-xs font-medium rounded-md border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
						Cancel
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
