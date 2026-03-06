/**
 * AmanClaw WhatsApp Bridge
 *
 * Connects to WhatsApp via Baileys (WhatsApp Web multi-device protocol).
 * Exposes a REST API for the Python bot to send messages.
 * Forwards incoming messages to the Python bot via HTTP callback.
 *
 * Env vars:
 *   BRIDGE_PORT          — HTTP port for REST API (default: 3001)
 *   PYTHON_CALLBACK_URL  — where to POST incoming messages (default: http://localhost:3002/whatsapp/incoming)
 *   WA_AUTH_DIR          — directory to persist auth state (default: ./auth_state)
 */

import {
  default as makeWASocket,
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
  makeCacheableSignalKeyStore,
} from "@whiskeysockets/baileys";
import express from "express";
import pino from "pino";
import qrcode from "qrcode-terminal";
import { Boom } from "@hapi/boom";
import { mkdir } from "fs/promises";

const PORT = parseInt(process.env.BRIDGE_PORT || "3001");
const CALLBACK_URL =
  process.env.PYTHON_CALLBACK_URL ||
  "http://localhost:3002/whatsapp/incoming";
const AUTH_DIR = process.env.WA_AUTH_DIR || "./auth_state";

const logger = pino({ level: "warn" });

let sock = null;
let connectionState = "disconnected";

// --- Baileys connection ---

async function connectWhatsApp() {
  await mkdir(AUTH_DIR, { recursive: true });
  const { state, saveCreds } = await useMultiFileAuthState(AUTH_DIR);
  const { version } = await fetchLatestBaileysVersion();

  sock = makeWASocket({
    version,
    auth: {
      creds: state.creds,
      keys: makeCacheableSignalKeyStore(state.keys, logger),
    },
    logger,
    printQRInTerminal: false,
    generateHighQualityLinkPreview: false,
    syncFullHistory: false,
  });

  // QR code for first-time pairing
  sock.ev.on("connection.update", (update) => {
    const { connection, lastDisconnect, qr } = update;

    if (qr) {
      console.log("\n=== Scan this QR code with WhatsApp ===\n");
      qrcode.generate(qr, { small: true });
      console.log("");
    }

    if (connection === "open") {
      connectionState = "connected";
      const user = sock.user;
      console.log(
        `WhatsApp connected: ${user?.name || user?.id || "unknown"}`
      );
    }

    if (connection === "close") {
      connectionState = "disconnected";
      const reason = new Boom(lastDisconnect?.error)?.output?.statusCode;

      if (reason === DisconnectReason.loggedOut) {
        console.error(
          "WhatsApp logged out. Delete auth_state/ and scan QR again."
        );
        process.exit(1);
      }

      // Auto-reconnect for transient errors
      console.log(`WhatsApp disconnected (reason: ${reason}), reconnecting...`);
      setTimeout(connectWhatsApp, 3000);
    }
  });

  // Persist auth credentials
  sock.ev.on("creds.update", saveCreds);

  // Handle incoming messages
  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    if (type !== "notify") return;

    for (const msg of messages) {
      // Skip our own messages, status broadcasts, and protocol messages
      if (msg.key.fromMe) continue;
      if (msg.key.remoteJid === "status@broadcast") continue;
      if (!msg.message) continue;

      const jid = msg.key.remoteJid;
      const isGroup = jid.endsWith("@g.us");

      // Extract text from various message types
      const text =
        msg.message.conversation ||
        msg.message.extendedTextMessage?.text ||
        msg.message.imageMessage?.caption ||
        null;

      if (!text) continue; // Skip non-text for now

      // Extract phone number from JID (remove @s.whatsapp.net)
      const phoneNumber = jid.replace("@s.whatsapp.net", "").replace("@g.us", "");

      // Get sender info
      const pushName = msg.pushName || "";

      const payload = {
        from: phoneNumber,
        jid: jid,
        name: pushName,
        text: text,
        is_group: isGroup,
        message_id: msg.key.id,
        timestamp: msg.messageTimestamp
          ? parseInt(msg.messageTimestamp.toString())
          : Math.floor(Date.now() / 1000),
      };

      // Forward to Python bot
      try {
        await fetch(CALLBACK_URL, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
          signal: AbortSignal.timeout(30000),
        });
      } catch (err) {
        console.error(`Failed to forward message to Python: ${err.message}`);
      }
    }
  });
}

// --- REST API for Python ---

const app = express();
app.use(express.json());

// Health check
app.get("/health", (_req, res) => {
  res.json({
    status: connectionState,
    user: sock?.user
      ? { id: sock.user.id, name: sock.user.name }
      : null,
  });
});

// Send a text message
app.post("/send", async (req, res) => {
  const { jid, text } = req.body;

  if (!jid || !text) {
    return res.status(400).json({ error: "Missing jid or text" });
  }

  if (connectionState !== "connected" || !sock) {
    return res.status(503).json({ error: "WhatsApp not connected" });
  }

  try {
    // Send with read receipts and typing indicator
    await sock.presenceSubscribe(jid);
    await sock.sendPresenceUpdate("composing", jid);

    // Brief delay for natural feel
    await new Promise((r) => setTimeout(r, 500));

    await sock.sendMessage(jid, { text });
    await sock.sendPresenceUpdate("paused", jid);

    res.json({ ok: true });
  } catch (err) {
    console.error(`Send failed: ${err.message}`);
    res.status(500).json({ error: err.message });
  }
});

// Send message to a phone number (convenience — auto-adds @s.whatsapp.net)
app.post("/send-to", async (req, res) => {
  const { phone, text } = req.body;

  if (!phone || !text) {
    return res.status(400).json({ error: "Missing phone or text" });
  }

  // Normalize: strip +, spaces, dashes
  const clean = phone.replace(/[\s\-+]/g, "");
  const jid = clean.includes("@") ? clean : `${clean}@s.whatsapp.net`;

  req.body.jid = jid;
  req.body.text = text;

  // Reuse /send handler
  if (connectionState !== "connected" || !sock) {
    return res.status(503).json({ error: "WhatsApp not connected" });
  }

  try {
    await sock.presenceSubscribe(jid);
    await sock.sendPresenceUpdate("composing", jid);
    await new Promise((r) => setTimeout(r, 500));
    await sock.sendMessage(jid, { text });
    await sock.sendPresenceUpdate("paused", jid);
    res.json({ ok: true });
  } catch (err) {
    console.error(`Send failed: ${err.message}`);
    res.status(500).json({ error: err.message });
  }
});

// Get connection info
app.get("/me", (_req, res) => {
  if (!sock?.user) {
    return res.status(503).json({ error: "Not connected" });
  }
  res.json({
    id: sock.user.id,
    name: sock.user.name,
    phone: sock.user.id.split(":")[0],
  });
});

// --- Start ---

app.listen(PORT, () => {
  console.log(`WhatsApp bridge API listening on port ${PORT}`);
  console.log(`Forwarding messages to ${CALLBACK_URL}`);
  connectWhatsApp().catch((err) => {
    console.error(`Failed to connect: ${err}`);
    process.exit(1);
  });
});
