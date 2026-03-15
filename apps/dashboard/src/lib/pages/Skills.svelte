<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import { PageHeader, Card, Badge } from '@amanclaw/ui'
  import { Zap, Loader2 } from '@amanclaw/ui'

  let skills: any[] = $state([])
  let loading = $state(true)

  onMount(async () => {
    try {
      const data = await apiFetch('/skills')
      skills = data.skills
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })
</script>

<PageHeader title="Skills" subtitle="{skills.length} skills registered" />

{#if loading}
  <div class="flex items-center gap-2 text-fg-muted">
    <Loader2 size={16} class="animate-spin" />
    <span class="text-sm">Loading...</span>
  </div>
{:else}
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
    {#each skills as skill}
      <Card>
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-[var(--color-primary-500-10)] flex items-center justify-center">
              <Zap size={14} class="text-primary-500" />
            </div>
            <h3 class="font-semibold text-fg">{skill.name}</h3>
          </div>
          <Badge variant="success">
            <span class="flex items-center gap-1">
              <span class="w-1.5 h-1.5 rounded-full bg-green-400"></span>
              Active
            </span>
          </Badge>
        </div>
        <p class="text-[13px] text-fg-muted line-clamp-2">{skill.description}</p>
      </Card>
    {/each}
  </div>
{/if}
