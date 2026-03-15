<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch, addUser, makeAdmin, removeAdmin, getUserStats } from '../stores/api'
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

  // Stats
  let stats: any = null

  // Add User modal
  let showAddModal = false
  let addForm = { platform: 'telegram', user_id: '', username: '', first_name: '', state: 'approved' }
  let addError = ''
  let addLoading = false

  // Polling
  let pollInterval: ReturnType<typeof setInterval> | null = null

  const platforms = ['', 'telegram', 'discord', 'whatsapp', 'whatsapp-web', 'slack']
  const statuses = ['', 'pending', 'approved', 'admin', 'blocked']
  const platformIcons: Record<string, string> = {
    telegram: '\u2708\uFE0F',
    discord: '\uD83C\uDFAE',
    whatsapp: '\uD83D\uDCAC',
    'whatsapp-web': '\uD83D\uDCAC',
    slack: '\uD83D\uDCBC',
  }

  onMount(() => {
    loadUsers()
    loadStats()
    pollInterval = setInterval(() => {
      loadUsers()
      loadStats()
    }, 5000)
  })

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval)
  })

  async function loadStats() {
    try {
      stats = await getUserStats()
    } catch (e) {
      console.error(e)
    }
  }

  async function loadUsers() {
    loading = users.length === 0
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
    await loadStats()
    if (selectedUser?.user_id === userId) selectedUser.state = 'approved'
  }

  async function blockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/block`, { method: 'PUT' })
    await loadUsers()
    await loadStats()
    if (selectedUser?.user_id === userId) selectedUser.state = 'blocked'
  }

  async function unblockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/unblock`, { method: 'PUT' })
    await loadUsers()
    await loadStats()
    if (selectedUser?.user_id === userId) selectedUser.state = 'pending'
  }

  async function doMakeAdmin(platform: string, userId: string) {
    await makeAdmin(platform, userId)
    await loadUsers()
    await loadStats()
    if (selectedUser?.user_id === userId) selectedUser.state = 'admin'
  }

  async function doRemoveAdmin(platform: string, userId: string) {
    await removeAdmin(platform, userId)
    await loadUsers()
    await loadStats()
    if (selectedUser?.user_id === userId) selectedUser.state = 'approved'
  }

  async function submitAddUser() {
    addError = ''
    if (!addForm.user_id.trim()) {
      addError = 'User ID is required'
      return
    }
    addLoading = true
    try {
      const body: any = { user_id: addForm.user_id.trim(), platform: addForm.platform, state: addForm.state }
      if (addForm.username.trim()) body.username = addForm.username.trim()
      if (addForm.first_name.trim()) body.first_name = addForm.first_name.trim()
      await addUser(body)
      showAddModal = false
      addForm = { platform: 'telegram', user_id: '', username: '', first_name: '', state: 'approved' }
      await loadUsers()
      await loadStats()
    } catch (e: any) {
      addError = e?.message || 'Failed to add user'
    } finally {
      addLoading = false
    }
  }

  function closeDetail() {
    selectedUser = null
    userHistory = []
  }

  function closeAddModal() {
    showAddModal = false
    addError = ''
  }

  function formatDate(d: string | null) {
    if (!d) return '-'
    return new Date(d).toLocaleString()
  }

  function statusBadgeStatus(state: string): 'online' | 'offline' | 'warning' {
    if (state === 'approved') return 'online'
    if (state === 'blocked') return 'offline'
    return 'warning'
  }
</script>

<div class="p-6 md:p-8">
  <!-- Header with Add User button -->
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Users</h2>
    <button
      on:click={() => (showAddModal = true)}
      class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium flex items-center gap-1.5"
    >
      + Add User
    </button>
  </div>

  <!-- Stats Cards -->
  {#if stats}
    <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4 mb-6">
      <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700">
        <p class="text-xs text-gray-500 dark:text-gray-400 uppercase">Total</p>
        <p class="text-2xl font-bold text-gray-900 dark:text-white mt-1">{stats.total ?? 0}</p>
        {#if stats.by_platform && Object.keys(stats.by_platform).length > 0}
          <div class="flex flex-wrap gap-1 mt-2">
            {#each Object.entries(stats.by_platform) as [p, count]}
              <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300">
                {platformIcons[p] || ''} {p}: {count}
              </span>
            {/each}
          </div>
        {/if}
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-purple-200 dark:border-purple-800">
        <p class="text-xs text-purple-600 dark:text-purple-400 uppercase">Admin</p>
        <p class="text-2xl font-bold text-purple-700 dark:text-purple-300 mt-1">{stats.admin ?? 0}</p>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-yellow-200 dark:border-yellow-800">
        <p class="text-xs text-yellow-600 dark:text-yellow-400 uppercase">Pending</p>
        <p class="text-2xl font-bold text-yellow-700 dark:text-yellow-300 mt-1">{stats.pending ?? 0}</p>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-green-200 dark:border-green-800">
        <p class="text-xs text-green-600 dark:text-green-400 uppercase">Approved</p>
        <p class="text-2xl font-bold text-green-700 dark:text-green-300 mt-1">{stats.approved ?? 0}</p>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-red-200 dark:border-red-800">
        <p class="text-xs text-red-600 dark:text-red-400 uppercase">Blocked</p>
        <p class="text-2xl font-bold text-red-700 dark:text-red-300 mt-1">{stats.blocked ?? 0}</p>
      </div>
    </div>
  {/if}

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
                  {#if user.state === 'admin'}
                    <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400">
                      <span class="w-2 h-2 rounded-full bg-purple-500"></span>
                      admin
                    </span>
                  {:else}
                    <StatusBadge
                      status={statusBadgeStatus(user.state)}
                      label={user.state}
                    />
                  {/if}
                </td>
                <td class="px-4 py-3 text-sm text-gray-500">{formatDate(user.last_seen)}</td>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <td class="px-4 py-3 text-right space-x-2" on:click|stopPropagation>
                  {#if user.state === 'admin'}
                    <button on:click={() => doRemoveAdmin(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-purple-600 hover:bg-purple-700 text-white rounded-lg">
                      Remove Admin
                    </button>
                  {:else if user.state === 'approved'}
                    <button on:click={() => doMakeAdmin(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 text-purple-600 dark:text-purple-400 border border-purple-300 dark:border-purple-600 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg">
                      Make Admin
                    </button>
                    <button on:click={() => blockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                      Block
                    </button>
                  {:else if user.state === 'pending'}
                    <button on:click={() => approveUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                      Approve
                    </button>
                    <button on:click={() => blockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                      Block
                    </button>
                  {:else if user.state === 'blocked'}
                    <button on:click={() => unblockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg">
                      Unblock
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
            <p class="font-semibold capitalize {selectedUser.state === 'admin' ? 'text-purple-700 dark:text-purple-400' : 'text-gray-900 dark:text-white'}">{selectedUser.state}</p>
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
          {#if selectedUser.state === 'admin'}
            <button on:click={() => doRemoveAdmin(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm">
              Remove Admin
            </button>
          {:else if selectedUser.state === 'approved'}
            <button on:click={() => doMakeAdmin(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 text-purple-600 dark:text-purple-400 border border-purple-300 dark:border-purple-600 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg text-sm">
              Make Admin
            </button>
            <button on:click={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {:else if selectedUser.state === 'pending'}
            <button on:click={() => approveUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm">
              Approve
            </button>
            <button on:click={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {:else if selectedUser.state === 'blocked'}
            <button on:click={() => unblockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg text-sm">
              Unblock
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Add User Modal -->
{#if showAddModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-16 px-4" on:click={closeAddModal}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md"
         on:click|stopPropagation>
      <div class="p-6">
        <div class="flex justify-between items-center mb-6">
          <h3 class="text-lg font-bold text-gray-900 dark:text-white">Add User</h3>
          <button on:click={closeAddModal} class="text-gray-400 hover:text-gray-600 text-xl">&times;</button>
        </div>

        {#if addError}
          <div class="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-700 dark:text-red-400">
            {addError}
          </div>
        {/if}

        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Platform</label>
            <select
              bind:value={addForm.platform}
              class="w-full px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            >
              {#each platforms.slice(1) as p}
                <option value={p}>{p}</option>
              {/each}
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">User ID <span class="text-red-500">*</span></label>
            <input
              type="text"
              bind:value={addForm.user_id}
              placeholder="e.g. 123456789"
              class="w-full px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Username <span class="text-gray-400">(optional)</span></label>
            <input
              type="text"
              bind:value={addForm.username}
              placeholder="e.g. johndoe"
              class="w-full px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">First Name <span class="text-gray-400">(optional)</span></label>
            <input
              type="text"
              bind:value={addForm.first_name}
              placeholder="e.g. John"
              class="w-full px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Initial Status</label>
            <select
              bind:value={addForm.state}
              class="w-full px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            >
              <option value="approved">Approved</option>
              <option value="pending">Pending</option>
            </select>
          </div>
        </div>

        <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
          <button on:click={closeAddModal}
            class="px-4 py-2 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm">
            Cancel
          </button>
          <button on:click={submitAddUser}
            disabled={addLoading}
            class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-sm">
            {addLoading ? 'Adding...' : 'Add User'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
