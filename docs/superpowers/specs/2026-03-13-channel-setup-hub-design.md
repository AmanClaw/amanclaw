# Channel Setup Hub — Design Spec

## Problem

Channel configuration is entirely environment-variable-based. Users must edit .env files and restart the engine to add/change channels. There's no QR code UI for WhatsApp Web (WAHA) setup — users must access WAHA's own interface separately. The dashboard shows read-only channel status cards with no configuration ability.

## Solution

A Channel Setup Hub in both dashboard and desktop that lets users configure, start, stop, and monitor all channels from the UI. WhatsApp Web gets a QR code display for seamless phone scanning. Config is persisted to config.yaml with env var fallback for backwards compatibility.

## Architecture

```
Dashboard/Desktop UI  →  Management API  →  Engine (ChannelManager)
     (Svelte)              (Axum REST)        (Rust runtime)
```

- Shared Svelte components between dashboard and desktop
- API proxies WAHA endpoints — UI never talks to WAHA directly
- Config changes write to config.yaml and trigger engine hot-reload
- Environment variables still work as fallback (existing deployments unaffected)

## API Endpoints

All under `/api/channels/`, authenticated via JWT.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/channels` | List all channels with status |
| GET | `/api/channels/:id` | Get channel config + status |
| PUT | `/api/channels/:id` | Update channel config (persists to config.yaml) |
| POST | `/api/channels/:id/start` | Start a channel |
| POST | `/api/channels/:id/stop` | Stop a channel |
| POST | `/api/channels/:id/test` | Send test message |
| GET | `/api/channels/whatsapp-web/qr` | Proxy WAHA QR code (returns base64 PNG) |
| GET | `/api/channels/whatsapp-web/session` | Proxy WAHA session status |

### QR Code Flow

1. User enters WAHA URL + API key in UI
2. UI calls `PUT /api/channels/whatsapp-web` to save config
3. UI polls `GET /api/channels/whatsapp-web/qr` every 5 seconds
4. API proxies to `GET {WAHA_URL}/api/{session}/auth/qr` and returns the QR image
5. User scans QR on phone
6. UI polls `GET /api/channels/whatsapp-web/session` — detects "WORKING" status
7. UI shows "Connected" and stops polling
8. QR auto-refreshes every 15 seconds (WAHA QR codes expire)

## Config Persistence

New `channels:` section in config.yaml:

```yaml
channels:
  telegram:
    enabled: true
    token: "bot123:ABC..."
  whatsapp_web:
    enabled: true
    waha_url: "http://localhost:3000"
    waha_api_key: "secret"
    session: "default"
    webhook_port: 8081
  whatsapp_cloud:
    enabled: false
    access_token: ""
    phone_number_id: ""
    verify_token: "amanclaw_verify"
    webhook_port: 8080
  discord:
    enabled: false
    token: ""
  slack:
    enabled: false
    bot_token: ""
```

**Backwards compatibility:** If `channels:` section is absent in config.yaml, the engine falls back to environment variables (current behavior).

## Channel Setup Inputs

| Channel | Fields | Special UI |
|---------|--------|-----------|
| WhatsApp Web (WAHA) | waha_url, waha_api_key, session | QR code display |
| WhatsApp Cloud | access_token, phone_number_id, verify_token | Webhook URL display |
| Telegram | token | - |
| Discord | token | - |
| Slack | bot_token | - |

## Engine: ChannelManager

New component that manages channel lifecycle with hot-reload support.

### ChannelConfig enum

```rust
enum ChannelConfig {
    Telegram { token: String },
    Discord { token: String },
    Slack { bot_token: String },
    WhatsAppCloud {
        access_token: String,
        phone_number_id: String,
        verify_token: String,
        webhook_port: u16,
    },
    WhatsAppWeb {
        waha_url: String,
        waha_api_key: Option<String>,
        session: String,
        webhook_port: u16,
    },
}
```

### New EngineCommand variants

- `ChannelUpdate { id: String, config: ChannelConfig }` — start or restart a channel
- `ChannelStop { id: String }` — stop a channel
- `ChannelStatus { reply: oneshot::Sender<Vec<ChannelStatusInfo>> }` — get all statuses

### ChannelManager behavior

```
ChannelUpdate received:
  1. If channel running → stop it (drop Arc, which triggers cleanup)
  2. Create new channel instance from config
  3. Call channel.start(msg_tx.clone())
  4. Insert into HashMap<String, Arc<dyn Channel>>
  5. Update config.yaml on disk
```

### ChannelStatusInfo

```rust
struct ChannelStatusInfo {
    id: String,          // "telegram", "whatsapp-web", etc.
    platform: String,    // Channel::platform() value
    running: bool,
    configured: bool,
    error: Option<String>,
}
```

## UI Design

### Shared component: ChannelSetup.svelte

Used in both dashboard (`/dashboard/src/lib/pages/Channels.svelte`) and desktop.

### Channel card states

| State | Visual | Actions |
|-------|--------|---------|
| Not configured | Gray card, "Setup" button | Opens config form |
| Configured, stopped | Yellow card, config summary | Start, Edit, Remove |
| Connecting | Blue card, spinner | Cancel |
| Connected (online) | Green card, uptime + msg count | Stop, Edit, Test |
| Error | Red card, error message | Retry, Edit |

### WhatsApp Web card — QR flow

1. User clicks "Setup" → form: WAHA URL + API key fields
2. Click "Connect" → saves config, card expands to show QR code
3. QR code displayed large and centered, scannable from phone
4. Caption: "Scan this QR code with WhatsApp on your phone"
5. QR auto-refreshes every 15 seconds
6. On successful scan → card transitions to green "Connected"
7. If expired → "Refresh QR" button

### Other channel cards — token flow

1. Click "Setup" → form with token field (password-masked, show/hide toggle)
2. Click "Save & Connect" → saves config, starts channel
3. On success → green "Connected"

### Desktop-specific

System notification when WhatsApp QR needs scanning (if user navigates away from channel page).

## New Files

| File | Purpose |
|------|---------|
| `rust/crates/amanclaw-core/src/channel_manager.rs` | ChannelManager with hot-reload |
| `rust/crates/amanclaw-traits/src/channel_config.rs` | ChannelConfig enum + ChannelStatusInfo |
| `rust/crates/amanclaw-api/src/channels.rs` | API endpoints for channel CRUD + QR proxy |
| `dashboard/src/lib/components/ChannelCard.svelte` | Reusable channel card component |
| `dashboard/src/lib/components/QrCodeDisplay.svelte` | QR code polling + display component |
| `dashboard/src/lib/pages/Channels.svelte` | Replace existing read-only page |
| `desktop/src/lib/pages/Channels.svelte` | Desktop channel page (reuses components) |

## Success Criteria

1. User can configure any channel from dashboard or desktop without editing files
2. WhatsApp Web QR code displays in-app and successfully pairs
3. Config persists to config.yaml, survives engine restart
4. Existing env-var-based deployments continue working (backwards compatible)
5. Channel start/stop works without full engine restart
6. Status updates reflect in UI within 5 seconds
