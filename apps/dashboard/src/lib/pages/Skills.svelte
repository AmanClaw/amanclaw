<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'

  let skills: any[] = []
  let loading = true

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

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Skills</h2>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each skills as skill}
        <div class="bg-white dark:bg-gray-800 rounded-xl p-5 shadow-sm border border-gray-200 dark:border-gray-700">
          <div class="flex items-center justify-between mb-2">
            <h3 class="font-semibold text-gray-900 dark:text-white">{skill.name}</h3>
            <span class="w-2 h-2 rounded-full bg-green-500"></span>
          </div>
          <p class="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">{skill.description}</p>
        </div>
      {/each}
    </div>
    <p class="text-sm text-gray-500 mt-4">{skills.length} skills registered</p>
  {/if}
</div>
