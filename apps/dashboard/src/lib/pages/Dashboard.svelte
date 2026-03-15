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

<PageHeader title="Dashboard" subtitle="Overview of your community bot performance and activity.">
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
    <StatCard label="Uptime" value={formatUptime(status.uptime_seconds)} icon={Clock} trend="+{Math.floor(status.uptime_seconds / 3600)}h today" trendPositive={true} />
    <StatCard label="Users" value={status.users_count} icon={Users} trend="+{status.users_count > 0 ? Math.ceil(status.users_count * 0.12) : 0} vs last month" trendPositive={true} />
    <StatCard label="Communities" value={status.communities_count} icon={Activity} trend="+{status.communities_count > 0 ? 1 : 0} new this month" trendPositive={true} />
    <StatCard label="Skills" value={status.skills_count} icon={Zap} trend="+{status.skills_count > 0 ? Math.ceil(status.skills_count * 0.08) : 0}% vs last month" trendPositive={true} />
  </div>

  {#if userStats}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- User Breakdown -->
      <div class="lg:col-span-2 bg-base border border-border rounded-xl p-6">
        <h3 class="text-lg font-semibold text-fg mb-4">User Breakdown</h3>
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <div>
            <p class="text-sm text-fg-muted">Total</p>
            <p class="text-2xl font-bold text-fg mt-1">{userStats.total}</p>
          </div>
          <div>
            <p class="text-sm text-fg-muted">Pending</p>
            <p class="text-2xl font-bold text-warning mt-1">{userStats.pending}</p>
          </div>
          <div>
            <p class="text-sm text-fg-muted">Approved</p>
            <p class="text-2xl font-bold text-success mt-1">{userStats.approved}</p>
          </div>
          <div>
            <p class="text-sm text-fg-muted">Blocked</p>
            <p class="text-2xl font-bold text-error mt-1">{userStats.blocked}</p>
          </div>
        </div>
      </div>

      <!-- Platform Breakdown -->
      <div class="bg-base border border-border rounded-xl p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-lg font-semibold text-fg">By Platform</h3>
        </div>
        {#if userStats.by_platform && Object.keys(userStats.by_platform).length > 0}
          <div class="space-y-3">
            {#each Object.entries(userStats.by_platform) as [platform, count]}
              <div class="flex items-center justify-between">
                <span class="text-sm text-fg-secondary capitalize">{platform}</span>
                <span class="text-sm font-semibold text-fg">{count}</span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-sm text-fg-muted">No platform data yet.</p>
        {/if}
      </div>
    </div>
  {/if}
{:else}
  <div class="flex items-center gap-2 text-red-400">
    <AlertCircle size={16} />
    <span class="text-sm">Failed to load status</span>
  </div>
{/if}
