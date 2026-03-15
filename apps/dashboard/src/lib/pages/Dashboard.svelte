<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import { StatCard, Badge, PageHeader } from '@amanclaw/ui'
  import { Clock, Users, Activity, Zap, User, Loader2, AlertCircle } from '@amanclaw/ui'

  let status: any = $state(null)
  let userStats: any = $state(null)
  let loading = $state(true)

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

<PageHeader title="Dashboard">
  {#snippet action()}
    {#if status}
      <Badge variant={status.running ? 'success' : 'error'}>
        <span class="flex items-center gap-1.5">
          <span class="w-2 h-2 rounded-full {status.running ? 'bg-green-400' : 'bg-red-400'}"></span>
          {status.running ? 'Running' : 'Stopped'}
        </span>
      </Badge>
    {/if}
  {/snippet}
</PageHeader>

{#if loading}
  <div class="flex items-center gap-2 text-fg-muted">
    <Loader2 size={16} class="animate-spin" />
    <span class="text-sm">Loading...</span>
  </div>
{:else if status}
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
    <StatCard label="Uptime" value={formatUptime(status.uptime_seconds)} icon={Clock} />
    <StatCard label="Users" value={status.users_count} icon={Users} />
    <StatCard label="Communities" value={status.communities_count} icon={Activity} />
    <StatCard label="Skills" value={status.skills_count} icon={Zap} />
  </div>

  {#if userStats}
    <h3 class="text-base font-semibold text-fg mb-4">User Breakdown</h3>
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <StatCard label="Total Users" value={userStats.total} icon={User} />
      <StatCard label="Pending" value={userStats.pending} icon={Clock} iconColor="text-amber-400 bg-[var(--color-warning-15)]" />
      <StatCard label="Approved" value={userStats.approved} icon={Activity} iconColor="text-green-400 bg-[var(--color-success-15)]" />
      <StatCard label="Blocked" value={userStats.blocked} icon={AlertCircle} iconColor="text-red-400 bg-[var(--color-error-15)]" />
    </div>

    {#if userStats.by_platform && Object.keys(userStats.by_platform).length > 0}
      <h3 class="text-base font-semibold text-fg mb-4">By Platform</h3>
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {#each Object.entries(userStats.by_platform) as [platform, count]}
          <StatCard label={platform} value={count as number} icon={Activity} />
        {/each}
      </div>
    {/if}
  {/if}
{:else}
  <div class="flex items-center gap-2 text-red-400">
    <AlertCircle size={16} />
    <span class="text-sm">Failed to load status</span>
  </div>
{/if}
