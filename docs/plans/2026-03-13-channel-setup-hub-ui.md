# Channel Setup Hub UI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the read-only Channels page in both dashboard and desktop with an interactive Channel Setup Hub — configure, start, stop, and monitor all channels from the UI, with QR code display for WhatsApp Web (WAHA).

**Architecture:** Reusable ChannelCard and QrCodeDisplay Svelte components shared between dashboard and desktop. Dashboard uses `apiFetch()` (HTTP REST), desktop uses `api.*` (Tauri IPC) — both calling the same backend API endpoints built in Part 1. Desktop adds Tauri commands that proxy to the management API or engine directly.

**Tech Stack:** Svelte 5, TypeScript, TailwindCSS 4, Tauri 2 (desktop), Axum REST API (backend)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `dashboard/src/lib/components/ChannelCard.svelte` | Reusable channel card with status, config form, actions |
| `dashboard/src/lib/components/QrCodeDisplay.svelte` | QR code polling + display for WhatsApp Web |
| `dashboard/src/lib/pages/Channels.svelte` | Replace existing read-only page with interactive hub |
| `desktop/src/lib/pages/Channels.svelte` | New desktop Channels page (uses Tauri IPC) |
| `desktop/src/lib/components/Sidebar.svelte` | Add "Channels" to navigation |
| `desktop/src/routes/+page.svelte` | Add Channels page routing |
| `desktop/src/lib/api.ts` | Add channel API methods |
| `desktop/src-tauri/src/commands.rs` | Add Tauri commands for channel management |

---

## Chunk 1: Dashboard Components

### Task 1: Create ChannelCard component for dashboard

**Files:**
- Create: `dashboard/src/lib/components/ChannelCard.svelte`

This component renders a single channel card with multiple states (not configured, configured/stopped, connecting, connected, error). It shows a config form inline and action buttons.

- [ ] **Step 1: Create ChannelCard.svelte**

```svelte
<script lang="ts">
  import StatusBadge from './StatusBadge.svelte'

  interface ChannelStatus {
    id: string
    platform: string
    configured: boolean
    enabled: boolean
    running: boolean
    error: string | null
  }

  interface Props {
    channel: ChannelStatus
    label: string
    description: string
    fields: { key: string; label: string; type: string; placeholder: string; required?: boolean }[]
    configValues: Record<string, string>
    onSave: (values: Record<string, string>) => Promise<void>
    onStart: () => Promise<void>
    onStop: () => Promise<void>
    showQr?: boolean
  }

  let {
    channel,
    label,
    description,
    fields,
    configValues = {},
    onSave,
    onStart,
    onStop,
    showQr = false,
  }: Props = $props()

  let editing = $state(false)
  let saving = $state(false)
  let starting = $state(false)
  let formValues = $state<Record<string, string>>({})
  let showPassword = $state<Record<string, boolean>>({})

  function openForm() {
    formValues = { ...configValues }
    editing = true
  }

  function cancelForm() {
    editing = false
  }

  async function handleSave() {
    saving = true
    try {
      await onSave(formValues)
      editing = false
    } catch (e) {
      console.error('Save failed:', e)
    } finally {
      saving = false
    }
  }

  async function handleStart() {
    starting = true
    try {
      await onStart()
    } catch (e) {
      console.error('Start failed:', e)
    } finally {
      starting = false
    }
  }

  async function handleStop() {
    try {
      await onStop()
    } catch (e) {
      console.error('Stop failed:', e)
    }
  }

  const statusBadge = $derived(
    channel.running ? 'online' :
    channel.error ? 'offline' :
    channel.configured ? 'warning' :
    'offline'
  )

  const statusLabel = $derived(
    channel.running ? 'Connected' :
    channel.error ? 'Error' :
    channel.configured ? 'Configured' :
    'Not configured'
  )

  const borderColor = $derived(
    channel.running ? 'border-green-300 dark:border-green-700' :
    channel.error ? 'border-red-300 dark:border-red-700' :
    channel.configured ? 'border-yellow-300 dark:border-yellow-700' :
    'border-gray-200 dark:border-gray-700'
  )
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border {borderColor}">
  <div class="flex items-center justify-between mb-2">
    <h3 class="font-semibold text-gray-900 dark:text-white">{label}</h3>
    <StatusBadge status={statusBadge} label={statusLabel} />
  </div>
  <p class="text-sm text-gray-500 mb-4">{description}</p>

  {#if channel.error}
    <div class="mb-3 p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs text-red-700 dark:text-red-400">
      {channel.error}
    </div>
  {/if}

  {#if editing}
    <!-- Config Form -->
    <div class="space-y-3 mb-4">
      {#each fields as field}
        <div>
          <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{field.label}</label>
          <div class="relative">
            <input
              type={field.type === 'password' && !showPassword[field.key] ? 'password' : 'text'}
              bind:value={formValues[field.key]}
              placeholder={field.placeholder}
              class="w-full px-3 py-2 text-sm border border-gray-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            {#if field.type === 'password'}
              <button
                type="button"
                onclick={() => showPassword[field.key] = !showPassword[field.key]}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-gray-400 hover:text-gray-600"
              >
                {showPassword[field.key] ? 'Hide' : 'Show'}
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
    <div class="flex gap-2">
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 transition-colors"
      >
        {saving ? 'Saving...' : 'Save & Connect'}
      </button>
      <button
        onclick={cancelForm}
        class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
      >
        Cancel
      </button>
    </div>
  {:else}
    <!-- Action buttons -->
    <div class="flex gap-2">
      {#if !channel.configured}
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700 transition-colors"
        >
          Setup
        </button>
      {:else if channel.running}
        <button
          onclick={handleStop}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-300 text-red-700 hover:bg-red-50 transition-colors"
        >
          Stop
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 transition-colors"
        >
          Edit
        </button>
      {:else}
        <button
          onclick={handleStart}
          disabled={starting}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
        >
          {starting ? 'Starting...' : 'Start'}
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 transition-colors"
        >
          Edit
        </button>
      {/if}
    </div>
  {/if}
</div>
```

- [ ] **Step 2: Verify no syntax errors**

Run: `cd dashboard && npx svelte-check --tsconfig ./tsconfig.app.json 2>&1 | tail -5`

- [ ] **Step 3: Commit**

```bash
git add dashboard/src/lib/components/ChannelCard.svelte
git commit -m "feat(dashboard): add ChannelCard component with config form and actions"
```

---

### Task 2: Create QrCodeDisplay component for dashboard

**Files:**
- Create: `dashboard/src/lib/components/QrCodeDisplay.svelte`

This component polls the WAHA QR endpoint every 5 seconds and displays the QR image for phone scanning. It auto-stops when the session status becomes "WORKING".

- [ ] **Step 1: Create QrCodeDisplay.svelte**

```svelte
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
      // WAHA returns QR data in various formats
      if (result.mimetype && result.data) {
        // Base64 image format
        qrData = `data:${result.mimetype};base64,${result.data}`
      } else if (result.value) {
        // Raw QR string — display as text for now
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
    } catch (_) {
      // Ignore — keep polling
    }
  }

  function startPolling() {
    loadQr()
    // Refresh QR every 15 seconds (WAHA QR codes expire)
    refreshTimer = setInterval(loadQr, 15000)
    // Check session status every 5 seconds
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
```

- [ ] **Step 2: Verify no syntax errors**

Run: `cd dashboard && npx svelte-check --tsconfig ./tsconfig.app.json 2>&1 | tail -5`

- [ ] **Step 3: Commit**

```bash
git add dashboard/src/lib/components/QrCodeDisplay.svelte
git commit -m "feat(dashboard): add QrCodeDisplay component with auto-polling"
```

---

## Chunk 2: Dashboard Channels Page

### Task 3: Replace dashboard Channels page with interactive hub

**Files:**
- Modify: `dashboard/src/lib/pages/Channels.svelte`

Replace the existing read-only page (4 hardcoded cards using `/api/status`) with the interactive Channel Setup Hub that uses the new `/api/channels` endpoints.

- [ ] **Step 1: Replace Channels.svelte**

Replace the entire file `dashboard/src/lib/pages/Channels.svelte` with:

```svelte
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

  // Config values for each channel (populated from existing config)
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
    // Poll channel status every 5 seconds
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
    if (id === 'whatsapp-web') {
      showQr = true
    }
    await loadChannels()
  }

  async function stopChannel(id: string) {
    await apiFetch(`/channels/${id}/stop`, { method: 'POST' })
    showQr = false
    await loadChannels()
  }

  // QR-specific functions
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

  // Channel display order
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
                // For non-WAHA channels, we'd save via a generic PUT endpoint
                // Currently only WhatsApp Web has a dedicated config endpoint
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
```

- [ ] **Step 2: Verify no syntax errors**

Run: `cd dashboard && npx svelte-check --tsconfig ./tsconfig.app.json 2>&1 | tail -5`

- [ ] **Step 3: Commit**

```bash
git add dashboard/src/lib/pages/Channels.svelte
git commit -m "feat(dashboard): replace read-only Channels page with interactive Channel Setup Hub"
```

---

## Chunk 3: Desktop Tauri Commands

### Task 4: Add channel management Tauri commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/main.rs` (register new commands)

Add Tauri commands that proxy to the management API for channel operations. The desktop app may be in local mode (engine runs in-process) or remote mode (connects to a server). For simplicity, these commands make HTTP calls to the management API endpoint (same as dashboard).

- [ ] **Step 1: Read commands.rs to find the pattern**

Read `desktop/src-tauri/src/commands.rs` to understand the existing command patterns, how state is accessed, and how the management API is called. Also read `desktop/src-tauri/src/state.rs` and `desktop/src-tauri/src/main.rs` to understand command registration.

- [ ] **Step 2: Add channel commands to commands.rs**

Add these Tauri commands at the end of the file (before the closing brace if there is one), following the existing patterns in the file:

```rust
// ── Channel Management ──────────────────────────────────────────

#[tauri::command]
pub async fn list_channels(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels", api_url);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_channel_status(state: tauri::State<'_, AppState>, id: String) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/{}", api_url, id);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_whatsapp_web_config(
    state: tauri::State<'_, AppState>,
    waha_url: String,
    waha_api_key: Option<String>,
    session: Option<String>,
    webhook_port: Option<u16>,
) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/whatsapp-web/config", api_url);
    let body = serde_json::json!({
        "enabled": true,
        "waha_url": waha_url,
        "waha_api_key": waha_api_key,
        "session": session.unwrap_or_else(|| "default".into()),
        "webhook_port": webhook_port.unwrap_or(8081),
    });
    let client = reqwest::Client::new();
    let resp = client.put(&url)
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_channel(state: tauri::State<'_, AppState>, id: String) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/{}/start", api_url, id);
    let client = reqwest::Client::new();
    let resp = client.post(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_channel(state: tauri::State<'_, AppState>, id: String) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/{}/stop", api_url, id);
    let client = reqwest::Client::new();
    let resp = client.post(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_whatsapp_qr(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/whatsapp-web/qr", api_url);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_whatsapp_session(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let api_url = state.api_url.read().await;
    let url = format!("{}/api/channels/whatsapp-web/session", api_url);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
```

**IMPORTANT:** Before adding these commands, first read the existing `commands.rs` and `state.rs` to check:
1. Does `AppState` have an `api_url` field? If not, check how existing commands access the management API URL. The desktop may construct the URL from a stored port, or it may use a different approach. Adapt the commands to match the existing pattern.
2. If the desktop app runs the engine in-process and doesn't use HTTP to talk to its own management API, you may need to access the engine directly via state. Look at how existing commands like `get_status` work.

- [ ] **Step 3: Register commands in main.rs**

In `desktop/src-tauri/src/main.rs`, find the `.invoke_handler(tauri::generate_handler![...])` call and add the new commands:

```rust
commands::list_channels,
commands::get_channel_status,
commands::save_whatsapp_web_config,
commands::start_channel,
commands::stop_channel,
commands::get_whatsapp_qr,
commands::get_whatsapp_session,
```

- [ ] **Step 4: Verify compilation**

Run: `cd desktop/src-tauri && cargo check 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/main.rs
git commit -m "feat(desktop): add Tauri commands for channel management"
```

---

## Chunk 4: Desktop Frontend

### Task 5: Add channel API methods and Channels page to desktop

**Files:**
- Modify: `desktop/src/lib/api.ts`
- Create: `desktop/src/lib/pages/Channels.svelte`
- Modify: `desktop/src/lib/components/Sidebar.svelte`
- Modify: `desktop/src/routes/+page.svelte`

- [ ] **Step 1: Add channel methods to api.ts**

In `desktop/src/lib/api.ts`, add these methods to the `api` object:

```typescript
	// Channel Management
	listChannels: () => invoke('list_channels'),
	getChannelStatus: (id: string) => invoke('get_channel_status', { id }),
	saveWhatsappWebConfig: (params: {
		wahaUrl: string; wahaApiKey?: string; session?: string; webhookPort?: number;
	}) => invoke('save_whatsapp_web_config', params),
	startChannel: (id: string) => invoke('start_channel', { id }),
	stopChannel: (id: string) => invoke('stop_channel', { id }),
	getWhatsappQr: () => invoke('get_whatsapp_qr'),
	getWhatsappSession: () => invoke('get_whatsapp_session'),
```

- [ ] **Step 2: Create desktop Channels.svelte**

Create `desktop/src/lib/pages/Channels.svelte`:

```svelte
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

	// QR state
	let qrData = $state<string | null>(null);
	let qrStatus = $state<'idle' | 'loading' | 'scanning' | 'connected' | 'error'>('idle');
	let qrError = $state('');
	let qrPollTimer: ReturnType<typeof setInterval> | null = null;
	let qrRefreshTimer: ReturnType<typeof setInterval> | null = null;

	// WhatsApp Web form
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
			const result = await api.listChannels() as ChannelStatus[];
			channels = result;
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

	// WhatsApp Web actions
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

	// QR polling
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
								<!-- WhatsApp Web Config Form -->
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
								<!-- Action Buttons -->
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

						<!-- QR Code Display (WhatsApp Web only) -->
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
```

- [ ] **Step 3: Add "Channels" to desktop Sidebar**

In `desktop/src/lib/components/Sidebar.svelte`, add to the `pages` array after the `communities` entry:

```typescript
		{ id: 'channels', label: 'Channels', icon: '⇌' },
```

- [ ] **Step 4: Add Channels route to +page.svelte**

In `desktop/src/routes/+page.svelte`:

1. Add import at the top with other imports:
```typescript
	import Channels from '$lib/pages/Channels.svelte';
```

2. Add routing case. Find the line `{:else if $currentPage === 'communities'}` and add BEFORE it:
```svelte
{:else if $currentPage === 'channels'}
	<Channels />
```

- [ ] **Step 5: Verify TypeScript**

Run: `cd desktop && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add desktop/src/lib/api.ts desktop/src/lib/pages/Channels.svelte desktop/src/lib/components/Sidebar.svelte desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Channels page with QR code display for WhatsApp Web"
```

---

## Summary

| Chunk | Task | What it delivers |
|-------|------|-----------------|
| 1 | Task 1 | ChannelCard component — reusable card with status, form, actions |
| 1 | Task 2 | QrCodeDisplay component — QR polling + auto-refresh for WAHA |
| 2 | Task 3 | Dashboard Channels page — replaces read-only page with interactive hub |
| 3 | Task 4 | Desktop Tauri commands — IPC bridge for channel management |
| 4 | Task 5 | Desktop Channels page — full channel management + QR display |

After all tasks: both dashboard and desktop show interactive channel cards with WhatsApp Web QR code scanning support.
