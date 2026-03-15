<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch, addUser, makeAdmin, removeAdmin, getUserStats } from '../stores/api'
  import { PageHeader, Badge, Button, Card } from '@amanclaw/ui'
  import { Plus, X, Loader2 } from '@amanclaw/ui'

  let users: any[] = $state([])
  let loading = $state(true)
  let search = $state('')
  let platformFilter = $state('')
  let statusFilter = $state('')
  let selectedUser: any = $state(null)
  let userHistory: any[] = $state([])
  let historyLoading = $state(false)
  let historyTotal = $state(0)

  // Stats
  let stats: any = $state(null)

  // Add User modal
  let showAddModal = $state(false)
  let addForm = $state({ platform: 'telegram', user_id: '', username: '', first_name: '', state: 'approved' })
  let addError = $state('')
  let addLoading = $state(false)

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

  function statusVariant(state: string): 'success' | 'warning' | 'error' | 'accent' | 'muted' {
    if (state === 'approved') return 'success'
    if (state === 'blocked') return 'error'
    if (state === 'admin') return 'accent'
    return 'warning'
  }
</script>

<PageHeader title="Users">
  {#snippet action()}
    <Button onclick={() => (showAddModal = true)} size="sm">
      <Plus size={14} /> Add User
    </Button>
  {/snippet}
</PageHeader>

<!-- Stats Cards -->
{#if stats}
  <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4 mb-6">
    <Card>
      <p class="text-xs text-fg-muted uppercase">Total</p>
      <p class="text-2xl font-bold text-fg mt-1">{stats.total ?? 0}</p>
      {#if stats.by_platform && Object.keys(stats.by_platform).length > 0}
        <div class="flex flex-wrap gap-1 mt-2">
          {#each Object.entries(stats.by_platform) as [p, count]}
            <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-elevated text-fg-secondary">
              {platformIcons[p] || ''} {p}: {count}
            </span>
          {/each}
        </div>
      {/if}
    </Card>
    <div class="bg-base rounded-xl p-4 border border-purple-500/20">
      <p class="text-xs text-purple-400 uppercase">Admin</p>
      <p class="text-2xl font-bold text-purple-300 mt-1">{stats.admin ?? 0}</p>
    </div>
    <div class="bg-base rounded-xl p-4 border border-amber-500/20">
      <p class="text-xs text-amber-400 uppercase">Pending</p>
      <p class="text-2xl font-bold text-amber-300 mt-1">{stats.pending ?? 0}</p>
    </div>
    <div class="bg-base rounded-xl p-4 border border-green-500/20">
      <p class="text-xs text-green-400 uppercase">Approved</p>
      <p class="text-2xl font-bold text-green-300 mt-1">{stats.approved ?? 0}</p>
    </div>
    <div class="bg-base rounded-xl p-4 border border-red-500/20">
      <p class="text-xs text-red-400 uppercase">Blocked</p>
      <p class="text-2xl font-bold text-red-300 mt-1">{stats.blocked ?? 0}</p>
    </div>
  </div>
{/if}

<!-- Filters -->
<div class="flex flex-wrap gap-3 mb-6">
  <input
    type="text"
    bind:value={search}
    oninput={loadUsers}
    placeholder="Search users..."
    class="flex-1 min-w-48 px-4 py-2.5 rounded-lg border border-border bg-base text-fg outline-none focus:ring-2 focus:ring-primary-500/50"
  />
  <select
    bind:value={platformFilter}
    onchange={loadUsers}
    class="px-4 py-2.5 rounded-lg border border-border bg-base text-fg"
  >
    <option value="">All platforms</option>
    {#each platforms.slice(1) as p}
      <option value={p}>{p}</option>
    {/each}
  </select>
  <select
    bind:value={statusFilter}
    onchange={loadUsers}
    class="px-4 py-2.5 rounded-lg border border-border bg-base text-fg"
  >
    <option value="">All statuses</option>
    {#each statuses.slice(1) as s}
      <option value={s}>{s}</option>
    {/each}
  </select>
</div>

{#if loading}
  <div class="flex items-center gap-2 text-fg-muted">
    <Loader2 size={16} class="animate-spin" />
    <span class="text-sm">Loading...</span>
  </div>
{:else}
  <div class="bg-base rounded-xl border border-border overflow-hidden">
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-border">
            <th class="text-left px-4 py-3 text-xs font-medium text-fg-muted uppercase">Platform</th>
            <th class="text-left px-4 py-3 text-xs font-medium text-fg-muted uppercase">User</th>
            <th class="text-left px-4 py-3 text-xs font-medium text-fg-muted uppercase">Status</th>
            <th class="text-left px-4 py-3 text-xs font-medium text-fg-muted uppercase">Last Seen</th>
            <th class="text-right px-4 py-3 text-xs font-medium text-fg-muted uppercase">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each users as user}
            <tr class="border-b border-border/50 hover:bg-[var(--color-elevated-50)] cursor-pointer"
                onclick={() => showUser(user.platform, user.user_id)}>
              <td class="px-4 py-3 text-sm">
                <span class="mr-1">{platformIcons[user.platform] || ''}</span>
                <span class="text-fg-secondary">{user.platform}</span>
              </td>
              <td class="px-4 py-3 text-sm">
                <div class="text-fg font-mono">{user.user_id}</div>
                {#if user.username || user.first_name}
                  <div class="text-xs text-fg-muted">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
                {/if}
              </td>
              <td class="px-4 py-3">
                <Badge variant={statusVariant(user.state)}>{user.state}</Badge>
              </td>
              <td class="px-4 py-3 text-sm text-fg-muted">{formatDate(user.last_seen)}</td>
              <td class="px-4 py-3 text-right space-x-2" onclick={(e: Event) => e.stopPropagation()}>
                {#if user.state === 'admin'}
                  <button onclick={() => doRemoveAdmin(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 bg-purple-600 hover:bg-purple-700 text-white rounded-lg">
                    Remove Admin
                  </button>
                {:else if user.state === 'approved'}
                  <button onclick={() => doMakeAdmin(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 text-purple-400 border border-purple-500/30 hover:bg-purple-500/10 rounded-lg">
                    Make Admin
                  </button>
                  <button onclick={() => blockUser(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                    Block
                  </button>
                {:else if user.state === 'pending'}
                  <button onclick={() => approveUser(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                    Approve
                  </button>
                  <button onclick={() => blockUser(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                    Block
                  </button>
                {:else if user.state === 'blocked'}
                  <button onclick={() => unblockUser(user.platform, user.user_id)}
                    class="text-xs px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white rounded-lg">
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
  <p class="text-sm text-fg-muted mt-3">{users.length} users</p>
{/if}

<!-- User Detail Modal -->
{#if selectedUser}
  <div class="fixed inset-0 bg-[var(--color-base-80)] backdrop-blur-sm z-50 flex items-start justify-center pt-16 px-4" onclick={closeDetail}>
    <div class="bg-base border border-border rounded-2xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
         onclick={(e: Event) => e.stopPropagation()}>
      <div class="p-6">
        <!-- Header -->
        <div class="flex justify-between items-start mb-6">
          <div>
            <h3 class="text-xl font-bold text-fg">
              {platformIcons[selectedUser.platform] || ''} {selectedUser.first_name || selectedUser.user_id}
            </h3>
            <p class="text-sm text-fg-muted">
              {selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
              {#if selectedUser.username} &middot; @{selectedUser.username}{/if}
            </p>
          </div>
          <button onclick={closeDetail} class="text-fg-muted hover:text-fg transition-colors p-1">
            <X size={18} />
          </button>
        </div>

        <!-- Info Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div class="bg-elevated rounded-lg p-3">
            <p class="text-xs text-fg-muted uppercase">Status</p>
            <p class="font-semibold capitalize {selectedUser.state === 'admin' ? 'text-purple-400' : 'text-fg'}">{selectedUser.state}</p>
          </div>
          <div class="bg-elevated rounded-lg p-3">
            <p class="text-xs text-fg-muted uppercase">Messages</p>
            <p class="font-semibold text-fg">{selectedUser.message_count}</p>
          </div>
          <div class="bg-elevated rounded-lg p-3">
            <p class="text-xs text-fg-muted uppercase">First Seen</p>
            <p class="font-semibold text-fg text-xs">{formatDate(selectedUser.first_seen)}</p>
          </div>
          <div class="bg-elevated rounded-lg p-3">
            <p class="text-xs text-fg-muted uppercase">Last Seen</p>
            <p class="font-semibold text-fg text-xs">{formatDate(selectedUser.last_seen)}</p>
          </div>
        </div>

        <!-- Facts -->
        {#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-fg-secondary mb-2">Learned Facts</h4>
            <div class="bg-elevated rounded-lg p-3">
              {#each Object.entries(selectedUser.facts) as [key, value]}
                <div class="flex justify-between py-1 text-sm">
                  <span class="text-fg-muted">{key}</span>
                  <span class="text-fg">{value}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Conversation History -->
        <div>
          <h4 class="text-sm font-semibold text-fg-secondary mb-2">
            Recent Conversations ({historyTotal} total)
          </h4>
          {#if historyLoading}
            <p class="text-fg-muted text-sm">Loading...</p>
          {:else}
            <div class="space-y-2 max-h-60 overflow-auto">
              {#each userHistory as msg}
                <div class="text-sm p-2 rounded-lg {msg.role === 'user' ? 'bg-[var(--color-primary-500-10)] text-fg' : 'bg-elevated text-fg'}">
                  <span class="text-xs font-medium text-fg-muted uppercase">{msg.role}</span>
                  <p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
                </div>
              {/each}
              {#if userHistory.length === 0}
                <p class="text-fg-muted text-sm">No conversation history</p>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Actions -->
        <div class="flex gap-2 mt-6 pt-4 border-t border-border">
          {#if selectedUser.state === 'admin'}
            <button onclick={() => doRemoveAdmin(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm">
              Remove Admin
            </button>
          {:else if selectedUser.state === 'approved'}
            <button onclick={() => doMakeAdmin(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 text-purple-400 border border-purple-500/30 hover:bg-purple-500/10 rounded-lg text-sm">
              Make Admin
            </button>
            <button onclick={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {:else if selectedUser.state === 'pending'}
            <button onclick={() => approveUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm">
              Approve
            </button>
            <button onclick={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {:else if selectedUser.state === 'blocked'}
            <button onclick={() => unblockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white rounded-lg text-sm">
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
  <div class="fixed inset-0 bg-[var(--color-base-80)] backdrop-blur-sm z-50 flex items-start justify-center pt-16 px-4" onclick={closeAddModal}>
    <div class="bg-base border border-border rounded-2xl shadow-xl w-full max-w-md"
         onclick={(e: Event) => e.stopPropagation()}>
      <div class="p-6">
        <div class="flex justify-between items-center mb-6">
          <h3 class="text-lg font-bold text-fg">Add User</h3>
          <button onclick={closeAddModal} class="text-fg-muted hover:text-fg transition-colors p-1">
            <X size={18} />
          </button>
        </div>

        {#if addError}
          <div class="mb-4 p-3 bg-[var(--color-error-15)] border border-[var(--color-error-20)] rounded-lg text-sm text-red-400">
            {addError}
          </div>
        {/if}

        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-fg-secondary mb-1">Platform</label>
            <select
              bind:value={addForm.platform}
              class="w-full px-4 py-2.5 rounded-lg border border-border bg-elevated text-fg"
            >
              {#each platforms.slice(1) as p}
                <option value={p}>{p}</option>
              {/each}
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium text-fg-secondary mb-1">User ID <span class="text-red-400">*</span></label>
            <input
              type="text"
              bind:value={addForm.user_id}
              placeholder="e.g. 123456789"
              class="w-full px-4 py-2.5 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-fg-secondary mb-1">Username <span class="text-fg-muted">(optional)</span></label>
            <input
              type="text"
              bind:value={addForm.username}
              placeholder="e.g. johndoe"
              class="w-full px-4 py-2.5 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-fg-secondary mb-1">First Name <span class="text-fg-muted">(optional)</span></label>
            <input
              type="text"
              bind:value={addForm.first_name}
              placeholder="e.g. John"
              class="w-full px-4 py-2.5 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-fg-secondary mb-1">Initial Status</label>
            <select
              bind:value={addForm.state}
              class="w-full px-4 py-2.5 rounded-lg border border-border bg-elevated text-fg"
            >
              <option value="approved">Approved</option>
              <option value="pending">Pending</option>
            </select>
          </div>
        </div>

        <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-border">
          <Button variant="secondary" size="sm" onclick={closeAddModal}>Cancel</Button>
          <Button size="sm" onclick={submitAddUser} disabled={addLoading}>
            {addLoading ? 'Adding...' : 'Add User'}
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
