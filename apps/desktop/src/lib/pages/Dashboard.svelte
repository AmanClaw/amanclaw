<script lang="ts">
	import { PageHeader, StatCard, Card, Button, Badge } from '@amanclaw/ui';
	import { Users, Zap, Activity, Play, Square, RefreshCw } from '@amanclaw/ui';
	import { botStatus } from '$lib/stores/app';
	import { api } from '$lib/api';

	async function handleStart() {
		try {
			await api.startEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}

	async function handleStop() {
		try {
			await api.stopEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (_) {}
	}

	async function handleRestart() {
		try {
			botStatus.set({ ...$botStatus, engine_status: 'starting' });
			await api.restartEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}
</script>

<PageHeader title="Dashboard" subtitle="Overview of your AmanClaw instance." />

<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
	<StatCard label="Communities" value={$botStatus.communities} icon={Users} trend="+{$botStatus.communities > 0 ? 1 : 0} new" trendPositive={true} />
	<StatCard label="Active Skills" value={$botStatus.skills} icon={Zap} />
	<StatCard label="Users" value={$botStatus.users} icon={Activity} />
	<StatCard label="Uptime" value={$botStatus.uptime_seconds > 0 ? Math.floor($botStatus.uptime_seconds / 60) + 'm' : '0m'} icon={RefreshCw} />
</div>

<!-- Engine Control -->
<div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
	<div class="lg:col-span-2">
		<Card>
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<span class="w-3 h-3 rounded-full {
						$botStatus.engine_status === 'running' ? 'bg-success' :
						$botStatus.engine_status === 'starting' ? 'bg-warning animate-pulse' :
						$botStatus.engine_status === 'error' ? 'bg-error' :
						'bg-fg-muted'
					}"></span>
					<div>
						<p class="text-sm font-medium text-fg">
							{$botStatus.engine_status === 'running' ? 'Engine Running' :
							 $botStatus.engine_status === 'starting' ? 'Engine Starting...' :
							 $botStatus.engine_status === 'error' ? 'Engine Error' :
							 'Engine Stopped'}
						</p>
						<p class="text-xs text-fg-muted">
							{$botStatus.mode === 'local' ? 'Local Mode' : 'Remote Mode'}
							{#if $botStatus.uptime_seconds > 0}
								 · Uptime: {Math.floor($botStatus.uptime_seconds / 60)}m
							{/if}
						</p>
					</div>
				</div>
				<div class="flex gap-2">
					{#if $botStatus.engine_status === 'running'}
						<Button variant="secondary" size="sm" onclick={handleRestart}>
							<RefreshCw size={12} />
							Restart
						</Button>
						<Button variant="destructive" size="sm" onclick={handleStop}>
							<Square size={12} />
							Stop
						</Button>
					{:else if $botStatus.engine_status !== 'starting'}
						<Button variant="primary" size="sm" onclick={handleStart}>
							<Play size={12} />
							Start
						</Button>
					{/if}
				</div>
			</div>

			{#if $botStatus.error}
				<div class="mt-3 p-2 bg-[var(--color-error-15)] rounded text-xs text-error">
					{$botStatus.error}
				</div>
			{/if}
		</Card>
	</div>

	<Card>
		<h3 class="text-sm font-semibold text-fg mb-3">Recent Activity</h3>
		<p class="text-[13px] text-fg-muted">Activity feed will be populated when the engine is running.</p>
	</Card>
</div>
