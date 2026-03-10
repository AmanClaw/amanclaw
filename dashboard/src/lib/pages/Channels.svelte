<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let status: any = null
  let loading = true

  onMount(async () => {
    try {
      status = await apiFetch('/status')
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Channels</h2>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if status}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Telegram</h3>
          <StatusBadge status={status.running ? 'online' : 'offline'} />
        </div>
        <p class="text-sm text-gray-500">Bot messaging via Telegram</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">WhatsApp Web</h3>
          <StatusBadge status={status.running ? 'online' : 'offline'} />
        </div>
        <p class="text-sm text-gray-500">Via wa-bridge (WAHA-compatible)</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Discord</h3>
          <StatusBadge status="offline" label="Not configured" />
        </div>
        <p class="text-sm text-gray-500">Discord bot integration</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Slack</h3>
          <StatusBadge status="offline" label="Not configured" />
        </div>
        <p class="text-sm text-gray-500">Slack workspace integration</p>
      </div>
    </div>
  {/if}
</div>
