<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import { PageHeader, Button, Card } from '@amanclaw/ui'
  import { Plus, Trash2, Loader2 } from '@amanclaw/ui'

  let communities: any[] = $state([])
  let loading = $state(true)
  let showForm = $state(false)
  let form = $state({ name: '', zone: 'SGR01', language: 'ms', platform: 'telegram', platform_group_id: '' })

  onMount(loadCommunities)

  async function loadCommunities() {
    loading = true
    try {
      const data = await apiFetch('/communities')
      communities = data.communities
    } catch (e) { console.error(e) }
    finally { loading = false }
  }

  async function createCommunity() {
    await apiFetch('/communities', { method: 'POST', body: JSON.stringify(form) })
    showForm = false
    form = { name: '', zone: 'SGR01', language: 'ms', platform: 'telegram', platform_group_id: '' }
    await loadCommunities()
  }

  async function deleteCommunity(id: string) {
    if (!confirm('Delete this community?')) return
    await apiFetch(`/communities/${id}`, { method: 'DELETE' })
    await loadCommunities()
  }
</script>

<PageHeader title="Communities">
  {#snippet action()}
    <Button size="sm" onclick={() => showForm = !showForm}>
      {#if showForm}
        Cancel
      {:else}
        <Plus size={14} /> Add
      {/if}
    </Button>
  {/snippet}
</PageHeader>

{#if showForm}
  <form onsubmit={(e: Event) => { e.preventDefault(); createCommunity() }}
    class="bg-base rounded-xl p-6 border border-border mb-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
    <input bind:value={form.name} placeholder="Community name" required
      class="px-3 py-2 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50" />
    <input bind:value={form.zone} placeholder="Zone (e.g. SGR01)"
      class="px-3 py-2 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50" />
    <input bind:value={form.platform_group_id} placeholder="Group ID" required
      class="px-3 py-2 rounded-lg border border-border bg-elevated text-fg outline-none focus:ring-2 focus:ring-primary-500/50" />
    <select bind:value={form.platform}
      class="px-3 py-2 rounded-lg border border-border bg-elevated text-fg">
      <option value="telegram">Telegram</option>
      <option value="whatsapp-web">WhatsApp</option>
      <option value="discord">Discord</option>
      <option value="slack">Slack</option>
    </select>
    <div class="sm:col-span-2">
      <Button type="submit">Create</Button>
    </div>
  </form>
{/if}

{#if loading}
  <div class="flex items-center gap-2 text-fg-muted">
    <Loader2 size={16} class="animate-spin" />
    <span class="text-sm">Loading...</span>
  </div>
{:else if communities.length === 0}
  <div class="text-center py-16 bg-base rounded-xl border border-border">
    <p class="text-fg-muted">No communities yet.</p>
  </div>
{:else}
  <div class="space-y-3">
    {#each communities as c}
      <Card>
        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-semibold text-fg">{c.name}</h3>
            <p class="text-[13px] text-fg-muted">{c.platform} · {c.zone} · {c.language}</p>
            {#if c.enabled_skills?.length}
              <p class="text-xs text-fg-muted mt-1">Skills: {c.enabled_skills.join(', ')}</p>
            {/if}
          </div>
          <Button variant="destructive" size="sm" onclick={() => deleteCommunity(c.id)}>
            <Trash2 size={14} /> Delete
          </Button>
        </div>
      </Card>
    {/each}
  </div>
{/if}
