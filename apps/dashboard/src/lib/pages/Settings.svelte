<script lang="ts">
  import { onMount } from 'svelte'
  import { isLoggedIn } from '../stores/auth'
  import { apiFetch } from '../stores/api'
  import { PageHeader, Card, Button } from '@amanclaw/ui'
  import { RefreshCw, LogOut, Info, Database, Loader2 } from '@amanclaw/ui'

  let islamicStatus: any[] | null = $state(null)
  let syncing = $state(false)

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

<PageHeader title="Settings" />

<div class="space-y-4 max-w-lg">
  <Card>
    <div class="flex items-center gap-2 mb-4">
      <Database size={16} class="text-primary-500" />
      <h3 class="font-semibold text-fg">Islamic Knowledge Data</h3>
    </div>

    {#if islamicStatus && islamicStatus.length > 0}
      <div class="space-y-2 mb-4">
        {#each islamicStatus as dataset}
          <div class="flex items-center justify-between text-sm">
            <span class="text-fg-secondary">{dataset.dataset}</span>
            <span class="text-fg-muted">{dataset.record_count} records</span>
            <span class="text-fg-muted text-xs">{dataset.last_synced ? dataset.last_synced.slice(0, 19) : 'never'}</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-sm text-fg-muted mb-4">No Islamic data synced yet.</p>
    {/if}

    <div class="flex gap-2">
      <Button onclick={syncIslamic} disabled={syncing}>
        {#if syncing}
          <Loader2 size={14} class="animate-spin" /> Syncing...
        {:else}
          <RefreshCw size={14} /> Sync All Data
        {/if}
      </Button>
      <Button variant="secondary" onclick={loadIslamicStatus}>
        <RefreshCw size={14} /> Refresh
      </Button>
    </div>
  </Card>

  <Card>
    <div class="flex items-center gap-2 mb-4">
      <LogOut size={16} class="text-red-400" />
      <h3 class="font-semibold text-fg">Account</h3>
    </div>
    <Button variant="destructive" onclick={logout}>
      <LogOut size={14} /> Logout
    </Button>
  </Card>

  <Card>
    <div class="flex items-center gap-2 mb-2">
      <Info size={16} class="text-primary-500" />
      <h3 class="font-semibold text-fg">About</h3>
    </div>
    <p class="text-[13px] text-fg-muted">AmanClaw Management Dashboard</p>
    <p class="text-xs text-fg-muted mt-1">LLM config, bot settings, and advanced options coming in a future update.</p>
  </Card>
</div>
