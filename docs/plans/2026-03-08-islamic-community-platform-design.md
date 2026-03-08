# AmanClaw Islamic Community Platform Design

**Date:** 2026-03-08
**Status:** Approved

## Overview

Extend AmanClaw with 11 Islamic/Malaysian-focused skills and a multi-community platform to serve Muslim communities across Malaysia. The platform supports all 5 chat adapters (Telegram, WhatsApp, WhatsApp Web, Discord, Slack) with bilingual BM+English support and rojak-style natural language.

## Goals

1. Build 11 Islamic skills covering daily Muslim life in Malaysia
2. Add multi-tenancy so one AmanClaw instance serves many community groups
3. Enable self-onboarding via in-chat wizard and web dashboard
4. Eventually offer managed hosting (AmanClaw Cloud) for non-technical communities

## Approach: Skills-First

Build all 11 Islamic skills first, then layer multi-tenancy and platform features on top. This delivers immediate value and allows dogfooding before scaling.

---

## Section 1: Islamic Skills

### Rust Built-in Skills (5)

High-frequency, performance-critical skills compiled into the binary.

| Skill | Data Source | Key Functions |
|-------|-----------|---------------|
| `skill-solat` | JAKIM e-Solat API | Prayer times by zone, proactive azan reminders, subscription management per group |
| `skill-quran` | Quran.com API + local SQLite cache | Search by surah/ayat/keyword, tafsir (BM + English), daily verse push |
| `skill-qiblat` | Great Circle calculation | Qiblat direction + degree from user's location |
| `skill-hijri` | Hijri algorithm + JAKIM calendar | Date conversion, upcoming Islamic events, Ramadan countdown |
| `skill-doa` | Local JSON/SQLite database | Daily doa, morning/evening azkar, doa by category (makan, tidur, musafir, etc.) |

### Python Script Plugins (6)

Community-friendly, easier to update and contribute to.

| Skill | Data Source | Key Functions |
|-------|-----------|---------------|
| `skill-hadith` | sunnah.com API / local DB | Search by keyword, Bukhari/Muslim/Abu Dawud collections, daily hadith |
| `skill-halal` | JAKIM Halal Portal | Verify product/restaurant/premises by name or cert number |
| `skill-zakat` | JAKIM rates (yearly update) | Calculate zakat fitrah, pendapatan, harta, savings. Zone-specific rates |
| `skill-masjid` | Google Places API + JAWHAR | Find nearest masjid/surau by location, prayer time at specific masjid |
| `skill-khutbah` | JAKIM weekly khutbah | Latest khutbah summary, searchable archive, BM + English |
| `skill-jakim` | JAKIM portal | General JAKIM services, fatwa search, Islamic events calendar |

### Data Sources

Primary: Official Malaysian government APIs (JAKIM e-Solat, JAKIM Halal Portal, JAWHAR).
Supplementary: Quran.com, sunnah.com for universal Islamic content.

---

## Section 2: Multi-Tenancy & Community Model

### Community Entity

```
Community {
    id: UUID
    name: "Masjid Al-Hidayah Shah Alam"
    zone: "SGR01"                    // JAKIM prayer zone
    language: "rojak"                // bm | en | rojak
    platform: "whatsapp"             // where this group lives
    platform_group_id: "..."         // Telegram group ID / WA group ID / Discord server ID
    enabled_skills: ["solat", "quran", "halal", "doa", "zakat"]
    notifications: {
        solat_reminder: true,        // 5 mins before each waktu
        daily_doa: true,             // morning azkar after Subuh
        daily_quran: true,           // verse after Maghrib
        weekly_khutbah: true         // Friday morning
    }
    admin_user_ids: [...]
    created_at: timestamp
}
```

### Storage

Extend existing SQLite database with `communities` and `community_settings` tables.

### Behavior

- Message from group -> lookup community config by `platform_group_id`
- No community found -> trigger onboarding wizard
- Community admins update settings via commands
- Individual user preferences override community defaults

---

## Section 3: Onboarding & User Experience

### In-Chat Wizard Flow

```
1. Bot added to group
   -> "Assalamualaikum! Saya AmanClaw, pembantu AI untuk komuniti Muslim Malaysia."
   -> "Siapa admin untuk setup? Sila taip /admin"

2. Admin claims ownership
   -> /admin -> bot verifies group admin status (platform-specific)

3. Zone selection
   -> "Pilih negeri:" [Selangor | Johor | Kedah | ...]
   -> "Pilih zon:" [SGR01 | SGR02 | ...] (based on state)

4. Language
   -> "Bahasa pilihan:" [BM | English | Rojak]

5. Skills
   -> "Pilih servis:" (toggle list, all ON by default)

6. Done
   -> "Setup selesai! Taip /help untuk senarai arahan."
```

### Admin Commands

- `/setzone <zone>` - change prayer zone
- `/setlang <bm|en|rojak>` - change language
- `/enable <skill>` / `/disable <skill>` - toggle skills
- `/notify <on|off>` - toggle push notifications
- `/community` - show current settings

### User Commands

- `/solat` - today's prayer times for group zone
- `/solat <zone>` - prayer times for specific zone
- `/quran <surah>:<ayat>` - lookup verse
- `/cari <keyword>` - search Quran + Hadith
- `/halal <product/restaurant>` - check halal status
- `/zakat` - interactive zakat calculator
- `/qiblat` - qiblat direction (asks for location)
- `/doa <category>` - doa lookup
- `/masjid` - nearest masjid (asks for location)
- `/hijri` - today's Hijri date
- `/khutbah` - latest weekly khutbah

### Natural Language

LLM-powered queries also work: "Bila waktu Maghrib hari ni?", "Is KFC halal?", "Doa sebelum makan"

### Language Support

- Bilingual: Bahasa Melayu + English
- Rojak mode: natural mix of BM/English like Malaysians actually chat
- User/community chooses preferred language, bot responds accordingly

---

## Section 4: Web Dashboard (Phase 2)

Simple admin panel for non-technical community managers.

```
dashboard.amanclaw.my
├── /login          -> Login via Telegram/WhatsApp (same account as group admin)
├── /communities    -> List of communities you manage
├── /community/:id  -> Settings page
│   ├── General     -> Name, zone, language
│   ├── Skills      -> Toggle skills on/off
│   ├── Notifications -> Schedule & content settings
│   ├── Members     -> User list, roles
│   └── Analytics   -> Usage stats (messages, most used skills)
└── /onboard        -> Generate bot invite link with pre-config
```

Tech: Static site or lightweight framework, calls AmanClaw via MCP HTTP server or thin REST API.

---

## Section 5: Managed Platform (Phase 3)

### AmanClaw Cloud

Hosted service for non-technical communities.

- Single/few AmanClaw instances on managed infrastructure
- Communities onboard via bot invite link - zero setup required
- Freemium model:
  - **Free tier:** solat, doa, hijri, qiblat (low API cost skills)
  - **Paid tier:** halal check, quran search, khutbah, masjid finder, zakat (higher API/LLM usage)
- Revenue covers server costs, LLM API costs, JAKIM API usage

### Self-Hosted (Open Source)

- Users clone repo, configure bot token + LLM key, deploy on own server
- Same codebase as managed platform
- Full documentation for deployment (Docker, systemd, Raspberry Pi)

---

## Section 6: Phased Roadmap

| Phase | What | Outcome |
|-------|------|---------|
| **1** | 11 Islamic skills + multi-community config | Working Islamic assistant on all platforms |
| **2** | In-chat onboarding wizard + web dashboard | Communities can self-onboard |
| **3** | AmanClaw Cloud (managed hosting) | Non-technical communities served |
| **4** | Specialized bots (UstazBot, HalalBot) | Focused experiences spun from same codebase |
| **5** | Open source plugin marketplace | Community contributes skills |

---

## Architecture Decisions

1. **Individual plugins per feature** - not a monolithic Islamic skill. Enables independent development, hot-reload, community contribution.
2. **Rust for core 5 skills, Python for other 6** - performance where it matters, accessibility where contribution matters.
3. **Official JAKIM APIs as primary data source** - authenticity and trust for Malaysian Muslim community.
4. **Single instance multi-tenancy first** - simpler infra, evolve to specialized bots later.
5. **In-chat wizard + web dashboard** - quick start via chat, detailed config via web.
6. **Skills-first approach** - deliver value immediately, platform features layered on top.
