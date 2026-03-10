# Phase 4: CommunityBot Product — Implementation Plan

---

### Task 1: CommunityBot Product Directory

Create `products/communitybot/` with all config files.

**Files to create:**

**`products/communitybot/config.yaml`** — Pre-configured for community use:
```yaml
llm:
  base_url: "${LLM_BASE_URL:-http://localhost:11434/v1}"
  model: "${LLM_MODEL:-llama3}"
  max_tokens: 1024
  temperature: 0.7

admin_users:
  telegram: []

skills:
  disabled: []

memory:
  db_path: "data/memory.db"

agents:
  - name: community
    soul: "souls/community.md"

learning:
  enabled: true

rate_limit:
  max_messages_per_minute: 30
  max_messages_per_hour: 300
```

**`products/communitybot/.env.example`:**
```bash
# === REQUIRED ===
# Pick ONE channel (at minimum):
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
# DISCORD_BOT_TOKEN=your-discord-bot-token
# WHATSAPP_ACCESS_TOKEN=your-whatsapp-token
# WHATSAPP_PHONE_NUMBER_ID=your-phone-number-id

# === LLM SETTINGS ===
# Default: Ollama running locally (free, no API key needed)
# LLM_BASE_URL=http://localhost:11434/v1
# LLM_MODEL=llama3
# LLM_API_KEY=not-needed-for-ollama

# For cloud LLMs (OpenAI, Anthropic, etc.):
# LLM_BASE_URL=https://api.openai.com/v1
# LLM_MODEL=gpt-4o-mini
# LLM_API_KEY=sk-your-api-key
```

**`products/communitybot/souls/community.md`** — Copy of existing `souls/community.md`

**`products/communitybot/Dockerfile`:**
```dockerfile
FROM ghcr.io/amanclaw/amanclaw:latest
COPY config.yaml /app/config.yaml
COPY souls/ /app/souls/
WORKDIR /app
CMD ["./amanclaw"]
```

**`products/communitybot/docker-compose.yml`:**
```yaml
services:
  communitybot:
    build: .
    env_file: .env
    volumes:
      - ./data:/app/data
    restart: unless-stopped
```

**Commit:** `feat: add CommunityBot product directory with config and Docker`

---

### Task 2: PaaS Deploy Templates

Create deploy configs for Fly.io, Railway, and Render.

**`products/communitybot/fly.toml`:**
```toml
app = "communitybot"
primary_region = "sin"

[build]
  dockerfile = "Dockerfile"

[env]
  LLM_BASE_URL = "http://localhost:11434/v1"
  LLM_MODEL = "llama3"

[http_service]
  internal_port = 8443
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[vm]]
  size = "shared-cpu-1x"
  memory = "512mb"
```

**`products/communitybot/railway.json`:**
```json
{
  "$schema": "https://railway.com/railway.schema.json",
  "build": {
    "dockerfilePath": "Dockerfile"
  },
  "deploy": {
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 10
  }
}
```

**`products/communitybot/render.yaml`:**
```yaml
services:
  - type: web
    name: communitybot
    runtime: docker
    dockerfilePath: ./Dockerfile
    plan: free
    envVars:
      - key: LLM_BASE_URL
        value: http://localhost:11434/v1
      - key: LLM_MODEL
        value: llama3
      - key: TELEGRAM_BOT_TOKEN
        sync: false
```

**Commit:** `feat: add PaaS deploy templates (Fly.io, Railway, Render)`

---

### Task 3: Landing Page

Create `products/communitybot/index.html` — self-contained HTML with dark theme.

Features:
- Hero section: CommunityBot name + tagline
- 3 feature cards: Multi-channel, AI-powered, Community-focused
- Deploy section: 4 buttons (Fly.io, Railway, Render, Docker)
- Quick setup steps (3 steps)
- Footer: "Powered by AmanClaw" with GitHub link
- Responsive, dark theme matching playground aesthetic
- No external dependencies, pure HTML/CSS

**Commit:** `feat: add CommunityBot landing page`

---

### Task 4: Product README

Create `products/communitybot/README.md` — non-developer friendly guide.

Sections:
- What is CommunityBot (1 paragraph)
- Quick Deploy (Fly.io / Railway / Render / Docker — step by step)
- Configuration (how to change LLM, add channels)
- Customization (edit SOUL.md, enable/disable skills)
- FAQ (common issues)

Write for someone who has never used a terminal before. Include exact commands.

**Commit:** `docs: add CommunityBot README for non-developers`

---

### Task 5: CLI Product Command

Add `amanclaw product` subcommand to scaffold product directories.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` — add `Product` command with `ProductAction` enum (New, List)
- Create: `rust/crates/amanclaw-cli/src/product_scaffold.rs` — product scaffolding logic
- Modify: `rust/crates/amanclaw-cli/src/main.rs` — add mod + handler

**`ProductAction` enum:**
```rust
#[derive(Subcommand, Debug)]
pub enum ProductAction {
    /// Create a new product from template
    New {
        /// Product template name (e.g. "communitybot")
        template: String,
        /// Output directory
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List available product templates
    List,
}
```

**`product_scaffold.rs`:**
- `scaffold_product(template, output_dir)` — generates all product files
- `list_templates()` — returns vec of available templates
- Templates are embedded as const strings
- Only "communitybot" template for now (extensible pattern for future SolatBot, UstazBot)

**Tests:**
- CLI parsing: test_cli_product_new, test_cli_product_list
- Scaffold: test_scaffold_communitybot creates all expected files
- List: test_list_templates returns communitybot

**Run:** `cargo test -p amanclaw-cli`

**Commit:** `feat(cli): add product scaffold command (amanclaw product new/list)`

---

### Task 6: Update README

Add CommunityBot to the main project README.

- Add "Specialized Products" section
- CommunityBot description + deploy buttons
- Link to products/communitybot/README.md

**Commit:** `docs: update README with CommunityBot product`

---

## Parallelization

- **Tasks 1 + 2 + 3 + 4**: All independent (static files, no code deps)
- **Task 5**: Independent (Rust code, no deps on static files)
- **Task 6**: Last (depends on all others)

Recommended: Tasks 1-5 parallel, Task 6 last.
