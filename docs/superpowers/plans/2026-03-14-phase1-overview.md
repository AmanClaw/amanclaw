# Phase 1: General Agent Parity — Overview

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform AmanClaw from a chat-app bot into a full-featured AI personal agent that developers can use from the terminal, with general-purpose skills and a plugin marketplace.

**Architecture:** Phase 1 is broken into 5 independent sub-plans that can be executed in parallel or sequentially. Each produces working, testable software on its own.

**Tech Stack:** Rust, clap, tokio, wasmtime, Python/JS script runtime, GitHub Actions

---

## Sub-Plans

Execute in this order (dependencies noted):

### Plan 1A: CLI Agent Mode (no dependencies)
**File:** `2026-03-14-phase1a-cli-agent-mode.md`
**Scope:** Add `amanclaw ask`, `amanclaw chat`, and `amanclaw agent` commands
**Impact:** Opens AmanClaw to developers beyond chat apps — highest priority

### Plan 1B: MCP Enhancements (no dependencies)
**File:** `2026-03-14-phase1b-mcp-enhancements.md`
**Scope:** SSE transport, Resources support, `amanclaw mcp` CLI commands
**Impact:** Enables thousands of community integrations

### Plan 1C: General Skills Batch 1 (no dependencies)
**File:** `2026-03-14-phase1c-general-skills.md`
**Scope:** 15 Python/JS skills across developer, research, and productivity categories
**Impact:** Core value proposition — the agent can actually do useful things

### Plan 1D: Skill Marketplace CLI (depends on 1C for content)
**File:** `2026-03-14-phase1d-skill-marketplace.md`
**Scope:** `amanclaw install/search/list` with remote registry
**Impact:** Ecosystem growth — community can contribute and discover skills

### Plan 1E: Polish & Publish (no dependencies)
**File:** `2026-03-14-phase1e-polish-publish.md`
**Scope:** Test coverage for all crates, crates.io publish, benchmark suite
**Impact:** Open source readiness — credibility for GitHub launch

---

## Current State (What's Already Done)

- CI/CD pipeline (ci.yml, release.yml, release-desktop.yml) ✓
- CONTRIBUTING.md, CHANGELOG.md, SECURITY.md ✓
- .dockerignore ✓
- MCP client/server (stdio + HTTP) ✓
- Plugin system (WASM + Script + Registry) ✓
- Skill index with search and packs ✓
- Dashboard with Skills and MCP Servers pages ✓
- 7 built-in Rust skills + 6 Python plugins ✓
- 10 integration tests + 2 benchmark suites ✓
