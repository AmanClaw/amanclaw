<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';

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
		if (ch.running) return 'border-green-300';
		if (ch.error) return 'border-red-300';
		if (ch.configured) return 'border-yellow-300';
		return 'border-gray-200';
	}

	function statusText(ch: ChannelStatus): string {
		if (ch.running) return 'Connected';
		if (ch.error) return 'Error';
		if (ch.configured) return 'Configured';
		return 'Not configured';
	}

	function statusDot(ch: ChannelStatus): string {
		if (ch.running) return 'bg-green-500';
		if (ch.error) return 'bg-red-500';
		if (ch.configured) return 'bg-yellow-500';
		return 'bg-gray-400';
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

<div class="p-8 max-w-4xl">
	<div class="mb-6">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Channels</h2>
		<p class="text-sm text-gray-500 mt-1">Configure, start, and monitor your messaging channels</p>
	</div>

	{#if loading}
		<p class="text-sm text-gray-400">Loading channels...</p>
	{:else}
		<div class="grid grid-cols-2 gap-3">
			{#each channelOrder as id}
				{@const ch = getChannel(id)}
				{@const meta = channelMeta[id]}
				{#if meta}
					<div class={id === 'whatsapp-web' && (showQr || editingChannel === 'whatsapp-web') ? 'col-span-2' : ''}>
						<div class="bg-gray-50 rounded-xl border {statusColor(ch)} p-5">
							<div class="flex items-center justify-between mb-1">
								<div class="flex items-center gap-2">
									<span class="w-2 h-2 rounded-full {statusDot(ch)}"></span>
									<h3 class="text-sm font-medium text-gray-900">{meta.label}</h3>
								</div>
								<span class="text-[10px] font-medium px-1.5 py-0.5 rounded
									{ch.running ? 'bg-green-100 text-green-700' :
									 ch.error ? 'bg-red-100 text-red-700' :
									 ch.configured ? 'bg-yellow-100 text-yellow-700' :
									 'bg-gray-100 text-gray-500'}">
									{statusText(ch)}
								</span>
							</div>
							<p class="text-[11px] text-gray-500 mb-3">{meta.description}</p>

							{#if ch.error}
								<div class="mb-2 p-2 bg-red-50 rounded text-[11px] text-red-700">{ch.error}</div>
							{/if}

							{#if editingChannel === id && id === 'whatsapp-web'}
								<div class="space-y-2 mb-3">
									<div>
										<label class="block text-[11px] font-medium text-gray-700 mb-0.5">WAHA URL</label>
										<input type="text" bind:value={wahaUrl} placeholder="http://localhost:3000"
											class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
									</div>
									<div class="grid grid-cols-3 gap-2">
										<div>
											<label class="block text-[11px] font-medium text-gray-700 mb-0.5">API Key</label>
											<input type="password" bind:value={wahaApiKey} placeholder="Optional"
												class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
										</div>
										<div>
											<label class="block text-[11px] font-medium text-gray-700 mb-0.5">Session</label>
											<input type="text" bind:value={wahaSession} placeholder="default"
												class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
										</div>
										<div>
											<label class="block text-[11px] font-medium text-gray-700 mb-0.5">Port</label>
											<input type="number" bind:value={wahaPort} placeholder="8081"
												class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
										</div>
									</div>
								</div>
								<div class="flex gap-2">
									<button onclick={saveWaConfig} disabled={saving}
										class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 disabled:opacity-50">
										{saving ? 'Saving...' : 'Save & Connect'}
									</button>
									<button onclick={() => editingChannel = null}
										class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100">
										Cancel
									</button>
								</div>
							{:else}
								<div class="flex gap-2">
									{#if !ch.configured}
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800">
												Setup
											</button>
										{:else}
											<span class="text-[11px] text-gray-400">Configure in Settings</span>
										{/if}
									{:else if ch.running}
										<button onclick={() => handleStop(id)}
											class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-300 text-red-700 hover:bg-red-50">
											Stop
										</button>
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100">
												Edit
											</button>
										{/if}
									{:else}
										<button onclick={() => handleStart(id)}
											class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800">
											Start
										</button>
										{#if id === 'whatsapp-web'}
											<button onclick={() => editingChannel = id}
												class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100">
												Edit
											</button>
										{/if}
									{/if}
								</div>
							{/if}
						</div>

						{#if id === 'whatsapp-web' && showQr}
							<div class="mt-2 bg-gray-50 rounded-xl border border-gray-200 p-5">
								{#if qrStatus === 'loading'}
									<div class="flex items-center justify-center p-6">
										<div class="animate-spin w-5 h-5 border-2 border-gray-900 border-t-transparent rounded-full"></div>
										<span class="ml-3 text-sm text-gray-500">Loading QR code...</span>
									</div>
								{:else if qrStatus === 'connected'}
									<div class="flex items-center justify-center p-4 bg-green-50 rounded-lg">
										<span class="text-sm font-medium text-green-700">WhatsApp Connected!</span>
									</div>
								{:else if qrStatus === 'error'}
									<div class="p-4">
										<p class="text-sm text-red-700 mb-2">{qrError}</p>
										<button onclick={startQrPolling}
											class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800">
											Retry
										</button>
									</div>
								{:else if qrStatus === 'scanning'}
									<div class="flex flex-col items-center">
										{#if qrData && qrData.startsWith('data:')}
											<img src={qrData} alt="WhatsApp QR Code" class="w-56 h-56 rounded-lg" />
										{:else if qrData}
											<div class="bg-white p-4 rounded-lg border border-gray-200">
												<p class="text-xs text-gray-500 font-mono break-all">{qrData}</p>
											</div>
										{/if}
										<p class="text-xs text-gray-500 mt-3">Scan this QR code with WhatsApp on your phone</p>
										<p class="text-[10px] text-gray-400 mt-1">QR refreshes automatically every 15 seconds</p>
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
