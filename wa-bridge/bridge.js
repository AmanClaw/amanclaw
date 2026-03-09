const { Client, LocalAuth } = require("whatsapp-web.js");
const express = require("express");
const qrcode = require("qrcode-terminal");

const AMANCLAW_WEBHOOK = process.env.WEBHOOK_URL || "http://127.0.0.1:8081/webhook";
const PORT = process.env.BRIDGE_PORT || 3000;

const app = express();
app.use(express.json());

let waClient = null;
let isReady = false;

const client = new Client({
  authStrategy: new LocalAuth({ dataPath: "./.wa-session" }),
  puppeteer: {
    executablePath: process.env.CHROMIUM_PATH || "/usr/bin/chromium",
    headless: true,
    args: [
      "--no-sandbox",
      "--disable-setuid-sandbox",
      "--disable-dev-shm-usage",
      "--disable-gpu",
      "--no-first-run",
      "--single-process",
      "--disable-extensions",
    ],
  },
});

client.on("qr", (qr) => {
  console.log("\n=== Scan this QR code with WhatsApp ===\n");
  qrcode.generate(qr, { small: true });
  console.log("\nWaiting for scan...\n");
});

client.on("ready", () => {
  console.log("[wa-bridge] WhatsApp client ready!");
  isReady = true;
  waClient = client;
});

client.on("authenticated", () => {
  console.log("[wa-bridge] Authenticated successfully");
});

client.on("auth_failure", (msg) => {
  console.error("[wa-bridge] Auth failure:", msg);
});

client.on("disconnected", (reason) => {
  console.warn("[wa-bridge] Disconnected:", reason);
  isReady = false;
  setTimeout(() => client.initialize(), 5000);
});

client.on("message", async (msg) => {
  const chat = await msg.getChat();
  const contact = await msg.getContact();

  const payload = {
    event: "message",
    session: "default",
    payload: {
      id: msg.id._serialized,
      from: msg.from,
      to: msg.to,
      body: msg.body || "",
      type: msg.type,
      fromMe: msg.fromMe,
      hasMedia: msg.hasMedia,
      chatId: chat.id._serialized,
      _data: {
        notifyName: contact.pushname || contact.name || null,
      },
    },
  };

  try {
    const res = await fetch(AMANCLAW_WEBHOOK, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!res.ok) console.error("[wa-bridge] Webhook error:", res.status);
  } catch (err) {
    console.error("[wa-bridge] Failed to forward message:", err.message);
  }
});

// WAHA-compatible: POST /api/sendText
app.post("/api/sendText", async (req, res) => {
  if (!isReady) return res.status(503).json({ error: "WhatsApp not connected" });

  const { chatId, text } = req.body;
  if (!chatId || !text) return res.status(400).json({ error: "chatId and text required" });

  try {
    const result = await waClient.sendMessage(chatId, text);
    res.json({ id: result.id._serialized, status: "sent" });
  } catch (err) {
    console.error("[wa-bridge] Send error:", err.message);
    res.status(500).json({ error: err.message });
  }
});

// GET /api/sessions
app.get("/api/sessions", (req, res) => {
  res.json([{ name: "default", status: isReady ? "WORKING" : "STARTING" }]);
});

// Health
app.get("/health", (req, res) => {
  res.json({ status: isReady ? "connected" : "disconnected", uptime: process.uptime() });
});

app.listen(PORT, () => {
  console.log("[wa-bridge] API server on port " + PORT);
  console.log("[wa-bridge] Webhook target: " + AMANCLAW_WEBHOOK);
});

client.initialize();
console.log("[wa-bridge] Initializing WhatsApp Web client...");
