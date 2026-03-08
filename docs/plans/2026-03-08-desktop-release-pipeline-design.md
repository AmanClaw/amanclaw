# AmanClaw Desktop Release Pipeline Design

**Date:** 2026-03-08
**Status:** Approved

## Overview

GitHub Actions CI/CD pipeline to build and release the AmanClaw Desktop app for macOS, Windows, and Linux. Triggered by git tags (`v*`). No code signing for v0.1.0. Icon generation from a single source PNG.

## Trigger

Push a git tag matching `v*` (e.g. `v0.1.0`) triggers the workflow.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Build Matrix

| Platform | Runner | Architecture | Outputs |
|----------|--------|-------------|---------|
| macOS | macos-latest | aarch64 + x86_64 (universal) | .dmg |
| Windows | windows-latest | x86_64 | .msi, .exe |
| Linux | ubuntu-22.04 | x86_64 | .deb, .AppImage |

## Workflow Structure

Single file: `.github/workflows/release-desktop.yml`

```
on: push tag v*
  → Job: build (matrix: macos, windows, linux)
    1. Checkout code
    2. Install Rust toolchain
    3. Install Node.js + npm
    4. npm install (desktop/)
    5. cargo tauri build
    6. Upload build artifacts
  → Job: release (needs: build)
    1. Download all artifacts
    2. Create GitHub Release from tag
    3. Attach all platform binaries
```

## Icon Pipeline

Source: `desktop/src-tauri/icons/app-icon.png` (1024x1024, provided by user)

Generated sizes:
- 32x32.png, 128x128.png, 128x128@2x.png (256x256)
- icon.ico (Windows, multi-size)
- icon.icns (macOS, multi-size)

Tool: `sharp` npm package via a `generate-icons.mjs` script.

Run manually or as prebuild step. Generated icons committed to repo.

## Tauri Config

- Bundle identifier: `my.amanclaw.desktop`
- All platform targets enabled
- Icon paths include .ico and .icns
- No signing configuration
- No auto-update (requires signing)

## Security

- No code signing (open source project, users bypass OS warnings)
- Signing can be added later via GitHub Secrets without pipeline changes
- No secrets required for the current pipeline

## Architecture Decisions

1. **Single workflow, matrix strategy** — keeps all platform builds in one file, easier to maintain.
2. **No signing for v0.1.0** — saves $300-500/year, standard for open source projects, can add later.
3. **Icon generation script** — user provides one 1024x1024 PNG, script generates all sizes including .ico/.icns.
4. **GitHub Releases** — standard distribution for open source, users download from releases page.
5. **No auto-update** — requires signing keys. Add in future when signing is configured.
