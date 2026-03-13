use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Available product templates.
pub fn list_templates() -> Vec<(&'static str, &'static str)> {
    vec![(
        "communitybot",
        "Friendly AI assistant for community group chats",
    )]
}

/// Scaffold a product directory from a template.
pub fn scaffold_product(template: &str, output_dir: Option<&str>) -> Result<PathBuf> {
    match template {
        "communitybot" => scaffold_communitybot(output_dir),
        other => bail!(
            "Unknown product template: {other}. Use 'amanclaw product list' to see available templates."
        ),
    }
}

fn scaffold_communitybot(output_dir: Option<&str>) -> Result<PathBuf> {
    let base = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("products"));
    let product_dir = base.join("communitybot");
    let souls_dir = product_dir.join("souls");

    std::fs::create_dir_all(&souls_dir)
        .with_context(|| format!("Failed to create {}", souls_dir.display()))?;

    // config.yaml
    write_file(&product_dir.join("config.yaml"), COMMUNITYBOT_CONFIG)?;

    // .env.example
    write_file(&product_dir.join(".env.example"), COMMUNITYBOT_ENV)?;

    // souls/community.md
    write_file(&souls_dir.join("community.md"), COMMUNITYBOT_SOUL)?;

    // Dockerfile
    write_file(&product_dir.join("Dockerfile"), COMMUNITYBOT_DOCKERFILE)?;

    // docker-compose.yml
    write_file(
        &product_dir.join("docker-compose.yml"),
        COMMUNITYBOT_COMPOSE,
    )?;

    // fly.toml
    write_file(&product_dir.join("fly.toml"), COMMUNITYBOT_FLY)?;

    // railway.json
    write_file(&product_dir.join("railway.json"), COMMUNITYBOT_RAILWAY)?;

    // render.yaml
    write_file(&product_dir.join("render.yaml"), COMMUNITYBOT_RENDER)?;

    Ok(product_dir)
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

const COMMUNITYBOT_CONFIG: &str = "\
# CommunityBot — Pre-configured AmanClaw for community groups
llm:
  base_url: \"http://localhost:11434/v1\"
  model: \"llama3\"
  max_tokens: 1024
  temperature: 0.7

admin_users:
  telegram: []

skills:
  disabled: []

memory:
  db_path: \"data/memory.db\"

agents:
  - name: community
    soul: \"souls/community.md\"

learning:
  enabled: true

rate_limit:
  max_messages_per_minute: 30
  max_messages_per_hour: 300
";

const COMMUNITYBOT_ENV: &str = "\
# CommunityBot Environment Variables
# Copy this file to .env and fill in your values.

# === REQUIRED: Pick at least ONE channel ===
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
# DISCORD_BOT_TOKEN=your-discord-bot-token
# WHATSAPP_ACCESS_TOKEN=your-whatsapp-token
# WHATSAPP_PHONE_NUMBER_ID=your-phone-number-id

# === LLM SETTINGS ===
# Default: Ollama running locally (free, no API key needed)
# LLM_BASE_URL=http://localhost:11434/v1
# LLM_MODEL=llama3
# LLM_API_KEY=not-needed-for-ollama

# For cloud LLMs:
# LLM_BASE_URL=https://api.openai.com/v1
# LLM_MODEL=gpt-4o-mini
# LLM_API_KEY=sk-your-api-key
";

const COMMUNITYBOT_SOUL: &str = "\
# CommunityBot

A friendly general-purpose assistant for community group chats. Helpful without being intrusive, adapting its behavior to the conversation context.

## Personality

- Friendly, approachable, and concise
- Responds in the user's language (Malay, English, or mixed)
- Adapts tone to the group's culture — casual in casual groups, formal when needed
- Supportive and welcoming, especially to new members
- Does not dominate conversations; speaks when spoken to or when genuinely useful

## Capabilities

- Answers general questions and helps find information
- Assists with group coordination (events, polls, schedules)
- Welcomes new members with a brief, warm greeting
- Summarizes long discussions when asked
- Routes specialized queries to the right resources or bots

## Guidelines

- In group chats: keep responses brief (1-3 sentences) to avoid flooding
- In DMs: provide detailed, thorough answers
- Do not reply to every message — only respond when directly addressed or clearly relevant
- Avoid controversial topics (politics, sectarian issues); stay neutral and helpful
- When unsure, offer to help find the answer rather than guessing
- Respect group admins and their rules; do not override moderator decisions
";

const COMMUNITYBOT_DOCKERFILE: &str = "\
FROM ghcr.io/amanclaw/amanclaw:latest
COPY config.yaml /app/config.yaml
COPY souls/ /app/souls/
WORKDIR /app
CMD [\"./amanclaw\"]
";

const COMMUNITYBOT_COMPOSE: &str = "\
services:
  communitybot:
    build: .
    env_file: .env
    volumes:
      - ./data:/app/data
    restart: unless-stopped
";

const COMMUNITYBOT_FLY: &str = "\
app = \"communitybot\"
primary_region = \"sin\"

[build]
  dockerfile = \"Dockerfile\"

[env]
  LLM_BASE_URL = \"http://localhost:11434/v1\"
  LLM_MODEL = \"llama3\"

[http_service]
  internal_port = 8443
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[vm]]
  size = \"shared-cpu-1x\"
  memory = \"512mb\"
";

const COMMUNITYBOT_RAILWAY: &str = "\
{
  \"$schema\": \"https://railway.com/railway.schema.json\",
  \"build\": {
    \"dockerfilePath\": \"Dockerfile\"
  },
  \"deploy\": {
    \"restartPolicyType\": \"ON_FAILURE\",
    \"restartPolicyMaxRetries\": 10
  }
}
";

const COMMUNITYBOT_RENDER: &str = "\
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
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].0, "communitybot");
    }

    #[test]
    fn test_scaffold_communitybot() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let dir = scaffold_product("communitybot", Some(out)).unwrap();

        assert!(dir.join("config.yaml").exists());
        assert!(dir.join(".env.example").exists());
        assert!(dir.join("souls/community.md").exists());
        assert!(dir.join("Dockerfile").exists());
        assert!(dir.join("docker-compose.yml").exists());
        assert!(dir.join("fly.toml").exists());
        assert!(dir.join("railway.json").exists());
        assert!(dir.join("render.yaml").exists());

        // Verify config content
        let config = std::fs::read_to_string(dir.join("config.yaml")).unwrap();
        assert!(config.contains("CommunityBot"));
        assert!(config.contains("llama3"));

        // Verify soul content
        let soul = std::fs::read_to_string(dir.join("souls/community.md")).unwrap();
        assert!(soul.contains("CommunityBot"));
        assert!(soul.contains("Friendly"));
    }

    #[test]
    fn test_scaffold_unknown_template() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let result = scaffold_product("nonexistent", Some(out));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown product template"));
    }
}
