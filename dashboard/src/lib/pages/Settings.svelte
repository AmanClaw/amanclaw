<script lang="ts">
  import { onMount } from 'svelte'
  import { isLoggedIn } from '../stores/auth'
  import { apiFetch } from '../stores/api'

  let islamicStatus: any[] | null = null
  let syncing = false

  async function loadIslamicStatus() {
    try {
      const data = await apiFetch('/islamic/status')
      islamicStatus = data.datasets || []
    } catch {
      // Islamic DB may not be configured
    }
  }

  async function syncIslamic() {
    syncing = true
    try {
      await apiFetch('/islamic/sync', {
        method: 'POST',
        body: JSON.stringify({ dataset: 'all' })
      })
      // Poll status after a short delay
      setTimeout(loadIslamicStatus, 3000)
    } catch {}
    syncing = false
  }

  function logout() {
    document.cookie = 'amanclaw_token=; Max-Age=0; Path=/'
    $isLoggedIn = false
    window.location.hash = '#/login'
  }

  onMount(loadIslamicStatus)
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Settings</h2>

  <div class="space-y-4 max-w-lg">
    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Islamic Knowledge Data</h3>

      {#if islamicStatus && islamicStatus.length > 0}
        <div class="space-y-2 mb-4">
          {#each islamicStatus as dataset}
            <div class="flex items-center justify-between text-sm">
              <span class="text-gray-700 dark:text-gray-300">{dataset.dataset}</span>
              <span class="text-gray-500">{dataset.record_count} records</span>
              <span class="text-gray-400 text-xs">{dataset.last_synced ? dataset.last_synced.slice(0, 19) : 'never'}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-sm text-gray-500 mb-4">No Islamic data synced yet.</p>
      {/if}

      <div class="flex gap-2">
        <button on:click={syncIslamic} disabled={syncing}
          class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg text-sm">
          {syncing ? 'Syncing...' : 'Sync All Data'}
        </button>
        <button on:click={loadIslamicStatus}
          class="px-4 py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg text-sm">
          Refresh
        </button>
      </div>
    </div>

    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Account</h3>
      <button on:click={logout}
        class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
        Logout
      </button>
    </div>

    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-2">About</h3>
      <p class="text-sm text-gray-500">AmanClaw Management Dashboard</p>
      <p class="text-xs text-gray-400 mt-1">LLM config, bot settings, and advanced options coming in a future update.</p>
    </div>
  </div>
</div>
