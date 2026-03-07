/**
 * AmanClaw WhatsApp Bridge
 *
 * Connects to WhatsApp via Baileys (WhatsApp Web multi-device protocol).
 * Exposes a REST API for the Python bot to send messages.
 * Forwards incoming messages to the Python bot via HTTP callback.
 */

import {
  default as makeWASocket,
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
  makeCacheableSignalKeyStore,
  downloadMediaMessage,
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
      // Log bot identity for debugging mention matching
      console.log(`Bot JID: ${user?.id}`);
      if (user?.lid) console.log(`Bot LID: ${user.lid}`);
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
      // In groups, allow fromMe so the bot owner can interact; in DMs skip to avoid echo
      const isGroupMsg = msg.key.remoteJid?.endsWith("@g.us");
      if (msg.key.fromMe && !isGroupMsg) continue;
      if (msg.key.remoteJid === "status@broadcast") continue;
      if (!msg.message) continue;

      const jid = msg.key.remoteJid;
      const isGroup = jid.endsWith("@g.us");

      // Extract text from various message types
      const extMsg = msg.message.extendedTextMessage;
      const imageMsg = msg.message.imageMessage;
      const docMsg = msg.message.documentMessage || msg.message.documentWithCaptionMessage?.message?.documentMessage;
      const audioMsg = msg.message.audioMessage;
      const videoMsg = msg.message.videoMessage;
      const stickerMsg = msg.message.stickerMessage;

      const text =
        msg.message.conversation ||
        extMsg?.text ||
        imageMsg?.caption ||
        docMsg?.caption ||
        videoMsg?.caption ||
        null;

      // Determine if this message has media
      const mediaMsg = imageMsg || docMsg || audioMsg || videoMsg || stickerMsg;

      // Skip if no text AND no media
      if (!text && !mediaMsg) continue;

      // Get sender info
      const pushName = msg.pushName || "";

      // In groups, participant is the sender; in DMs, it's the remoteJid
      const participant = msg.key.participant || jid;
      const phoneNumber = participant
        .replace("@s.whatsapp.net", "")
        .replace("@lid", "")
        .replace("@g.us", "");

      // Extract mentioned JIDs (from @mentions in the message)
      const mentionedJids = extMsg?.contextInfo?.mentionedJid || imageMsg?.contextInfo?.mentionedJid || [];

      // Build bot identity info for mention matching
      const botUser = sock.user || {};
      const botJid = botUser.id || "";
      const botLid = botUser.lid || "";
      const botNumber = botJid.split(":")[0].split("@")[0];
      const botLidNumber = botLid ? botLid.split(":")[0].split("@")[0] : "";

      // Check if bot is mentioned (match against both phone JID and LID)
      let botMentioned = false;
      for (const m of mentionedJids) {
        const mentionNum = m.split(":")[0].split("@")[0];
        if (
          (botNumber && mentionNum === botNumber) ||
          (botLidNumber && mentionNum === botLidNumber)
        ) {
          botMentioned = true;
          break;
        }
      }

      // Download media if present
      let mediaBase64 = null;
      let mediaType = null;
      let mediaFilename = null;
      let mediaMimetype = null;

      if (mediaMsg) {
        try {
          const buffer = await downloadMediaMessage(msg, "buffer", {}, {
            logger,
            reuploadRequest: sock.updateMediaMessage,
          });
          mediaBase64 = buffer.toString("base64");

          if (imageMsg) {
            mediaType = "image";
            mediaMimetype = imageMsg.mimetype || "image/jpeg";
          } else if (docMsg) {
            mediaType = "document";
            mediaFilename = docMsg.fileName || "document";
            mediaMimetype = docMsg.mimetype || "application/octet-stream";
          } else if (audioMsg) {
            mediaType = "audio";
            mediaMimetype = audioMsg.mimetype || "audio/ogg";
          } else if (videoMsg) {
            mediaType = "video";
            mediaMimetype = videoMsg.mimetype || "video/mp4";
          } else if (stickerMsg) {
            mediaType = "sticker";
            mediaMimetype = stickerMsg.mimetype || "image/webp";
          }

          console.log(`Downloaded ${mediaType}: ${mediaFilename || mediaMimetype} (${buffer.length} bytes)`);
        } catch (err) {
          console.error(`Failed to download media: ${err.message}`);
        }
      }

      const payload = {
        from: phoneNumber,
        jid: jid,
        name: pushName,
        text: text || "",
        is_group: isGroup,
        message_id: msg.key.id,
        bot_mentioned: botMentioned,
        mentioned_jids: mentionedJids,
        bot_jid: botJid,
        bot_lid: botLid,
        timestamp: msg.messageTimestamp
          ? parseInt(msg.messageTimestamp.toString())
          : Math.floor(Date.now() / 1000),
      };

      // Add media fields if present
      if (mediaBase64) {
        payload.media = {
          type: mediaType,
          data: mediaBase64,
          mimetype: mediaMimetype,
          filename: mediaFilename,
        };
      }

      // Forward to Python bot
      try {
        await fetch(CALLBACK_URL, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
          signal: AbortSignal.timeout(60000),
        });
      } catch (err) {
        console.error(`Failed to forward message to Python: ${err.message}`);
      }
    }
  });
}

// --- REST API for Python ---

const app = express();
app.use(express.json({ limit: "50mb" }));

// Health check
app.get("/health", (_req, res) => {
  res.json({
    status: connectionState,
    user: sock?.user
      ? { id: sock.user.id, name: sock.user.name, lid: sock.user.lid }
      : null,
  });
});

// Send a text message (supports quote_id for replying to a specific message)
app.post("/send", async (req, res) => {
  const { jid, text, quote_id } = req.body;

  if (!jid || !text) {
    return res.status(400).json({ error: "Missing jid or text" });
  }

  if (connectionState !== "connected" || !sock) {
    return res.status(503).json({ error: "WhatsApp not connected" });
  }

  try {
    await sock.presenceSubscribe(jid);
    await sock.sendPresenceUpdate("composing", jid);
    await new Promise((r) => setTimeout(r, 500));

    const msgOptions = { text };

    if (quote_id) {
      msgOptions.quoted = {
        key: {
          remoteJid: jid,
          id: quote_id,
        },
        message: {},
      };
    }

    await sock.sendMessage(jid, msgOptions);
    await sock.sendPresenceUpdate("paused", jid);

    res.json({ ok: true });
  } catch (err) {
    console.error(`Send failed: ${err.message}`);
    res.status(500).json({ error: err.message });
  }
});

// Send message to a phone number (convenience)
app.post("/send-to", async (req, res) => {
  const { phone, text } = req.body;

  if (!phone || !text) {
    return res.status(400).json({ error: "Missing phone or text" });
  }

  const clean = phone.replace(/[\s\-+]/g, "");
  const jid = clean.includes("@") ? clean : `${clean}@s.whatsapp.net`;

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
    lid: sock.user.lid,
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
