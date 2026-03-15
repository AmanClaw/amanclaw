<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

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

<div>
	<PageHeader title="Users" subtitle="Manage bot users and permissions">
		{#snippet action()}
			<Button size="sm" onclick={() => showAddModal = true}>+ Add User</Button>
		{/snippet}
	</PageHeader>

	<!-- Stats -->
	{#if stats}
		<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-6">
			<div class="bg-base rounded-xl border border-border p-4">
				<p class="text-[11px] font-medium text-fg-muted uppercase tracking-wider">Total</p>
				<p class="text-2xl font-semibold text-fg mt-1">{stats.total}</p>
			</div>
			<div class="bg-[var(--color-accent-500-15)] rounded-xl border border-[var(--color-accent-500-15)] p-4">
				<p class="text-[11px] font-medium text-[var(--color-accent-500)] uppercase tracking-wider">Admin</p>
				<p class="text-2xl font-semibold text-[var(--color-accent-500)] mt-1">{stats.admin ?? 0}</p>
			</div>
			<div class="bg-[var(--color-warning-15)] rounded-xl border border-[var(--color-warning-20)] p-4">
				<p class="text-[11px] font-medium text-warning uppercase tracking-wider">Pending</p>
				<p class="text-2xl font-semibold text-warning mt-1">{stats.pending}</p>
			</div>
			<div class="bg-[var(--color-success-15)] rounded-xl border border-[var(--color-success-20)] p-4">
				<p class="text-[11px] font-medium text-success uppercase tracking-wider">Approved</p>
				<p class="text-2xl font-semibold text-success mt-1">{stats.approved}</p>
			</div>
			<div class="bg-[var(--color-error-15)] rounded-xl border border-[var(--color-error-20)] p-4">
				<p class="text-[11px] font-medium text-error uppercase tracking-wider">Blocked</p>
				<p class="text-2xl font-semibold text-error mt-1">{stats.blocked}</p>
			</div>
		</div>

		{#if stats.by_platform && Object.keys(stats.by_platform).length > 0}
			<div class="flex flex-wrap gap-2 mb-6">
				{#each Object.entries(stats.by_platform) as [plat, count]}
					<span class="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-medium bg-elevated text-fg-secondary rounded-full">
						<span class="text-[10px] font-bold text-fg-muted">{platformIcons[plat] || plat}</span>
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
			class="flex-1 min-w-48 px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg outline-none focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
		/>
		<select
			bind:value={platformFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg-secondary"
		>
			<option value="">All platforms</option>
			{#each platforms as p}
				<option value={p}>{p}</option>
			{/each}
		</select>
		<select
			bind:value={statusFilter}
			onchange={loadUsers}
			class="px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg-secondary"
		>
			<option value="">All statuses</option>
			{#each statuses as s}
				<option value={s}>{s}</option>
			{/each}
		</select>
	</div>

	{#if loading}
		<p class="text-sm text-fg-muted">Loading...</p>
	{:else if users.length === 0}
		<div class="text-center py-16 bg-base rounded-xl border border-border">
			<p class="text-sm text-fg-muted">No users found</p>
		</div>
	{:else}
		<div class="bg-base rounded-xl border border-border overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-border">
							<th class="text-left px-4 py-3 text-[11px] font-medium text-fg-muted uppercase tracking-wider">Platform</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-fg-muted uppercase tracking-wider">User</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-fg-muted uppercase tracking-wider">Status</th>
							<th class="text-left px-4 py-3 text-[11px] font-medium text-fg-muted uppercase tracking-wider">Last Seen</th>
							<th class="text-right px-4 py-3 text-[11px] font-medium text-fg-muted uppercase tracking-wider">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each users as user}
							<tr class="border-b border-border/50 hover:bg-[var(--color-elevated-50)] transition-colors cursor-pointer"
								onclick={() => showUser(user.user_id, user.platform)}>
								<td class="px-4 py-3">
									<span class="inline-flex items-center gap-1.5">
										<span class="text-[10px] font-bold text-fg-muted">{platformIcons[user.platform] || '??'}</span>
										<span class="text-fg-muted">{user.platform}</span>
									</span>
								</td>
								<td class="px-4 py-3">
									<div class="text-fg font-mono text-xs">{user.user_id}</div>
									{#if user.username || user.first_name}
										<div class="text-[11px] text-fg-muted">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
									{/if}
								</td>
								<td class="px-4 py-3">
									<span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full
										{user.state === 'admin' ? 'bg-[var(--color-accent-500-15)] text-[var(--color-accent-500)]' :
										 user.state === 'approved' ? 'bg-[var(--color-success-15)] text-success' :
										 user.state === 'pending' ? 'bg-[var(--color-warning-15)] text-warning' :
										 user.state === 'blocked' ? 'bg-[var(--color-error-15)] text-error' :
										 'bg-elevated text-fg-secondary'}">
										{user.state}
									</span>
								</td>
								<td class="px-4 py-3 text-xs text-fg-muted">{formatDate(user.last_seen)}</td>
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<td class="px-4 py-3 text-right" onclick={(e) => e.stopPropagation()}>
									{#if user.state === 'admin'}
										<button onclick={() => removeAdmin(user.user_id, user.platform)}
											class="text-xs text-[var(--color-accent-500)] hover:text-[var(--color-accent-500)] font-medium mr-2">Remove Admin</button>
									{:else}
										{#if user.state === 'pending'}
											<button onclick={() => approve(user.user_id, user.platform)}
												class="text-xs text-success hover:text-success font-medium mr-2">Approve</button>
										{/if}
										{#if user.state === 'blocked'}
											<button onclick={() => unblock(user.user_id, user.platform)}
												class="text-xs text-warning hover:text-warning font-medium mr-2">Unblock</button>
										{/if}
										{#if user.state === 'approved'}
											<button onclick={() => makeAdmin(user.user_id, user.platform)}
												class="text-xs text-[var(--color-accent-500)] hover:text-[var(--color-accent-500)] font-medium mr-2">Make Admin</button>
										{/if}
										{#if user.state !== 'blocked'}
											<button onclick={() => block(user.user_id, user.platform)}
												class="text-xs text-error hover:text-error font-medium">Block</button>
										{/if}
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
		<p class="text-xs text-fg-muted mt-2">{users.length} users</p>
	{/if}
</div>

<!-- User Detail Modal -->
{#if selectedUser}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="fixed inset-0 bg-black/40 z-50 flex items-start justify-center pt-12 px-4" onclick={closeDetail}>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="bg-base rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
			onclick={(e) => e.stopPropagation()}>
			<div class="p-6">
				<!-- Header -->
				<div class="flex justify-between items-start mb-5">
					<div>
						<h3 class="text-lg font-semibold text-fg">
							<span class="text-[11px] font-bold text-fg-muted mr-1">{platformIcons[selectedUser.platform] || '??'}</span>
							{selectedUser.first_name || selectedUser.user_id}
						</h3>
						<p class="text-xs text-fg-muted mt-0.5">
							{selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
							{#if selectedUser.username} &middot; @{selectedUser.username}{/if}
						</p>
					</div>
					<button onclick={closeDetail} class="text-fg-muted hover:text-fg-secondary text-lg leading-none">&times;</button>
				</div>

				<!-- Info Grid -->
				<div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-5">
					<div class="bg-base/50 rounded-lg p-3">
						<p class="text-[10px] text-fg-muted uppercase">Status</p>
						<p class="text-sm font-semibold text-fg capitalize">{selectedUser.state}</p>
					</div>
					<div class="bg-base/50 rounded-lg p-3">
						<p class="text-[10px] text-fg-muted uppercase">Messages</p>
						<p class="text-sm font-semibold text-fg">{selectedUser.message_count}</p>
					</div>
					<div class="bg-base/50 rounded-lg p-3">
						<p class="text-[10px] text-fg-muted uppercase">First Seen</p>
						<p class="text-[11px] font-semibold text-fg">{formatDate(selectedUser.first_seen)}</p>
					</div>
					<div class="bg-base/50 rounded-lg p-3">
						<p class="text-[10px] text-fg-muted uppercase">Last Seen</p>
						<p class="text-[11px] font-semibold text-fg">{formatDate(selectedUser.last_seen)}</p>
					</div>
				</div>

				<!-- Facts -->
				{#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
					<div class="mb-5">
						<h4 class="text-xs font-semibold text-fg-secondary mb-2">Learned Facts</h4>
						<div class="bg-base/50 rounded-lg p-3">
							{#each Object.entries(selectedUser.facts) as [key, value]}
								<div class="flex justify-between py-1 text-xs">
									<span class="text-fg-muted">{key}</span>
									<span class="text-fg">{value}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Conversation History -->
				<div>
					<h4 class="text-xs font-semibold text-fg-secondary mb-2">
						Recent Conversations ({historyTotal} total)
					</h4>
					{#if historyLoading}
						<p class="text-fg-muted text-xs">Loading...</p>
					{:else}
						<div class="space-y-1.5 max-h-56 overflow-auto">
							{#each userHistory as msg}
								<div class="text-xs p-2 rounded-lg {msg.role === 'user' ? 'bg-[var(--color-info-15)] text-blue-900' : 'bg-base/50 text-fg'}">
									<span class="text-[10px] font-medium text-fg-muted uppercase">{msg.role}</span>
									<p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
								</div>
							{/each}
							{#if userHistory.length === 0}
								<p class="text-fg-muted text-xs">No conversation history</p>
							{/if}
						</div>
					{/if}
				</div>

				<!-- Actions -->
				<div class="flex gap-2 mt-5 pt-4 border-t border-border">
					{#if selectedUser.state === 'admin'}
						<button onclick={() => removeAdmin(selectedUser.user_id, selectedUser.platform)}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--color-accent-500)] hover:bg-[var(--color-accent-500)] text-white transition-colors">
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
								class="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--color-accent-500)] hover:bg-[var(--color-accent-500)] text-white transition-colors">
								Make Admin
							</button>
						{/if}
						{#if selectedUser.state === 'blocked'}
							<button onclick={() => unblock(selectedUser.user_id, selectedUser.platform)}
								class="px-3 py-1.5 text-xs font-medium rounded-md bg-warning hover:bg-warning text-white transition-colors">
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
		<div class="bg-base rounded-xl shadow-xl w-full max-w-md" onclick={(e) => e.stopPropagation()}>
			<div class="p-6">
				<div class="flex justify-between items-center mb-5">
					<h3 class="text-lg font-semibold text-fg">Add User</h3>
					<button onclick={() => showAddModal = false} class="text-fg-muted hover:text-fg-secondary text-lg leading-none">&times;</button>
				</div>

				<div class="space-y-3">
					<div>
						<label class="block text-[11px] font-medium text-fg-secondary mb-1">Platform</label>
						<select bind:value={newPlatform}
							class="w-full px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg-secondary">
							{#each platforms as p}
								<option value={p}>{p}</option>
							{/each}
						</select>
					</div>
					<div>
						<label class="block text-[11px] font-medium text-fg-secondary mb-1">User ID <span class="text-error">*</span></label>
						<input type="text" bind:value={newUserId} placeholder="e.g. 123456789"
							class="w-full px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg outline-none focus:border-primary-500 focus:ring-1 focus:ring-primary-500" />
						<p class="text-[10px] text-fg-muted mt-0.5">Telegram: numeric ID | Discord: snowflake | WhatsApp: phone number</p>
					</div>
					<div class="grid grid-cols-2 gap-3">
						<div>
							<label class="block text-[11px] font-medium text-fg-secondary mb-1">Username</label>
							<input type="text" bind:value={newUsername} placeholder="Optional"
								class="w-full px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg outline-none focus:border-primary-500 focus:ring-1 focus:ring-primary-500" />
						</div>
						<div>
							<label class="block text-[11px] font-medium text-fg-secondary mb-1">First Name</label>
							<input type="text" bind:value={newFirstName} placeholder="Optional"
								class="w-full px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg outline-none focus:border-primary-500 focus:ring-1 focus:ring-primary-500" />
						</div>
					</div>
					<div>
						<label class="block text-[11px] font-medium text-fg-secondary mb-1">Initial Status</label>
						<select bind:value={newStatus}
							class="w-full px-3 py-2 text-sm rounded-lg border border-border bg-base text-fg-secondary">
							<option value="approved">Approved</option>
							<option value="pending">Pending</option>
						</select>
					</div>
				</div>

				{#if addError}
					<div class="mt-3 p-2 bg-[var(--color-error-15)] rounded-lg text-xs text-error">{addError}</div>
				{/if}

				<div class="flex gap-2 mt-5 pt-4 border-t border-border">
					<button onclick={addNewUser} disabled={addSaving}
						class="px-4 py-2 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 disabled:opacity-50 transition-colors">
						{addSaving ? 'Adding...' : 'Add User'}
					</button>
					<button onclick={() => showAddModal = false}
						class="px-4 py-2 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
						Cancel
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
