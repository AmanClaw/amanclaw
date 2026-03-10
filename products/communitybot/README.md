# CommunityBot

CommunityBot is a pre-configured, ready-to-deploy AI assistant built on [AmanClaw](https://github.com/AmanClaw/amanclaw). It works in Telegram, Discord, WhatsApp, and Slack group chats — answering questions, welcoming new members, and helping coordinate events. It supports Malay and English, includes built-in Islamic community skills, and runs on any OpenAI-compatible LLM (including free local models via Ollama).

## Quick Deploy

### Docker (self-hosted)

```bash
# 1. Clone the repository
git clone https://github.com/AmanClaw/amanclaw.git
cd amanclaw/products/communitybot

# 2. Create your environment file
cp .env.example .env
# Open .env in a text editor and add your bot token

# 3. Start the bot
docker compose up -d
```

### Fly.io

```bash
# 1. Clone the repository
git clone https://github.com/AmanClaw/amanclaw.git
cd amanclaw/products/communitybot

# 2. Launch on Fly.io (follow the prompts)
fly launch

# 3. Set your bot token as a secret
fly secrets set TELEGRAM_BOT_TOKEN=your-telegram-bot-token
```

### Railway

1. Fork the [AmanClaw repository](https://github.com/AmanClaw/amanclaw) on GitHub.
2. Go to [Railway](https://railway.com/), create a new project, and connect your forked repo. Set the root directory to `products/communitybot`.
3. In the Railway dashboard, go to **Variables** and add your `TELEGRAM_BOT_TOKEN` (or whichever channel you use).

### Render

1. Fork the [AmanClaw repository](https://github.com/AmanClaw/amanclaw) on GitHub.
2. Go to [Render](https://render.com/), click **New Blueprint Instance**, and connect your forked repo. Render will detect the `render.yaml` file automatically.
3. In the Render dashboard, set your `TELEGRAM_BOT_TOKEN` environment variable.

## Configuration

### Changing the LLM provider

By default, CommunityBot uses Ollama running locally (free, no API key needed). To use a cloud LLM instead, edit your `.env` file:

```bash
LLM_BASE_URL=https://api.openai.com/v1
LLM_MODEL=gpt-4o-mini
LLM_API_KEY=sk-your-api-key
```

Any OpenAI-compatible API works — OpenAI, Anthropic (via proxy), Groq, Together AI, etc.

### Adding chat channels

Uncomment and fill in the relevant variables in your `.env` file. You can run multiple channels at once:

```bash
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
DISCORD_BOT_TOKEN=your-discord-bot-token
WHATSAPP_ACCESS_TOKEN=your-whatsapp-token
WHATSAPP_PHONE_NUMBER_ID=your-phone-number-id
```

## Customization

The bot's personality is defined in `souls/community.md`. Open it in any text editor to change how the bot behaves:

- **Tone** — Make it more formal or more casual.
- **Language** — Add or remove language preferences.
- **Scope** — Tell it to focus on specific topics or avoid others.
- **Behavior** — Adjust when and how it responds in group chats vs DMs.

Changes take effect after restarting the bot (`docker compose restart`).

## Troubleshooting

**Bot is not responding to messages**
- Check that your bot token is correct in the `.env` file.
- For Telegram: make sure you've started a conversation with the bot or added it to a group.
- Check the logs: `docker compose logs -f`

**LLM connection errors**
- If using Ollama locally, make sure Ollama is running (`ollama serve`) and you've pulled the model (`ollama pull llama3`).
- If using a cloud LLM, verify your API key is valid and has credits.
- Check that `LLM_BASE_URL` is reachable from your server.

**Bot replies are slow**
- Local LLMs on CPU can be slow. Consider using a smaller model or switching to a cloud LLM.
- Check your server's available memory — LLMs need at least 4 GB of RAM for small models.

**Docker build fails**
- Make sure Docker and Docker Compose are installed: `docker --version` and `docker compose version`.
- Try pulling the latest base image: `docker pull ghcr.io/amanclaw/amanclaw:latest`.

**Rate limiting**
- The default config allows 30 messages per minute and 300 per hour. Edit `config.yaml` to adjust these limits.
