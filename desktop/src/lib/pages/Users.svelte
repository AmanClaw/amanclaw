<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let users: any[] = $state([]);
	let loading = $state(true);

	async function loadUsers() {
		try {
			const data = await api.getUsers() as any;
			users = data.users || [];
		} catch (e) {
			// Not connected
		}
		loading = false;
	}

	async function approve(userId: string, platform: string) {
		try {
			await api.approveUser(userId, platform);
			await loadUsers();
		} catch (_) {}
	}

	async function block(userId: string, platform: string) {
		try {
			await api.blockUser(userId, platform);
			await loadUsers();
		} catch (_) {}
	}

	onMount(() => {
		loadUsers();
		const interval = setInterval(loadUsers, 5000);
		return () => clearInterval(interval);
	});
</script>

<div class="p-8 max-w-4xl">
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Users</h2>
		<p class="text-sm text-gray-500 mt-1">Manage bot users and permissions</p>
	</div>

	{#if loading}
		<p class="text-sm text-gray-500">Loading...</p>
	{:else if users.length === 0}
		<div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
			<p class="text-sm text-gray-500">No users registered yet</p>
		</div>
	{:else}
		<div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-gray-100">
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">User</th>
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Platform</th>
						<th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Status</th>
						<th class="text-right px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each users as user}
						<tr class="border-b border-gray-50 hover:bg-gray-50 transition-colors">
							<td class="px-4 py-3 text-gray-900">{user.user_id}</td>
							<td class="px-4 py-3 text-gray-500">{user.platform}</td>
							<td class="px-4 py-3">
								<span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full
									{user.status === 'Admin' ? 'bg-purple-100 text-purple-700' :
									 user.status === 'Approved' ? 'bg-green-100 text-green-700' :
									 user.status === 'Pending' ? 'bg-yellow-100 text-yellow-700' :
									 user.status === 'Blocked' ? 'bg-red-100 text-red-700' :
									 'bg-gray-100 text-gray-700'}">
									{user.status}
								</span>
							</td>
							<td class="px-4 py-3 text-right">
								{#if user.status === 'Pending' || user.status === 'New'}
									<button onclick={() => approve(user.user_id, user.platform)}
										class="text-xs text-green-600 hover:text-green-800 font-medium mr-3">Approve</button>
								{/if}
								{#if user.status !== 'Blocked' && user.status !== 'Admin'}
									<button onclick={() => block(user.user_id, user.platform)}
										class="text-xs text-red-600 hover:text-red-800 font-medium">Block</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
