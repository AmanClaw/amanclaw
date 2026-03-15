# Repository Restructure — Design Spec

**Date:** 2026-03-15
**Scope:** Reorganize the AmanClaw monorepo from organic growth layout to clean monorepo structure
**Approach:** Flat crates + top-level apps (Approach B)

---

## Goal

Move from the current `rust/` wrapper layout to a clean monorepo where `Cargo.toml` is at root, code is organized by role (crates, apps, skills, channels), and infra concerns are grouped.

## Current Structure (problems)

```
amanclaw/
├── rust/                    ← unnecessary wrapper, cargo commands need cd
│   ├── Cargo.toml           ← workspace root buried one level deep
│   ├── crates/              ← 16 library crates
│   ├── plugins/             ← mixed skills + channel adapters
│   ├── sdks/
│   ├── Dockerfile
│   └── docker-compose.yml
├── cloud/                   ← sits outside workspace, awkward paths
├── dashboard/               ← sibling to rust/, not grouped with other apps
├── desktop/                 ← same
├── deploy/                  ← K8s manifests separate from Docker
├── plugins/                 ← Python scripts (duplicate name with rust/plugins/)
├── souls/                   ← also exists at rust/souls/ (duplicate)
├── wa-bridge/               ← loose Node.js bridge
├── deploy-raspi.sh          ← loose scripts at root
└── install.sh
```

## New Structure

```
amanclaw/
├── Cargo.toml               # Workspace root at top level
├── Cargo.lock
├── crates/                   # All Rust library crates (15)
│   ├── amanclaw-traits/
│   ├── amanclaw-core/
│   ├── amanclaw-memory/
│   ├── amanclaw-llm/
│   ├── amanclaw-security/
│   ├── amanclaw-mcp/
│   ├── amanclaw-api/
│   ├── amanclaw-gateway/
│   ├── amanclaw-islamic-db/
│   ├── amanclaw-registry/
│   ├── amanclaw-skill-index/
│   ├── amanclaw-plugin-sdk/
│   ├── amanclaw-wasm-runtime/
│   ├── amanclaw-script-runtime/
│   └── amanclaw-prayer-times/
├── apps/                     # Binary applications
│   ├── cli/                  # amanclaw binary
│   ├── cloud/                # amanclaw-cloud binary
│   ├── dashboard/            # Svelte web dashboard
│   └── desktop/              # Tauri desktop app
├── skills/                   # Rust skill plugins (10)
│   ├── skill-solat/
│   ├── skill-quran/
│   ├── skill-qiblat/
│   ├── skill-hijri/
│   ├── skill-doa/
│   ├── skill-hadith-rs/
│   ├── skill-fiqh/
│   ├── skill-shell/
│   ├── skill-sysinfo/
│   └── skill-echo-wasm/
├── channels/                 # Channel adapter crates (5)
│   ├── channel-telegram/
│   ├── channel-discord/
│   ├── channel-whatsapp/
│   ├── channel-whatsapp-web/
│   └── channel-slack/
├── plugins/                  # Script plugins (Python/JS)
│   ├── skill_web_search.py
│   ├── skill_hadith.py
│   └── ...
├── sdks/                     # Plugin development SDKs
│   ├── python/
│   └── assemblyscript/
├── infra/                    # All infrastructure/deployment
│   ├── docker/
│   │   ├── Dockerfile
│   │   ├── Dockerfile.cloud
│   │   └── docker-compose.yml
│   ├── k3s/                  # K8s manifests
│   ├── scripts/              # Deploy, setup, backup, install scripts
│   └── wa-bridge/            # WhatsApp Web bridge
├── docs/
│   ├── images/
│   ├── specs/                # Design specs (flattened from superpowers/specs/)
│   └── plans/                # Implementation plans (merged)
├── products/
│   └── communitybot/
├── souls/                    # SOUL.md persona files (single location)
├── wit/                      # WASM Interface Types
├── .github/
│   ├── ISSUE_TEMPLATE/
│   └── workflows/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── CHANGELOG.md
├── SECURITY.md
├── config.example.yaml
└── .env.example
```

## Migration Map

### Directory Moves

| From | To |
|------|----|
| `rust/Cargo.toml` | `Cargo.toml` |
| `rust/Cargo.lock` | `Cargo.lock` |
| `rust/crates/*` | `crates/*` |
| `rust/crates/amanclaw-cli/` | `apps/cli/` |
| `rust/plugins/skill-*` | `skills/skill-*` |
| `rust/plugins/channel-*` | `channels/channel-*` |
| `rust/sdks/*` | `sdks/*` |
| `rust/wit/` | `wit/` |
| `rust/Dockerfile` | `infra/docker/Dockerfile` |
| `rust/docker-compose.yml` | `infra/docker/docker-compose.yml` |
| `rust/.cargo/` | `.cargo/` |
| `cloud/` | `apps/cloud/` |
| `cloud/Dockerfile` | `infra/docker/Dockerfile.cloud` |
| `dashboard/` | `apps/dashboard/` |
| `desktop/` | `apps/desktop/` |
| `deploy/k3s/` | `infra/k3s/` |
| `deploy/scripts/` | `infra/scripts/` |
| `wa-bridge/` | `infra/wa-bridge/` |
| `deploy-raspi.sh` | `infra/scripts/deploy-raspi.sh` |
| `install.sh` | `infra/scripts/install.sh` |
| `docs/superpowers/specs/` | `docs/specs/` |
| `docs/superpowers/plans/` | `docs/plans/` |
| `docs/plans/` (old) | `docs/plans/` (merge) |

### Deletions

| Path | Reason |
|------|--------|
| `rust/` | Empty after migration |
| `rust/souls/` | Duplicate of root `souls/` |
| `rust/docs/` | Content merged into root `docs/` |
| `data/` | Runtime data, add to .gitignore |

### Cargo.toml Path Updates

Every crate's `Cargo.toml` needs relative path references updated. The workspace members list changes to:

```toml
members = [
    "crates/amanclaw-traits", "crates/amanclaw-core", "crates/amanclaw-memory",
    "crates/amanclaw-llm", "crates/amanclaw-security", "crates/amanclaw-mcp",
    "crates/amanclaw-api", "crates/amanclaw-gateway", "crates/amanclaw-islamic-db",
    "crates/amanclaw-registry", "crates/amanclaw-skill-index",
    "crates/amanclaw-plugin-sdk", "crates/amanclaw-wasm-runtime",
    "crates/amanclaw-script-runtime", "crates/amanclaw-prayer-times",
    "apps/cli", "apps/cloud",
    "skills/skill-solat", "skills/skill-quran", "skills/skill-qiblat",
    "skills/skill-hijri", "skills/skill-doa", "skills/skill-hadith-rs",
    "skills/skill-fiqh", "skills/skill-shell", "skills/skill-sysinfo",
    "skills/skill-echo-wasm",
    "channels/channel-telegram", "channels/channel-discord",
    "channels/channel-whatsapp", "channels/channel-whatsapp-web",
    "channels/channel-slack",
]
```

### CI/CD Updates

- `cd rust && cargo test` → `cargo test`
- `cd rust && cargo clippy` → `cargo clippy`
- `cd dashboard && npm run build` → `cd apps/dashboard && npm run build`
- Docker build context paths in release workflows
- Desktop release workflow paths

### Other Updates

- README.md architecture diagram
- Dockerfile COPY paths
- Tauri config paths in desktop app
- Products/communitybot references
