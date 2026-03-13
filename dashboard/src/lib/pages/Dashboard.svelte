<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatCard from '../components/StatCard.svelte'
  import StatusBadge from '../components/StatusBadge.svelte'

  let status: any = null
  let userStats: any = null
  let loading = true

  onMount(async () => {
    try {
      const [s, u] = await Promise.all([
        apiFetch('/status'),
        apiFetch('/stats').catch(() => null),
      ])
      status = s
      userStats = u
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })

  function formatUptime(seconds: number): string {
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    return h > 0 ? `${h}h ${m}m` : `${m}m`
  }
</script>

<div class="p-6 md:p-8">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h2>
    {#if status}
      <StatusBadge status={status.running ? 'online' : 'offline'} label={status.running ? 'Running' : 'Stopped'} />
    {/if}
  </div>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if status}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <StatCard label="Uptime" value={formatUptime(status.uptime_seconds)} icon="⏱️" />
      <StatCard label="Users" value={status.users_count} icon="👥" />
      <StatCard label="Communities" value={status.communities_count} icon="🏘️" />
      <StatCard label="Skills" value={status.skills_count} icon="⚡" />
    </div>

    {#if userStats}
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">User Breakdown</h3>
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        <StatCard label="Total Users" value={userStats.total} icon="👤" />
        <StatCard label="Pending" value={userStats.pending} icon="⏳" />
        <StatCard label="Approved" value={userStats.approved} icon="✅" />
        <StatCard label="Blocked" value={userStats.blocked} icon="🚫" />
      </div>

      {#if userStats.by_platform && Object.keys(userStats.by_platform).length > 0}
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">By Platform</h3>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {#each Object.entries(userStats.by_platform) as [platform, count]}
            <StatCard label={platform} value={count} icon="📱" />
          {/each}
        </div>
      {/if}
    {/if}
  {:else}
    <p class="text-red-500">Failed to load status</p>
  {/if}
</div>
