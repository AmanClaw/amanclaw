<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch } from '../stores/api'
  import ChannelCard from '../components/ChannelCard.svelte'
  import QrCodeDisplay from '../components/QrCodeDisplay.svelte'

  interface ChannelStatus {
    id: string
    platform: string
    configured: boolean
    enabled: boolean
    running: boolean
    error: string | null
  }

  let channels = $state<ChannelStatus[]>([])
  let loading = $state(true)
  let showQr = $state(false)
  let refreshTimer: ReturnType<typeof setInterval> | null = null

  let wahaUrl = $state('')
  let wahaApiKey = $state('')
  let wahaSession = $state('default')
  let wahaPort = $state('8081')

  const channelMeta: Record<string, { label: string; description: string; fields: any[] }> = {
    'telegram': {
      label: 'Telegram',
      description: 'Bot messaging via Telegram',
      fields: [
        { key: 'token', label: 'Bot Token', type: 'password', placeholder: '123456:ABC-DEF...', required: true },
      ],
    },
    'discord': {
      label: 'Discord',
      description: 'Discord bot integration',
      fields: [
        { key: 'token', label: 'Bot Token', type: 'password', placeholder: 'MTIz...', required: true },
      ],
    },
    'slack': {
      label: 'Slack',
      description: 'Slack workspace integration',
      fields: [
        { key: 'bot_token', label: 'Bot Token', type: 'password', placeholder: 'xoxb-...', required: true },
        { key: 'app_token', label: 'App Token (optional)', type: 'password', placeholder: 'xapp-...' },
      ],
    },
    'whatsapp-cloud': {
      label: 'WhatsApp Cloud',
      description: 'Official WhatsApp Business API',
      fields: [
        { key: 'access_token', label: 'Access Token', type: 'password', placeholder: 'EAAx...', required: true },
        { key: 'phone_number_id', label: 'Phone Number ID', type: 'text', placeholder: '1234567890', required: true },
        { key: 'verify_token', label: 'Verify Token', type: 'text', placeholder: 'amanclaw_verify' },
      ],
    },
    'whatsapp-web': {
      label: 'WhatsApp Web',
      description: 'Via WAHA bridge — scan QR code to connect',
      fields: [
        { key: 'waha_url', label: 'WAHA URL', type: 'text', placeholder: 'http://localhost:3000', required: true },
        { key: 'waha_api_key', label: 'API Key (optional)', type: 'password', placeholder: 'your-api-key' },
        { key: 'session', label: 'Session Name', type: 'text', placeholder: 'default' },
        { key: 'webhook_port', label: 'Webhook Port', type: 'text', placeholder: '8081' },
      ],
    },
  }

  async function loadChannels() {
    try {
      channels = await apiFetch('/channels')
    } catch (e) {
      console.error('Failed to load channels:', e)
    } finally {
      loading = false
    }
  }

  onMount(() => {
    loadChannels()
    refreshTimer = setInterval(loadChannels, 5000)
  })

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer)
  })

  function getChannel(id: string): ChannelStatus {
    return channels.find(c => c.id === id) || {
      id, platform: id, configured: false, enabled: false, running: false, error: null
    }
  }

  async function saveWhatsAppWeb(values: Record<string, string>) {
    await apiFetch('/channels/whatsapp-web/config', {
      method: 'PUT',
      body: JSON.stringify({
        enabled: true,
        waha_url: values.waha_url || 'http://localhost:3000',
        waha_api_key: values.waha_api_key || null,
        session: values.session || 'default',
        webhook_port: parseInt(values.webhook_port || '8081'),
      }),
    })
    wahaUrl = values.waha_url || ''
    wahaApiKey = values.waha_api_key || ''
    wahaSession = values.session || 'default'
    wahaPort = values.webhook_port || '8081'
    showQr = true
    await loadChannels()
  }

  async function startChannel(id: string) {
    await apiFetch(`/channels/${id}/start`, { method: 'POST' })
    if (id === 'whatsapp-web') showQr = true
    await loadChannels()
  }

  async function stopChannel(id: string) {
    await apiFetch(`/channels/${id}/stop`, { method: 'POST' })
    showQr = false
    await loadChannels()
  }

  async function fetchQr() {
    return apiFetch('/channels/whatsapp-web/qr')
  }

  async function fetchSession() {
    return apiFetch('/channels/whatsapp-web/session')
  }

  function onWhatsAppConnected() {
    showQr = false
    loadChannels()
  }

  const channelOrder = ['telegram', 'whatsapp-web', 'whatsapp-cloud', 'discord', 'slack']
</script>

<div class="p-6 md:p-8">
  <div class="mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Channels</h2>
    <p class="text-sm text-gray-500 mt-1">Configure, start, and monitor your messaging channels</p>
  </div>

  {#if loading}
    <p class="text-gray-500">Loading channels...</p>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {#each channelOrder as id}
        {@const meta = channelMeta[id]}
        {@const ch = getChannel(id)}
        {#if meta}
          <div class={id === 'whatsapp-web' && showQr ? 'sm:col-span-2' : ''}>
            <ChannelCard
              channel={ch}
              label={meta.label}
              description={meta.description}
              fields={meta.fields}
              configValues={id === 'whatsapp-web' ? { waha_url: wahaUrl, waha_api_key: wahaApiKey, session: wahaSession, webhook_port: wahaPort } : {}}
              onSave={id === 'whatsapp-web' ? saveWhatsAppWeb : async (values) => {
                console.log('Save not yet implemented for', id, values)
              }}
              onStart={() => startChannel(id)}
              onStop={() => stopChannel(id)}
            />
            {#if id === 'whatsapp-web' && showQr && ch.configured}
              <QrCodeDisplay
                {fetchQr}
                {fetchSession}
                onConnected={onWhatsAppConnected}
              />
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
