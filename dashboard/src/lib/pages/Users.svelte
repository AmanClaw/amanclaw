<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let users: any[] = []
  let loading = true
  let filter = ''

  onMount(loadUsers)

  async function loadUsers() {
    loading = true
    try {
      const data = await apiFetch('/users')
      users = data.users
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  async function approveUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/approve`, { method: 'POST' })
    await loadUsers()
  }

  async function blockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/block`, { method: 'POST' })
    await loadUsers()
  }

  $: filteredUsers = users.filter(u =>
    u.user_id.includes(filter) || u.platform.includes(filter) || u.state.toLowerCase().includes(filter.toLowerCase())
  )
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Users</h2>

  <input
    type="text"
    bind:value={filter}
    placeholder="Search users..."
    class="w-full max-w-md mb-6 px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
  />

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-200 dark:border-gray-700">
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">User ID</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Platform</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Status</th>
              <th class="text-right px-4 py-3 text-xs font-medium text-gray-500 uppercase">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredUsers as user}
              <tr class="border-b border-gray-100 dark:border-gray-700/50">
                <td class="px-4 py-3 text-sm text-gray-900 dark:text-white font-mono">{user.user_id}</td>
                <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-300">{user.platform}</td>
                <td class="px-4 py-3">
                  <StatusBadge
                    status={user.state === 'Approved' ? 'online' : user.state === 'Blocked' ? 'offline' : 'warning'}
                    label={user.state}
                  />
                </td>
                <td class="px-4 py-3 text-right space-x-2">
                  {#if user.state !== 'Approved'}
                    <button on:click={() => approveUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                      Approve
                    </button>
                  {/if}
                  {#if user.state !== 'Blocked'}
                    <button on:click={() => blockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                      Block
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <p class="text-sm text-gray-500 mt-3">{filteredUsers.length} users</p>
  {/if}
</div>
