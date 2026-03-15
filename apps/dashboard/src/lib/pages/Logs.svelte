<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch } from '../stores/api'
  import { PageHeader } from '@amanclaw/ui'
  import { Loader2 } from '@amanclaw/ui'

  let logs: any[] = $state([])
  let loading = $state(true)
  let filter = $state('')
  let autoScroll = $state(true)
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

  let filteredLogs = $derived(
    filter
      ? logs.filter(l => JSON.stringify(l).toLowerCase().includes(filter.toLowerCase()))
      : logs
  )
</script>

<div class="flex flex-col h-full">
  <PageHeader title="Logs">
    {#snippet action()}
      <label class="flex items-center gap-2 text-sm text-fg-muted">
        <input type="checkbox" bind:checked={autoScroll} class="rounded" />
        Auto-scroll
      </label>
    {/snippet}
  </PageHeader>

  <input
    type="text"
    bind:value={filter}
    placeholder="Filter logs..."
    class="w-full max-w-md mb-4 px-4 py-2 rounded-lg border border-border bg-surface text-fg outline-none focus:ring-2 focus:ring-primary-500/50"
  />

  <div class="flex-1 bg-base rounded-xl p-4 overflow-auto font-mono text-xs text-fg-secondary min-h-[400px] border border-border">
    {#if loading}
      <div class="flex items-center gap-2 text-fg-muted">
        <Loader2 size={14} class="animate-spin" />
        <span>Loading...</span>
      </div>
    {:else if filteredLogs.length === 0}
      <p class="text-fg-muted">No logs yet. Logs will appear here as events occur.</p>
    {:else}
      {#each filteredLogs as log}
        <div class="py-0.5 hover:bg-[var(--color-elevated-50)]">
          <span class="text-fg-muted">{log.timestamp || ''}</span>
          <span class="{log.level === 'ERROR' ? 'text-red-400' : log.level === 'WARN' ? 'text-amber-400' : 'text-green-400'}">[{log.level || 'INFO'}]</span>
          <span class="text-fg-secondary">{log.message || JSON.stringify(log)}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>
