<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	interface ChannelStatus {
		id: string;
		platform: string;
		configured: boolean;
		enabled: boolean;
		running: boolean;
		error: string | null;
	}

	let channels = $state<ChannelStatus[]>([]);
	let loading = $state(true);
	let showQr = $state(false);
	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	let qrData = $state<string | null>(null);
	let qrStatus = $state<'idle' | 'loading' | 'scanning' | 'connected' | 'error'>('idle');
	let qrError = $state('');
	let qrPollTimer: ReturnType<typeof setInterval> | null = null;
	let qrRefreshTimer: ReturnType<typeof setInterval> | null = null;

	let wahaUrl = $state('http://localhost:3000');
	let wahaApiKey = $state('');
	let wahaSession = $state('default');
	let wahaPort = $state(8081);
	let editingChannel = $state<string | null>(null);
	let saving = $state(false);

	const channelMeta: Record<string, { label: string; description: string }> = {
		telegram: { label: 'Telegram', description: 'Bot messaging via Telegram' },
		discord: { label: 'Discord', description: 'Discord bot integration' },
		slack: { label: 'Slack', description: 'Slack workspace integration' },
		'whatsapp-cloud': { label: 'WhatsApp Cloud', description: 'Official WhatsApp Business API' },
		'whatsapp-web': { label: 'WhatsApp Web', description: 'Via WAHA bridge — scan QR to connect' },
	};

	const channelOrder = ['telegram', 'whatsapp-web', 'whatsapp-cloud', 'discord', 'slack'];

	async function loadChannels() {
		try {
			const result = await api.listChannels() as any;
			channels = Array.isArray(result) ? result : (result?.channels ?? []);
		} catch (e) {
			console.error('Failed to load channels:', e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadChannels();
		refreshTimer = setInterval(loadChannels, 5000);
	});

	onDestroy(() => {
		if (refreshTimer) clearInterval(refreshTimer);
		stopQrPolling();
	});

	function getChannel(id: string): ChannelStatus {
		return channels.find((c) => c.id === id) || {
			id, platform: id, configured: false, enabled: false, running: false, error: null,
		};
	}

	function statusColor(ch: ChannelStatus): string {
		if (ch.running) return 'border-[var(--color-success-20)]';
		if (ch.error) return 'border-[var(--color-error-20)]';
		if (ch.configured) return 'border-[var(--color-warning-20)]';
		return 'border-border';
	}

	function statusText(ch: ChannelStatus): string {
		if (ch.running) return 'Connected';
		if (ch.error) return 'Error';
		if (ch.configured) return 'Configured';
		return 'Not configured';
	}

	function statusDot(ch: ChannelStatus): string {
		if (ch.running) return 'bg-success';
		if (ch.error) return 'bg-[var(--color-error-15)]0';
		if (ch.configured) return 'bg-[var(--color-warning-15)]0';
		return 'bg-fg-muted';
	}

	async function saveWaConfig() {
		saving = true;
		try {
			await api.saveWhatsappWebConfig({
				wahaUrl, wahaApiKey: wahaApiKey || undefined, session: wahaSession, webhookPort: wahaPort,
			});
			editingChannel = null;
			showQr = true;
			startQrPolling();
			await loadChannels();
		} catch (e) {
			console.error('Save failed:', e);
		} finally {
			saving = false;
		}
	}

	async function handleStart(id: string) {
		try {
			await api.startChannel(id);
			if (id === 'whatsapp-web') {
				showQr = true;
				startQrPolling();
			}
			await loadChannels();
		} catch (e) {
			console.error('Start failed:', e);
		}
	}

	async function handleStop(id: string) {
		try {
			await api.stopChannel(id);
			if (id === 'whatsapp-web') {
				showQr = false;
				stopQrPolling();
			}
			await loadChannels();
		} catch (e) {
			console.error('Stop failed:', e);
		}
	}

	function startQrPolling() {
		qrStatus = 'loading';
		loadQr();
		qrRefreshTimer = setInterval(loadQr, 15000);
		qrPollTimer = setInterval(checkSession, 5000);
	}

	function stopQrPolling() {
		if (qrPollTimer) { clearInterval(qrPollTimer); qrPollTimer = null; }
		if (qrRefreshTimer) { clearInterval(qrRefreshTimer); qrRefreshTimer = null; }
	}

	async function loadQr() {
		try {
			const result = await api.getWhatsappQr() as any;
			if (result.error) { qrStatus = 'error'; qrError = result.error; return; }
			if (result.mimetype && result.data) {
				qrData = `data:${result.mimetype};base64,${result.data}`;
			} else if (result.value) {
				qrData = result.value;
			}
			qrStatus = 'scanning';
		} catch (e: any) {
			qrStatus = 'error';
			qrError = e?.toString() || 'Failed to load QR';
		}
	}

	async function checkSession() {
		try {
			const result = await api.getWhatsappSession() as any;
			const s = result?.status || result?.engine?.state;
			if (s === 'WORKING' || s === 'CONNECTED') {
				qrStatus = 'connected';
				showQr = false;
				stopQrPolling();
				await loadChannels();
			}
		} catch (_) {}
	}
</script>

<div class="max-w-4xl">
	<PageHeader title="Channels" subtitle="Configure, start, and monitor your messaging channels" />

	{#if loading}
		<p class="text-sm text-fg-muted">Loading channels...</p>
	{:else}
		<div class="grid grid-cols-2 gap-3">
			{#each channelOrder as id}
				{@const ch = getChannel(id)}
				{@const meta = channelMeta[id]}
				{#if meta}
					<div class={id === 'whatsapp-web' && (showQr || editingChannel === 'whatsapp-web') ? 'col-span-2' : ''}>
						<div class="bg-surface rounded-xl border {statusColor(ch)} p-5">
							<div class="flex items-center justify-between mb-1">
								<div class="flex items-center gap-2">
									<span class="w-2 h-2 rounded-full {statusDot(ch)}"></span>
									<h3 class="text-sm font-medium text-fg">{meta.label}</h3>
								</div>
								<span class="text-[10px] font-medium px-1.5 py-0.5 rounded
									{ch.running ? 'bg-[var(--color-success-15)] text-success' :
									 ch.error ? 'bg-[var(--color-error-15)] text-error' :
									 ch.configured ? 'bg-[var(--color-warning-15)] text-warning' :
									 'bg-elevated text-fg-muted'}">
									{statusText(ch)}
								</span>
							</div>
							<p class="text-[11px] text-fg-muted mb-3">{meta.description}</p>

							{#if ch.error}
								<div class="mb-2 p-2 bg-[var(--color-error-15)] rounded text-[11px] text-error">{ch.error}</div>
							{/if}

							{#if editingChannel === id && id === 'whatsapp-web'}
								<div class="space-y-2 mb-3">
									<div>
										<label class="block text-[11px] font-medium text-fg-secondary mb-0.5">WAHA URL</label>
										<input type="text" bind:value={wahaUrl} placeholder="http://localhost:3000"
											class="w-full px-3 py-1.5 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
									</div>
									<div class="grid grid-cols-3 gap-2">
										<div>
											<label class="block text-[11px] font-medium text-fg-secondary mb-0.5">API Key</label>
											<input type="password" bind:value={wahaApiKey} placeholder="Optional"
												class="w-full px-3 py-1.5 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
										</div>
										<div>
											<label class="block text-[11px] font-medium text-fg-secondary mb-0.5">Session</label>
											<input type="text" bind:value={wahaSession} placeholder="default"
												class="w-full px-3 py-1.5 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
										</div>
										<div>
											<label class="block text-[11px] font-medium text-fg-secondary mb-0.5">Port</label>
											<input type="number" bind:value={wahaPort} placeholder="8081"
												class="w-full px-3 py-1.5 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
										</div>
									</div>
								</div>
								<div class="flex gap-2">
									<button onclick={saveWaConfig} disabled={saving}
										class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 disabled:opacity-50">
										{saving ? 'Saving...' : 'Save & Connect'}
									</button>
									<button onclick={() => editingChannel = null}
										class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-elevated">
										Cancel
									</button>
								</div>
							{:else}
								<div class="flex gap-2">
									{#if !ch.configured}
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600">
												Setup
											</button>
										{:else}
											<span class="text-[11px] text-fg-muted">Configure in Settings</span>
										{/if}
									{:else if ch.running}
										<button onclick={() => handleStop(id)}
											class="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)]">
											Stop
										</button>
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-elevated">
												Edit
											</button>
										{/if}
									{:else}
										<button onclick={() => handleStart(id)}
											class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600">
											Start
										</button>
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-elevated">
												Edit
											</button>
										{/if}
									{/if}
								</div>
							{/if}
						</div>

						{#if id === 'whatsapp-web' && showQr}
							<div class="mt-2 bg-surface rounded-xl border border-border p-5">
								{#if qrStatus === 'loading'}
									<div class="flex items-center justify-center p-6">
										<div class="animate-spin w-5 h-5 border-2 border-primary-500 border-t-transparent rounded-full"></div>
										<span class="ml-3 text-sm text-fg-muted">Loading QR code...</span>
									</div>
								{:else if qrStatus === 'connected'}
									<div class="flex items-center justify-center p-4 bg-[var(--color-success-15)] rounded-lg">
										<span class="text-sm font-medium text-success">WhatsApp Connected!</span>
									</div>
								{:else if qrStatus === 'error'}
									<div class="p-4">
										<p class="text-sm text-error mb-2">{qrError}</p>
										<button onclick={startQrPolling}
											class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600">
											Retry
										</button>
									</div>
								{:else if qrStatus === 'scanning'}
									<div class="flex flex-col items-center">
										{#if qrData && qrData.startsWith('data:')}
											<img src={qrData} alt="WhatsApp QR Code" class="w-56 h-56 rounded-lg" />
										{:else if qrData}
											<div class="bg-surface p-4 rounded-lg border border-border">
												<p class="text-xs text-fg-muted font-mono break-all">{qrData}</p>
											</div>
										{/if}
										<p class="text-xs text-fg-muted mt-3">Scan this QR code with WhatsApp on your phone</p>
										<p class="text-[10px] text-fg-muted mt-1">QR refreshes automatically every 15 seconds</p>
									</div>
								{/if}
							</div>
						{/if}
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
