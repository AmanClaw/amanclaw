# WhatsApp Bridge (Baileys)

High-performance WhatsApp Web bridge using [@whiskeysockets/baileys](https://github.com/WhiskeySockets/Baileys).

Connects directly to WhatsApp's multi-device protocol — no browser, no Selenium, no third-party API.

## Setup

```bash
cd bridge/whatsapp
npm install
```

## First Run — QR Pairing

```bash
npm start
```

A QR code will appear in the terminal. Scan it with:
- WhatsApp > Settings > Linked Devices > Link a Device

Auth state is saved to `auth_state/` — you only need to scan once.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BRIDGE_PORT` | `3001` | REST API port |
| `PYTHON_CALLBACK_URL` | `http://localhost:3002/whatsapp/incoming` | Where to POST incoming messages |
| `WA_AUTH_DIR` | `./auth_state` | Directory to persist WhatsApp auth |

## API

### `GET /health`
Connection status and user info.

### `POST /send`
Send a message by JID.
```json
{"jid": "60123456789@s.whatsapp.net", "text": "Hello!"}
```

### `POST /send-to`
Send a message by phone number (auto-formats JID).
```json
{"phone": "+60123456789", "text": "Hello!"}
```

### `GET /me`
Get the connected WhatsApp account info.

## With Docker

The bridge runs as a separate service alongside the Python bot. See `docker-compose.yml` in the project root.

## Notes

- Auth state in `auth_state/` is sensitive — treat it like a password
- Add `auth_state/` to `.gitignore` (already done)
- The bridge auto-reconnects on transient disconnections
- If you get "logged out", delete `auth_state/` and scan QR again
