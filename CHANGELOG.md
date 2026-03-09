# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with test, lint, and security audit
- Contributor documentation (CONTRIBUTING.md, SECURITY.md)
- Docker improvements (.dockerignore, healthcheck)
- GitHub issue templates

## [0.1.0] - 2026-03-09

### Added
- Core engine with middleware pipeline
- 7 built-in Rust skills (solat, qiblat, hijri, doa, quran, sysinfo, shell)
- 6 Python plugins (hadith, halal, zakat, masjid, khutbah, jakim)
- 5 channel adapters (Telegram, Discord, WhatsApp, WhatsApp Web, Slack)
- WASM plugin runtime with sandboxing
- Python/JS script runtime
- MCP client support
- Multi-agent routing with SOUL.md personas
- Cron scheduler with timezone support
- Webhook triggers with auth validation
- WebSocket gateway (JSON-RPC 2.0)
- Sub-agent spawning
- Skill marketplace/registry
- FTS5 hybrid search (BM25 + vector)
- SQLite memory backend with vector store
- Security: auth, rate limiting, injection detection
- Desktop admin app (Svelte + Tauri)
- Docker support with security hardening
- Raspberry Pi deployment script
