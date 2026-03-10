<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'

  let communities: any[] = []
  let loading = true
  let showForm = false
  let form = { name: '', zone: 'SGR01', language: 'ms', platform: 'telegram', platform_group_id: '' }

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

<div class="p-6 md:p-8">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Communities</h2>
    <button on:click={() => showForm = !showForm}
      class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm">
      {showForm ? 'Cancel' : '+ Add'}
    </button>
  </div>

  {#if showForm}
    <form on:submit|preventDefault={createCommunity}
      class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700 mb-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
      <input bind:value={form.name} placeholder="Community name" required
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <input bind:value={form.zone} placeholder="Zone (e.g. SGR01)"
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <input bind:value={form.platform_group_id} placeholder="Group ID" required
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <select bind:value={form.platform}
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white">
        <option value="telegram">Telegram</option>
        <option value="whatsapp-web">WhatsApp</option>
        <option value="discord">Discord</option>
        <option value="slack">Slack</option>
      </select>
      <button type="submit" class="sm:col-span-2 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg">Create</button>
    </form>
  {/if}

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if communities.length === 0}
    <p class="text-gray-500">No communities yet.</p>
  {:else}
    <div class="space-y-3">
      {#each communities as c}
        <div class="bg-white dark:bg-gray-800 rounded-xl p-5 shadow-sm border border-gray-200 dark:border-gray-700 flex items-center justify-between">
          <div>
            <h3 class="font-semibold text-gray-900 dark:text-white">{c.name}</h3>
            <p class="text-sm text-gray-500">{c.platform} · {c.zone} · {c.language}</p>
            {#if c.enabled_skills?.length}
              <p class="text-xs text-gray-400 mt-1">Skills: {c.enabled_skills.join(', ')}</p>
            {/if}
          </div>
          <button on:click={() => deleteCommunity(c.id)}
            class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">Delete</button>
        </div>
      {/each}
    </div>
  {/if}
</div>
