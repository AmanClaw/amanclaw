<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch } from '../stores/api'

  let logs: any[] = []
  let loading = true
  let filter = ''
  let autoScroll = true
  let interval: any

  onMount(() => {
    loadLogs()
    interval = setInterval(loadLogs, 3000)
  })

  onDestroy(() => clearInterval(interval))

  async function loadLogs() {
    try {
      const data = await apiFetch('/logs')
      logs = data.logs || []
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  $: filteredLogs = filter
    ? logs.filter(l => JSON.stringify(l).toLowerCase().includes(filter.toLowerCase()))
    : logs
</script>

<div class="p-6 md:p-8 flex flex-col h-full">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Logs</h2>
    <label class="flex items-center gap-2 text-sm text-gray-500">
      <input type="checkbox" bind:checked={autoScroll} />
      Auto-scroll
    </label>
  </div>

  <input
    type="text"
    bind:value={filter}
    placeholder="Filter logs..."
    class="w-full max-w-md mb-4 px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
  />

  <div class="flex-1 bg-gray-900 rounded-xl p-4 overflow-auto font-mono text-xs text-gray-300 min-h-100">
    {#if loading}
      <p class="text-gray-500">Loading...</p>
    {:else if filteredLogs.length === 0}
      <p class="text-gray-500">No logs yet. Logs will appear here as events occur.</p>
    {:else}
      {#each filteredLogs as log}
        <div class="py-0.5 hover:bg-gray-800/50">
          <span class="text-gray-500">{log.timestamp || ''}</span>
          <span class="{log.level === 'ERROR' ? 'text-red-400' : log.level === 'WARN' ? 'text-yellow-400' : 'text-green-400'}">[{log.level || 'INFO'}]</span>
          <span>{log.message || JSON.stringify(log)}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>
