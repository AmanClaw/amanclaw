<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let users: any[] = []
  let loading = true
  let search = ''
  let platformFilter = ''
  let statusFilter = ''
  let selectedUser: any = null
  let userHistory: any[] = []
  let historyLoading = false
  let historyTotal = 0

  const platforms = ['', 'telegram', 'discord', 'whatsapp', 'whatsapp-web', 'slack']
  const statuses = ['', 'pending', 'approved', 'blocked']
  const platformIcons: Record<string, string> = {
    telegram: '\u2708\uFE0F',
    discord: '\uD83C\uDFAE',
    whatsapp: '\uD83D\uDCAC',
    'whatsapp-web': '\uD83D\uDCAC',
    slack: '\uD83D\uDCBC',
  }

  onMount(loadUsers)

  async function loadUsers() {
    loading = true
    try {
      const params = new URLSearchParams()
      if (platformFilter) params.set('platform', platformFilter)
      if (statusFilter) params.set('status', statusFilter)
      if (search) params.set('search', search)
      const qs = params.toString()
      const data = await apiFetch(`/users${qs ? '?' + qs : ''}`)
      users = data.users
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  async function showUser(platform: string, userId: string) {
    try {
      selectedUser = await apiFetch(`/users/${platform}/${userId}`)
      await loadHistory(platform, userId)
    } catch (e) {
      console.error(e)
    }
  }

  async function loadHistory(platform: string, userId: string, offset = 0) {
    historyLoading = true
    try {
      const data = await apiFetch(`/users/${platform}/${userId}/history?limit=20&offset=${offset}`)
      userHistory = data.messages
      historyTotal = data.total
    } catch (e) {
      console.error(e)
    } finally {
      historyLoading = false
    }
  }

  async function approveUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/approve`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'approved'
  }

  async function blockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/block`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'blocked'
  }

  async function unblockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/unblock`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'pending'
  }

  function closeDetail() {
    selectedUser = null
    userHistory = []
  }

  function formatDate(d: string | null) {
    if (!d) return '-'
    return new Date(d).toLocaleString()
  }
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Users</h2>

  <!-- Filters -->
  <div class="flex flex-wrap gap-3 mb-6">
    <input
      type="text"
      bind:value={search}
      on:input={loadUsers}
      placeholder="Search users..."
      class="flex-1 min-w-48 px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
    />
    <select
      bind:value={platformFilter}
      on:change={loadUsers}
      class="px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
    >
      <option value="">All platforms</option>
      {#each platforms.slice(1) as p}
        <option value={p}>{p}</option>
      {/each}
    </select>
    <select
      bind:value={statusFilter}
      on:change={loadUsers}
      class="px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
    >
      <option value="">All statuses</option>
      {#each statuses.slice(1) as s}
        <option value={s}>{s}</option>
      {/each}
    </select>
  </div>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-200 dark:border-gray-700">
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Platform</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">User</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Status</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Last Seen</th>
              <th class="text-right px-4 py-3 text-xs font-medium text-gray-500 uppercase">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user}
              <tr class="border-b border-gray-100 dark:border-gray-700/50 hover:bg-gray-50 dark:hover:bg-gray-700/30 cursor-pointer"
                  on:click={() => showUser(user.platform, user.user_id)}>
                <td class="px-4 py-3 text-sm">
                  <span class="mr-1">{platformIcons[user.platform] || ''}</span>
                  <span class="text-gray-600 dark:text-gray-300">{user.platform}</span>
                </td>
                <td class="px-4 py-3 text-sm">
                  <div class="text-gray-900 dark:text-white font-mono">{user.user_id}</div>
                  {#if user.username || user.first_name}
                    <div class="text-xs text-gray-500">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
                  {/if}
                </td>
                <td class="px-4 py-3">
                  <StatusBadge
                    status={user.state === 'approved' ? 'online' : user.state === 'blocked' ? 'offline' : 'warning'}
                    label={user.state}
                  />
                </td>
                <td class="px-4 py-3 text-sm text-gray-500">{formatDate(user.last_seen)}</td>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <td class="px-4 py-3 text-right space-x-2" on:click|stopPropagation>
                  {#if user.state !== 'approved'}
                    <button on:click={() => approveUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                      Approve
                    </button>
                  {/if}
                  {#if user.state === 'blocked'}
                    <button on:click={() => unblockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg">
                      Unblock
                    </button>
                  {:else if user.state !== 'blocked'}
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
    <p class="text-sm text-gray-500 mt-3">{users.length} users</p>
  {/if}
</div>

<!-- User Detail Modal -->
{#if selectedUser}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-16 px-4" on:click={closeDetail}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
         on:click|stopPropagation>
      <div class="p-6">
        <!-- Header -->
        <div class="flex justify-between items-start mb-6">
          <div>
            <h3 class="text-xl font-bold text-gray-900 dark:text-white">
              {platformIcons[selectedUser.platform] || ''} {selectedUser.first_name || selectedUser.user_id}
            </h3>
            <p class="text-sm text-gray-500">
              {selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
              {#if selectedUser.username} &middot; @{selectedUser.username}{/if}
            </p>
          </div>
          <button on:click={closeDetail} class="text-gray-400 hover:text-gray-600 text-xl">&times;</button>
        </div>

        <!-- Info Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Status</p>
            <p class="font-semibold text-gray-900 dark:text-white capitalize">{selectedUser.state}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Messages</p>
            <p class="font-semibold text-gray-900 dark:text-white">{selectedUser.message_count}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">First Seen</p>
            <p class="font-semibold text-gray-900 dark:text-white text-xs">{formatDate(selectedUser.first_seen)}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Last Seen</p>
            <p class="font-semibold text-gray-900 dark:text-white text-xs">{formatDate(selectedUser.last_seen)}</p>
          </div>
        </div>

        <!-- Facts -->
        {#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Learned Facts</h4>
            <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
              {#each Object.entries(selectedUser.facts) as [key, value]}
                <div class="flex justify-between py-1 text-sm">
                  <span class="text-gray-500">{key}</span>
                  <span class="text-gray-900 dark:text-white">{value}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Conversation History -->
        <div>
          <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
            Recent Conversations ({historyTotal} total)
          </h4>
          {#if historyLoading}
            <p class="text-gray-500 text-sm">Loading...</p>
          {:else}
            <div class="space-y-2 max-h-60 overflow-auto">
              {#each userHistory as msg}
                <div class="text-sm p-2 rounded-lg {msg.role === 'user' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-900 dark:text-blue-100' : 'bg-gray-50 dark:bg-gray-700/50 text-gray-900 dark:text-white'}">
                  <span class="text-xs font-medium text-gray-500 uppercase">{msg.role}</span>
                  <p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
                </div>
              {/each}
              {#if userHistory.length === 0}
                <p class="text-gray-500 text-sm">No conversation history</p>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Actions -->
        <div class="flex gap-2 mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
          {#if selectedUser.state !== 'approved'}
            <button on:click={() => approveUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm">
              Approve
            </button>
          {/if}
          {#if selectedUser.state === 'blocked'}
            <button on:click={() => unblockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg text-sm">
              Unblock
            </button>
          {:else if selectedUser.state !== 'blocked'}
            <button on:click={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
