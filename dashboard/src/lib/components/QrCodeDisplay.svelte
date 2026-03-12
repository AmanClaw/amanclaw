<script lang="ts">
  import { onMount, onDestroy } from 'svelte'

  interface Props {
    fetchQr: () => Promise<any>
    fetchSession: () => Promise<any>
    onConnected: () => void
  }

  let { fetchQr, fetchSession, onConnected }: Props = $props()

  let qrData = $state<string | null>(null)
  let status = $state<'loading' | 'scanning' | 'connected' | 'error'>('loading')
  let errorMsg = $state('')
  let pollTimer: ReturnType<typeof setInterval> | null = null
  let refreshTimer: ReturnType<typeof setInterval> | null = null

  async function loadQr() {
    try {
      const result = await fetchQr()
      if (result.error) {
        status = 'error'
        errorMsg = result.error
        return
      }
      if (result.mimetype && result.data) {
        qrData = `data:${result.mimetype};base64,${result.data}`
      } else if (result.value) {
        qrData = result.value
      } else {
        qrData = null
      }
      status = 'scanning'
    } catch (e: any) {
      status = 'error'
      errorMsg = e?.message || 'Failed to load QR code'
    }
  }

  async function checkSession() {
    try {
      const result = await fetchSession()
      const sessionStatus = result?.status || result?.engine?.state
      if (sessionStatus === 'WORKING' || sessionStatus === 'CONNECTED') {
        status = 'connected'
        stopPolling()
        onConnected()
      }
    } catch (_) {}
  }

  function startPolling() {
    loadQr()
    refreshTimer = setInterval(loadQr, 15000)
    pollTimer = setInterval(checkSession, 5000)
  }

  function stopPolling() {
    if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
    if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null }
  }

  onMount(() => {
    startPolling()
  })

  onDestroy(() => {
    stopPolling()
  })
</script>

<div class="mt-4">
  {#if status === 'loading'}
    <div class="flex items-center justify-center p-8">
      <div class="animate-spin w-6 h-6 border-2 border-blue-600 border-t-transparent rounded-full"></div>
      <span class="ml-3 text-sm text-gray-500">Loading QR code...</span>
    </div>
  {:else if status === 'connected'}
    <div class="flex items-center justify-center p-6 bg-green-50 dark:bg-green-900/20 rounded-lg">
      <span class="text-green-700 dark:text-green-400 font-medium text-sm">WhatsApp Connected!</span>
    </div>
  {:else if status === 'error'}
    <div class="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg">
      <p class="text-sm text-red-700 dark:text-red-400 mb-2">{errorMsg}</p>
      <button
        onclick={() => { status = 'loading'; startPolling() }}
        class="px-3 py-1.5 text-xs font-medium rounded-md bg-red-600 text-white hover:bg-red-700 transition-colors"
      >
        Retry
      </button>
    </div>
  {:else if status === 'scanning'}
    <div class="flex flex-col items-center p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
      {#if qrData && qrData.startsWith('data:')}
        <img src={qrData} alt="WhatsApp QR Code" class="w-64 h-64 rounded-lg" />
      {:else if qrData}
        <div class="bg-white p-4 rounded-lg">
          <p class="text-xs text-gray-500 font-mono break-all">{qrData}</p>
        </div>
      {:else}
        <p class="text-sm text-gray-500">Waiting for QR code...</p>
      {/if}
      <p class="text-xs text-gray-500 mt-3">Scan this QR code with WhatsApp on your phone</p>
      <p class="text-[10px] text-gray-400 mt-1">QR refreshes automatically every 15 seconds</p>
    </div>
  {/if}
</div>
