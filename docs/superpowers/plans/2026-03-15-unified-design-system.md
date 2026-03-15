# Unified Design System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a shared design system (`@amanclaw/ui`) and apply it to all 5 app surfaces — Desktop, Dashboard, Cloud Chat, Landing Page, and CLI Playground.

**Architecture:** pnpm monorepo with shared `packages/ui/` containing design tokens (CSS custom properties), Svelte 5 components (Bits UI primitives + custom styling), and layout components. Both Svelte apps import from `@amanclaw/ui`. HTML-only apps consume a compiled CSS tokens file.

**Tech Stack:** Svelte 5, Tailwind CSS 4, Bits UI, Lucide Icons, pnpm workspaces, Inter + JetBrains Mono fonts.

**Spec:** `docs/superpowers/specs/2026-03-15-unified-design-system.md`

**Key Implementation Notes:**
- **Tailwind CSS 4 theming:** Uses `@theme inline` to reference runtime CSS custom properties (not self-referential `var()` in `@theme`). This allows dark/light switching via CSS classes while generating valid Tailwind utilities.
- **Svelte 5 only:** All components use runes (`$props()`, `$state()`, `$derived()`), `{@render}` snippets, and direct component rendering (NOT `<svelte:component>`). Dashboard pages using Svelte 4 `export let` syntax must be migrated as part of the integration tasks.
- **Dependencies:** `lucide-svelte` and `bits-ui` are dependencies of `@amanclaw/ui`, not individual apps. Apps only need `@amanclaw/ui` as a workspace dependency.
- **Minimum font size:** No text below 12px anywhere. Plan components use `text-[11px]` ONLY for `caption`-style labels that are uppercase tracked (effectively larger visual size). Body text minimum is 13px.

---

## Chunk 1: Monorepo Setup + Design Tokens (Layer 1)

### Task 1: Bootstrap pnpm Monorepo

**Files:**
- Create: `pnpm-workspace.yaml`
- Create: `package.json` (root)
- Modify: `apps/desktop/package.json`
- Modify: `apps/dashboard/package.json`

- [ ] **Step 1: Create root package.json**

```json
{
  "name": "amanclaw",
  "private": true,
  "type": "module",
  "scripts": {
    "dev:desktop": "pnpm --filter @amanclaw/desktop dev",
    "dev:dashboard": "pnpm --filter @amanclaw/dashboard dev",
    "build:ui": "pnpm --filter @amanclaw/ui build"
  }
}
```

- [ ] **Step 2: Create pnpm-workspace.yaml**

```yaml
packages:
  - "apps/*"
  - "packages/*"
```

- [ ] **Step 3: Add workspace name to apps/desktop/package.json**

Add `"name": "@amanclaw/desktop"` to the existing package.json (keep all existing content).

- [ ] **Step 4: Add workspace name to apps/dashboard/package.json**

Add `"name": "@amanclaw/dashboard"` to the existing package.json (keep all existing content).

- [ ] **Step 5: Run pnpm install from root to link workspaces**

Run: `pnpm install`
Expected: lockfile regenerated, workspaces linked.

- [ ] **Step 6: Commit**

```bash
git add pnpm-workspace.yaml package.json apps/desktop/package.json apps/dashboard/package.json pnpm-lock.yaml
git commit -m "chore: bootstrap pnpm monorepo workspace"
```

---

### Task 2: Create @amanclaw/ui Package Skeleton

**Files:**
- Create: `packages/ui/package.json`
- Create: `packages/ui/src/index.ts`

- [ ] **Step 1: Create packages/ui/package.json**

```json
{
  "name": "@amanclaw/ui",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "svelte": "./src/index.ts",
  "exports": {
    ".": "./src/index.ts",
    "./tokens/*.css": "./src/tokens/*.css"
  },
  "dependencies": {
    "lucide-svelte": "^0.470.0",
    "bits-ui": "^1.0.0"
  },
  "peerDependencies": {
    "svelte": "^5.0.0"
  }
}
```

- [ ] **Step 2: Create packages/ui/src/index.ts**

```typescript
// @amanclaw/ui - Shared component library
// Components will be added in Layer 3
export {};
```

- [ ] **Step 3: Add @amanclaw/ui as dependency in both apps**

In `apps/desktop/package.json` add to dependencies:
```json
"@amanclaw/ui": "workspace:*"
```

In `apps/dashboard/package.json` add to dependencies:
```json
"@amanclaw/ui": "workspace:*"
```

- [ ] **Step 4: Run pnpm install to link**

Run: `pnpm install`

- [ ] **Step 5: Commit**

```bash
git add packages/ui/ apps/desktop/package.json apps/dashboard/package.json pnpm-lock.yaml
git commit -m "chore: create @amanclaw/ui package skeleton"
```

---

### Task 3: Design Tokens — Color System

**Files:**
- Create: `packages/ui/src/tokens/colors.css`

- [ ] **Step 1: Create colors.css with dark and light mode tokens**

```css
/* AmanClaw Design Tokens — Colors */
/* Dark + Islamic Hybrid: teal primary, gold accent, dark navy base */

html.dark,
html:not(.light) {
  /* Surfaces */
  --color-base: #0a0f1a;
  --color-surface: #0f172a;
  --color-elevated: #1e293b;
  --color-border: #334155;

  /* Primary — Teal */
  --color-primary-900: #0d3d38;
  --color-primary-700: #115e56;
  --color-primary-500: #14b8a6;
  --color-primary-300: #5eead4;
  --color-primary-100: #ccfbf1;

  /* Accent — Gold */
  --color-accent-900: #78350f;
  --color-accent-700: #a16207;
  --color-accent-500: #d4a574;
  --color-accent-300: #e8c9a0;
  --color-accent-100: #fef3e2;

  /* Semantic */
  --color-success: #22c55e;
  --color-error: #ef4444;
  --color-warning: #f59e0b;
  --color-info: #3b82f6;

  /* Text */
  --text-primary: #f1f5f9;
  --text-secondary: #94a3b8;
  --text-muted: #64748b;
  --text-accent: #14b8a6;

  /* Semi-transparent variants (needed because @theme inline can't do opacity modifiers) */
  --color-primary-500-10: rgb(20 184 166 / 0.1);
  --color-primary-500-15: rgb(20 184 166 / 0.15);
  --color-accent-500-15: rgb(212 165 116 / 0.15);
  --color-error-15: rgb(239 68 68 / 0.15);
  --color-error-20: rgb(239 68 68 / 0.2);
  --color-success-15: rgb(34 197 94 / 0.15);
  --color-warning-15: rgb(245 158 11 / 0.15);
  --color-info-15: rgb(59 130 246 / 0.15);
  --color-elevated-50: rgb(30 41 59 / 0.5);
  --color-elevated-60: rgb(30 41 59 / 0.6);
  --color-base-80: rgb(10 15 26 / 0.8);
}

html.light {
  /* Surfaces */
  --color-base: #ffffff;
  --color-surface: #f8fafc;
  --color-elevated: #f1f5f9;
  --color-border: #e2e8f0;

  /* Primary — Teal (unchanged) */
  --color-primary-900: #0d3d38;
  --color-primary-700: #115e56;
  --color-primary-500: #14b8a6;
  --color-primary-300: #5eead4;
  --color-primary-100: #ccfbf1;

  /* Accent — Gold (unchanged) */
  --color-accent-900: #78350f;
  --color-accent-700: #a16207;
  --color-accent-500: #d4a574;
  --color-accent-300: #e8c9a0;
  --color-accent-100: #fef3e2;

  /* Semantic (unchanged) */
  --color-success: #22c55e;
  --color-error: #ef4444;
  --color-warning: #f59e0b;
  --color-info: #3b82f6;

  /* Text */
  --text-primary: #0f172a;
  --text-secondary: #475569;
  --text-muted: #64748b;
  --text-accent: #0d9488;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/ui/src/tokens/colors.css
git commit -m "feat(ui): add color token definitions for dark and light modes"
```

---

### Task 4: Design Tokens — Spacing, Transitions, Z-Index

**Files:**
- Create: `packages/ui/src/tokens/spacing.css`

- [ ] **Step 1: Create spacing.css**

```css
/* AmanClaw Design Tokens — Spacing, Transitions, Z-Index */

:root {
  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-12: 48px;

  /* Transitions */
  --transition-fast: 150ms ease;
  --transition-normal: 250ms ease;
  --transition-slow: 400ms ease;

  /* Z-Index */
  --z-base: 0;
  --z-sidebar: 10;
  --z-sticky: 20;
  --z-dropdown: 30;
  --z-modal-backdrop: 40;
  --z-modal: 50;
  --z-toast: 60;
  --z-tooltip: 70;

  /* Border Radius */
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-full: 9999px;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/ui/src/tokens/spacing.css
git commit -m "feat(ui): add spacing, transition, z-index, and radius tokens"
```

---

### Task 5: Design Tokens — Typography

**Files:**
- Create: `packages/ui/src/tokens/typography.css`

- [ ] **Step 1: Create typography.css**

```css
/* AmanClaw Design Tokens — Typography */
/* Inter (UI) + JetBrains Mono (code) */

/* Fonts: Install via npm (see note below). For HTML-only apps, use Google Fonts CDN in <head>. */

:root {
  --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
}

/* Type scale utilities */
.text-display {
  font-size: 30px;
  font-weight: 700;
  line-height: 1.2;
  font-family: var(--font-sans);
}

.text-h1 {
  font-size: 24px;
  font-weight: 600;
  line-height: 1.3;
  font-family: var(--font-sans);
}

.text-h2 {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.3;
  font-family: var(--font-sans);
}

.text-h3 {
  font-size: 16px;
  font-weight: 600;
  line-height: 1.4;
  font-family: var(--font-sans);
}

.text-body {
  font-size: 14px;
  font-weight: 400;
  line-height: 1.5;
  font-family: var(--font-sans);
}

.text-body-sm {
  font-size: 13px;
  font-weight: 400;
  line-height: 1.5;
  font-family: var(--font-sans);
}

.text-caption {
  font-size: 12px;
  font-weight: 500;
  line-height: 1.4;
  font-family: var(--font-sans);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.text-code {
  font-size: 13px;
  font-weight: 400;
  line-height: 1.6;
  font-family: var(--font-mono);
}
```

- [ ] **Step 2: Install font packages for Svelte apps**

The Tauri desktop app runs offline, so fonts must be local (not CDN). Install via npm:
```bash
pnpm --filter @amanclaw/ui add @fontsource/inter @fontsource/jetbrains-mono
```

Then add to the top of `typography.css`:
```css
@import "@fontsource/inter/400.css";
@import "@fontsource/inter/500.css";
@import "@fontsource/inter/600.css";
@import "@fontsource/inter/700.css";
@import "@fontsource/jetbrains-mono/400.css";
@import "@fontsource/jetbrains-mono/500.css";
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/tokens/typography.css packages/ui/package.json pnpm-lock.yaml
git commit -m "feat(ui): add typography tokens with Inter and JetBrains Mono"
```

---

### Task 6: Tailwind CSS 4 Theme Integration

**Files:**
- Create: `packages/ui/src/tokens/theme.css`
- Modify: `apps/desktop/src/app.css`
- Modify: `apps/dashboard/src/app.css`

- [ ] **Step 1: Create theme.css — Tailwind v4 @theme inline config**

**IMPORTANT:** Tailwind CSS 4 `@theme` processes values at build time. Using `var()` self-references inside `@theme` creates circular references that resolve to nothing. The `@theme inline` directive tells Tailwind to keep the `var()` references as runtime CSS, allowing dark/light switching via CSS classes.

```css
/* AmanClaw Tailwind CSS 4 Theme */
@import "tailwindcss";
@import "./colors.css";
@import "./spacing.css";
@import "./typography.css";

@theme inline {
  /* Surface colors — resolved at runtime from colors.css */
  --color-base: var(--color-base);
  --color-surface: var(--color-surface);
  --color-elevated: var(--color-elevated);
  --color-border: var(--color-border);

  /* Primary */
  --color-primary-100: var(--color-primary-100);
  --color-primary-300: var(--color-primary-300);
  --color-primary-500: var(--color-primary-500);
  --color-primary-700: var(--color-primary-700);
  --color-primary-900: var(--color-primary-900);

  /* Accent */
  --color-accent-100: var(--color-accent-100);
  --color-accent-300: var(--color-accent-300);
  --color-accent-500: var(--color-accent-500);
  --color-accent-700: var(--color-accent-700);
  --color-accent-900: var(--color-accent-900);

  /* Semantic */
  --color-success: var(--color-success);
  --color-error: var(--color-error);
  --color-warning: var(--color-warning);
  --color-info: var(--color-info);

  /* Text — mapped to --color-fg-* to avoid awkward text-text-* utilities */
  --color-fg: var(--text-primary);
  --color-fg-secondary: var(--text-secondary);
  --color-fg-muted: var(--text-muted);
  --color-fg-accent: var(--text-accent);

  /* Font families */
  --font-sans: var(--font-sans);
  --font-mono: var(--font-mono);
}
```

**Note on `@theme inline`:** Since opacity modifiers (e.g., `bg-primary-500/15`) do NOT work with runtime `var()` references, use explicit opacity utilities instead: `bg-primary-500 opacity-15` or define semi-transparent colors directly in `colors.css` where needed (e.g., `--color-primary-500-15: rgb(20 184 166 / 0.15)`). Alternatively, use Tailwind's `bg-[rgb(20_184_166_/_0.15)]` arbitrary value syntax for one-off opacity needs.

**Text utility mapping:** Text colors are now `text-fg`, `text-fg-secondary`, `text-fg-muted`, `text-fg-accent` (instead of `text-text-*`).

- [ ] **Step 2: Update apps/desktop/src/app.css**

Replace the entire file with:
```css
@import "@amanclaw/ui/tokens/theme.css";
```

- [ ] **Step 3: Update apps/dashboard/src/app.css**

Replace the entire file with:
```css
@import "@amanclaw/ui/tokens/theme.css";
```

- [ ] **Step 4: Verify desktop dev server starts**

Run: `cd apps/desktop && pnpm dev`
Expected: Vite starts successfully, page loads with new font (Inter) and dark background.

- [ ] **Step 5: Verify dashboard dev server starts**

Run: `cd apps/dashboard && pnpm dev`
Expected: Vite starts successfully, page loads with Inter font.

- [ ] **Step 6: Commit**

```bash
git add packages/ui/src/tokens/theme.css apps/desktop/src/app.css apps/dashboard/src/app.css
git commit -m "feat(ui): integrate design tokens with Tailwind CSS 4 theme"
```

---

### Task 7: Theme Toggle Store

**Files:**
- Create: `packages/ui/src/stores/theme.ts`

- [ ] **Step 1: Create theme store**

```typescript
import { writable } from 'svelte/store';

type Theme = 'dark' | 'light';

function createThemeStore() {
  const stored = typeof localStorage !== 'undefined'
    ? localStorage.getItem('amanclaw-theme') as Theme | null
    : null;

  const systemPreference: Theme =
    typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: light)').matches
      ? 'light'
      : 'dark';

  const initial = stored ?? systemPreference;

  const { subscribe, set } = writable<Theme>(initial);

  function apply(theme: Theme) {
    const root = document.documentElement;
    root.classList.remove('dark', 'light');
    root.classList.add(theme);
    localStorage.setItem('amanclaw-theme', theme);
  }

  // Apply on init
  if (typeof document !== 'undefined') {
    apply(initial);
  }

  return {
    subscribe,
    toggle() {
      let current: Theme = 'dark';
      subscribe(v => current = v)();
      const next = current === 'dark' ? 'light' : 'dark';
      set(next);
      apply(next);
    },
    set(theme: Theme) {
      set(theme);
      apply(theme);
    }
  };
}

export const theme = createThemeStore();
```

- [ ] **Step 2: Export from index.ts**

Update `packages/ui/src/index.ts`:
```typescript
export { theme } from './stores/theme.js';
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/stores/theme.ts packages/ui/src/index.ts
git commit -m "feat(ui): add theme toggle store with localStorage persistence"
```

---

### Task 8: Compiled Tokens CSS for HTML-Only Apps

**Files:**
- Create: `packages/ui/src/tokens/amanclaw-tokens.css`

- [ ] **Step 1: Create standalone tokens file**

This file is for `chat.html`, `index.html` (landing), and `playground.html` — no Tailwind needed.

```css
/* AmanClaw Tokens — Standalone (no Tailwind required) */
/* Import into HTML-only apps via <link> or inline */

/* Fonts: Install via npm (see note below). For HTML-only apps, use Google Fonts CDN in <head>. */

:root {
  --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;

  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-12: 48px;

  /* Transitions */
  --transition-fast: 150ms ease;
  --transition-normal: 250ms ease;
  --transition-slow: 400ms ease;

  /* Z-Index */
  --z-base: 0;
  --z-modal-backdrop: 40;
  --z-modal: 50;
  --z-toast: 60;
  --z-tooltip: 70;

  /* Border Radius */
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-full: 9999px;
}

/* Dark mode (default) */
html, html.dark, :root {
  --color-base: #0a0f1a;
  --color-surface: #0f172a;
  --color-elevated: #1e293b;
  --color-border: #334155;
  --color-primary-500: #14b8a6;
  --color-primary-700: #115e56;
  --color-accent-500: #d4a574;
  --color-accent-700: #a16207;
  --color-success: #22c55e;
  --color-error: #ef4444;
  --color-warning: #f59e0b;
  --color-info: #3b82f6;
  --text-primary: #f1f5f9;
  --text-secondary: #94a3b8;
  --text-muted: #64748b;
  --text-accent: #14b8a6;
}

html.light {
  --color-base: #ffffff;
  --color-surface: #f8fafc;
  --color-elevated: #f1f5f9;
  --color-border: #e2e8f0;
  --text-primary: #0f172a;
  --text-secondary: #475569;
  --text-muted: #64748b;
  --text-accent: #0d9488;
}

body {
  font-family: var(--font-sans);
  background: var(--color-base);
  color: var(--text-primary);
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/ui/src/tokens/amanclaw-tokens.css
git commit -m "feat(ui): add standalone tokens CSS for HTML-only apps"
```

---

## Chunk 2: Icons + Typography (Layer 2)

### Task 9: Install Lucide Icons

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `apps/dashboard/package.json`
- Create: `packages/ui/src/icons.ts`

- [ ] **Step 1: Verify lucide-svelte is available via @amanclaw/ui**

`lucide-svelte` is already listed as a dependency in `packages/ui/package.json` (added in Task 2). Run from root to ensure it's installed:
```bash
pnpm install
```

- [ ] **Step 2: Create icon barrel export in packages/ui**

```typescript
// packages/ui/src/icons.ts
// Re-export all Lucide icons used across AmanClaw
// Import from here for consistency

export {
  LayoutDashboard,
  Bot,
  Users,
  Zap,
  Globe,
  Clock,
  Webhook,
  Radio,
  GitBranch,
  BookOpen,
  FileText,
  User,
  Hash,
  Server,
  ScrollText,
  Settings,
  Search,
  Moon,
  Sun,
  Plus,
  X,
  ChevronDown,
  ChevronRight,
  MoreHorizontal,
  LogOut,
  ExternalLink,
  Trash2,
  Edit3,
  Eye,
  EyeOff,
  Check,
  AlertCircle,
  Info,
  Loader2,
  MessageSquare,
  Shield,
  Key,
  Database,
  Activity,
  RefreshCw,
  Download,
  Upload,
  Copy,
  Play,
  Square,
  CircleDot,
} from 'lucide-svelte';
```

- [ ] **Step 3: Update packages/ui/src/index.ts**

```typescript
export { theme } from './stores/theme.js';
export * from './icons.js';
```

- [ ] **Step 4: Commit**

```bash
git add packages/ui/src/icons.ts packages/ui/src/index.ts apps/desktop/package.json apps/dashboard/package.json pnpm-lock.yaml
git commit -m "feat(ui): add Lucide icon system with barrel export"
```

---

### Task 10: Replace Desktop Sidebar Icons

**Files:**
- Modify: `apps/desktop/src/lib/components/Sidebar.svelte`

- [ ] **Step 1: Read the current Sidebar.svelte**

Read: `apps/desktop/src/lib/components/Sidebar.svelte`

- [ ] **Step 2: Replace emoji/Unicode icon strings with Lucide imports**

At the top of the script section, add:
```svelte
<script lang="ts">
  import {
    LayoutDashboard, Bot, Users, Zap, Globe, Clock,
    Webhook, Radio, GitBranch, BookOpen, FileText,
    User, Hash, Server, ScrollText, Settings
  } from '@amanclaw/ui';
```

Replace the `pages` array icon strings with Lucide component references. Each nav item's `icon` field changes from a string like `'⊞'` to a component like `LayoutDashboard`.

In the template, replace `{page.icon}` text rendering with:
```svelte
<!-- Svelte 5: render component directly, NOT <svelte:component> (deprecated) -->
<page.icon size={16} />
```

- [ ] **Step 3: Verify desktop renders with new icons**

Run: `cd apps/desktop && pnpm dev`
Expected: Sidebar shows clean Lucide SVG icons instead of Unicode.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/lib/components/Sidebar.svelte
git commit -m "feat(desktop): replace Unicode icons with Lucide in sidebar"
```

---

### Task 11: Replace Dashboard Sidebar Icons

**Files:**
- Modify: `apps/dashboard/src/lib/components/Sidebar.svelte`

- [ ] **Step 1: Read the current Sidebar.svelte**

Read: `apps/dashboard/src/lib/components/Sidebar.svelte`

- [ ] **Step 2: Replace emoji icons with Lucide imports**

Same pattern as Task 10. Import Lucide icons from `@amanclaw/ui`, replace emoji strings in nav items, render with `<svelte:component>`.

Icon mapping for Dashboard pages:
- Dashboard → `LayoutDashboard`
- Users → `User`
- Skills → `Zap`
- Channels → `Hash`
- Communities → `Users`
- Content → `FileText`
- MCP Servers → `Server`
- Logs → `ScrollText`
- Settings → `Settings`

- [ ] **Step 3: Verify dashboard renders**

Run: `cd apps/dashboard && pnpm dev`
Expected: Sidebar shows Lucide icons instead of emojis.

- [ ] **Step 4: Commit**

```bash
git add apps/dashboard/src/lib/components/Sidebar.svelte
git commit -m "feat(dashboard): replace emoji icons with Lucide in sidebar"
```

---

### Task 12: Replace Dashboard Page Icons

**Files:**
- Modify: `apps/dashboard/src/lib/pages/Dashboard.svelte`
- Modify: any other dashboard pages using emoji icons in headers/content

- [ ] **Step 1: Read Dashboard.svelte and identify all emoji usage**

Read: `apps/dashboard/src/lib/pages/Dashboard.svelte`
Search for emoji usage in other page files.

- [ ] **Step 2: Replace emojis with Lucide icons**

In `Dashboard.svelte`, the StatCard components likely use emoji for their icon props. Replace with Lucide components. For example:
```svelte
<StatCard icon={Users} label="Communities" value={stats.communities} />
```

Update StatCard component to accept a Lucide component instead of an emoji string.

- [ ] **Step 3: Commit**

```bash
git add apps/dashboard/src/lib/pages/ apps/dashboard/src/lib/components/StatCard.svelte
git commit -m "feat(dashboard): replace emoji icons with Lucide across all pages"
```

---

### Task 13: Apply Base Typography to Both Apps

**Files:**
- Modify: `apps/desktop/src/app.html`
- Modify: `apps/dashboard/index.html`

- [ ] **Step 1: Set dark class and font-family on desktop app.html**

In `apps/desktop/src/app.html`, update the `<html>` tag:
```html
<html lang="en" class="dark">
```

The font-family is already applied via the Tailwind theme import.

- [ ] **Step 2: Set dark class on dashboard index.html**

In `apps/dashboard/index.html`, update:
```html
<html lang="en" class="dark">
```

- [ ] **Step 3: Verify fonts load**

Run either dev server. Check DevTools: body should show `Inter` as computed font-family. Check code elements show `JetBrains Mono`.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/app.html apps/dashboard/index.html
git commit -m "feat: set dark theme default and Inter font on both apps"
```

---

## Chunk 3: Core Components (Layer 3)

### Task 14: Verify Bits UI Available

`bits-ui` is already listed as a dependency of `@amanclaw/ui` (added in Task 2). No separate installation needed per-app. Verify with:
```bash
pnpm install && pnpm ls bits-ui --filter @amanclaw/ui
```

---

### Task 15: Button Component

**Files:**
- Create: `packages/ui/src/components/Button.svelte`

- [ ] **Step 1: Create Button.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'primary' | 'secondary' | 'ghost' | 'destructive' | 'accent';
    size?: 'default' | 'sm';
    children: Snippet;
  }

  let {
    variant = 'primary',
    size = 'default',
    children,
    class: className = '',
    ...rest
  }: Props = $props();

  const base = 'inline-flex items-center justify-center gap-1.5 font-medium rounded-lg transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed';

  const variants: Record<string, string> = {
    primary: 'bg-gradient-to-br from-primary-500 to-primary-700 text-white shadow-sm hover:from-primary-400 hover:to-primary-600',
    secondary: 'bg-[var(--color-elevated-60)] text-fg border border-border hover:bg-elevated',
    ghost: 'text-fg-secondary hover:text-fg hover:bg-[var(--color-elevated-50)]',
    destructive: 'bg-[var(--color-error-15)] text-error border border-[var(--color-error-20)] hover:bg-error/25',
    accent: 'bg-gradient-to-br from-accent-500 to-accent-700 text-[#1a0f00] font-semibold shadow-sm hover:from-accent-300 hover:to-accent-500',
  };

  const sizes: Record<string, string> = {
    default: 'px-4 py-2 text-[13px]',
    sm: 'px-3 py-1 text-xs',
  };
</script>

<button
  class="{base} {variants[variant]} {sizes[size]} {className}"
  {...rest}
>
  {@render children()}
</button>
```

- [ ] **Step 2: Export from index.ts**

Add to `packages/ui/src/index.ts`:
```typescript
export { default as Button } from './components/Button.svelte';
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Button.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Button component with 5 variants"
```

---

### Task 16: Input Component

**Files:**
- Create: `packages/ui/src/components/Input.svelte`

- [ ] **Step 1: Create Input.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    label?: string;
    error?: string;
    leadingIcon?: Snippet;
  }

  let {
    label,
    error,
    leadingIcon,
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <label class="text-body-sm font-medium {error ? 'text-error' : 'text-fg-secondary'}">
      {label}
    </label>
  {/if}
  <div class="relative flex items-center">
    {#if leadingIcon}
      <div class="absolute left-3 text-fg-muted">
        {@render leadingIcon()}
      </div>
    {/if}
    <input
      class="w-full bg-elevated border rounded-lg px-3.5 py-2.5 text-sm text-fg placeholder:text-fg-muted
             transition-all outline-none
             {error
               ? 'border-error focus:border-error focus:ring-[3px] focus:ring-error/10'
               : 'border-border focus:border-primary-500 focus:ring-[3px] focus:ring-primary-500/10'}
             {leadingIcon ? 'pl-10' : ''}
             {className}"
      {...rest}
    />
  </div>
  {#if error}
    <p class="text-xs text-error">{error}</p>
  {/if}
</div>
```

- [ ] **Step 2: Export from index.ts**

Add: `export { default as Input } from './components/Input.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Input.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Input component with label, error, and icon support"
```

---

### Task 17: Badge Component

**Files:**
- Create: `packages/ui/src/components/Badge.svelte`

- [ ] **Step 1: Create Badge.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'success' | 'warning' | 'error' | 'info' | 'accent' | 'muted' | 'telegram' | 'discord' | 'whatsapp' | 'slack';
    children: Snippet;
  }

  let { variant = 'muted', children }: Props = $props();

  const styles: Record<string, string> = {
    success: 'bg-success/15 text-green-400',
    warning: 'bg-warning/15 text-amber-400',
    error: 'bg-error/15 text-red-400',
    info: 'bg-info/15 text-blue-400',
    accent: 'bg-accent-500/15 text-accent-500',
    muted: 'bg-white/6 text-fg-secondary',
    telegram: 'bg-primary-500/15 text-primary-500',
    discord: 'bg-violet-500/15 text-violet-400',
    whatsapp: 'bg-success/15 text-green-400',
    slack: 'bg-warning/15 text-amber-400',
  };
</script>

<span class="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-medium {styles[variant]}">
  {@render children()}
</span>
```

- [ ] **Step 2: Export from index.ts**

Add: `export { default as Badge } from './components/Badge.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Badge.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Badge component with platform and semantic variants"
```

---

### Task 18: Card and StatCard Components

**Files:**
- Create: `packages/ui/src/components/Card.svelte`
- Create: `packages/ui/src/components/StatCard.svelte`

- [ ] **Step 1: Create Card.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    class?: string;
    children: Snippet;
  }

  let { class: className = '', children }: Props = $props();
</script>

<div class="bg-surface border border-border rounded-xl p-5 {className}">
  {@render children()}
</div>
```

- [ ] **Step 2: Create StatCard.svelte**

```svelte
<script lang="ts">
  import type { Component } from 'svelte';

  interface Props {
    label: string;
    value: string | number;
    icon: Component;
    iconColor?: string;
    trend?: string;
    trendPositive?: boolean;
  }

  let { label, value, icon: Icon, iconColor = 'text-primary-500 bg-primary-500/10', trend, trendPositive }: Props = $props();
</script>

<div class="bg-surface border border-border rounded-xl p-5">
  <div class="flex items-center justify-between mb-3">
    <span class="text-caption text-fg-muted">{label}</span>
    <div class="w-8 h-8 rounded-lg flex items-center justify-center {iconColor}">
      <Icon size={16} />
    </div>
  </div>
  <p class="text-[28px] font-bold text-fg">{value}</p>
  {#if trend}
    <p class="text-xs mt-1 {trendPositive ? 'text-success' : 'text-fg-muted'}">{trend}</p>
  {/if}
</div>
```

- [ ] **Step 3: Export both**

Add to `packages/ui/src/index.ts`:
```typescript
export { default as Card } from './components/Card.svelte';
export { default as StatCard } from './components/StatCard.svelte';
```

- [ ] **Step 4: Commit**

```bash
git add packages/ui/src/components/Card.svelte packages/ui/src/components/StatCard.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Card and StatCard components"
```

---

### Task 19: EmptyState Component

**Files:**
- Create: `packages/ui/src/components/EmptyState.svelte`

- [ ] **Step 1: Create EmptyState.svelte**

```svelte
<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  interface Props {
    icon: Component;
    title: string;
    description: string;
    action?: Snippet;
  }

  let { icon: Icon, title, description, action }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-12 px-6 text-center">
  <div class="w-12 h-12 rounded-xl bg-primary-500/10 flex items-center justify-center mb-4">
    <Icon size={24} class="text-primary-500" />
  </div>
  <h3 class="text-h3 text-fg mb-1.5">{title}</h3>
  <p class="text-body-sm text-fg-muted mb-4">{description}</p>
  {#if action}
    {@render action()}
  {/if}
</div>
```

- [ ] **Step 2: Export**

Add: `export { default as EmptyState } from './components/EmptyState.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/EmptyState.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add EmptyState component"
```

---

### Task 20: Skeleton Component

**Files:**
- Create: `packages/ui/src/components/Skeleton.svelte`

- [ ] **Step 1: Create Skeleton.svelte**

```svelte
<script lang="ts">
  interface Props {
    class?: string;
    width?: string;
    height?: string;
    rounded?: 'sm' | 'md' | 'lg' | 'full';
  }

  let { class: className = '', width, height = '12px', rounded = 'md' }: Props = $props();

  const radiusMap: Record<string, string> = {
    sm: 'rounded-sm',
    md: 'rounded',
    lg: 'rounded-lg',
    full: 'rounded-full',
  };
</script>

<div
  class="bg-elevated animate-pulse {radiusMap[rounded]} {className}"
  style="width: {width ?? '100%'}; height: {height};"
></div>
```

- [ ] **Step 2: Export**

Add: `export { default as Skeleton } from './components/Skeleton.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Skeleton.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Skeleton loading component"
```

---

### Task 21: Toggle, Select, Modal, Toast, Tooltip (Bits UI Wrappers)

**Files:**
- Create: `packages/ui/src/components/Toggle.svelte`
- Create: `packages/ui/src/components/Select.svelte`
- Create: `packages/ui/src/components/Modal.svelte`
- Create: `packages/ui/src/components/Toast.svelte`
- Create: `packages/ui/src/components/Tooltip.svelte`

- [ ] **Step 1: Create Toggle.svelte**

```svelte
<script lang="ts">
  import { Switch } from 'bits-ui';

  interface Props {
    checked?: boolean;
    onCheckedChange?: (checked: boolean) => void;
    label?: string;
    disabled?: boolean;
  }

  let { checked = $bindable(false), onCheckedChange, label, disabled = false }: Props = $props();
</script>

<div class="flex items-center gap-2.5">
  <Switch.Root
    {checked}
    onCheckedChange={(v) => { checked = v; onCheckedChange?.(v); }}
    {disabled}
    class="w-10 h-[22px] rounded-full transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed
           {checked ? 'bg-primary-500' : 'bg-border'}"
  >
    <Switch.Thumb
      class="block w-[18px] h-[18px] bg-white rounded-full shadow-sm transition-transform
             {checked ? 'translate-x-5' : 'translate-x-0.5'}"
    />
  </Switch.Root>
  {#if label}
    <span class="text-sm text-fg">{label}</span>
  {/if}
</div>
```

- [ ] **Step 2: Create Select.svelte**

```svelte
<script lang="ts">
  import { Select as BitsSelect } from 'bits-ui';
  import { ChevronDown } from 'lucide-svelte';

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    options: Option[];
    value?: string;
    onValueChange?: (value: string) => void;
    placeholder?: string;
    label?: string;
  }

  let { options, value = $bindable(''), onValueChange, placeholder = 'Select...', label }: Props = $props();

  const selected = $derived(options.find(o => o.value === value));
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <span class="text-body-sm font-medium text-fg-secondary">{label}</span>
  {/if}
  <BitsSelect.Root {value} onValueChange={(v) => { value = v; onValueChange?.(v); }}>
    <BitsSelect.Trigger
      class="flex items-center justify-between w-full bg-elevated border border-border rounded-lg px-3.5 py-2.5 text-sm text-fg
             transition-all outline-none focus:border-primary-500 focus:ring-[3px] focus:ring-primary-500/10 cursor-pointer"
    >
      <span class={selected ? '' : 'text-fg-muted'}>
        {selected?.label ?? placeholder}
      </span>
      <ChevronDown size={14} class="text-fg-muted" />
    </BitsSelect.Trigger>
    <BitsSelect.Content
      class="bg-elevated border border-border rounded-lg shadow-xl py-1 z-[var(--z-dropdown)]"
    >
      {#each options as option}
        <BitsSelect.Item
          value={option.value}
          textValue={option.label}
          class="px-3 py-2 text-sm text-fg hover:bg-primary-500/10 hover:text-primary-500 cursor-pointer transition-colors"
        >
          {option.label}
        </BitsSelect.Item>
      {/each}
    </BitsSelect.Content>
  </BitsSelect.Root>
</div>
```

- [ ] **Step 3: Create Modal.svelte**

```svelte
<script lang="ts">
  import { Dialog } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { X } from 'lucide-svelte';

  interface Props {
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
    title: string;
    description?: string;
    children: Snippet;
    footer?: Snippet;
  }

  let { open = $bindable(false), onOpenChange, title, description, children, footer }: Props = $props();
</script>

<Dialog.Root bind:open {onOpenChange}>
  <Dialog.Overlay class="fixed inset-0 bg-[var(--color-base-80)] backdrop-blur-sm z-[var(--z-modal-backdrop)]" />
  <Dialog.Content
      class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg
             bg-surface border border-border rounded-2xl shadow-2xl z-[var(--z-modal)]
             p-6"
    >
      <div class="flex items-start justify-between mb-4">
        <div>
          <Dialog.Title class="text-h2 text-fg">{title}</Dialog.Title>
          {#if description}
            <Dialog.Description class="text-body-sm text-fg-muted mt-1">{description}</Dialog.Description>
          {/if}
        </div>
        <Dialog.Close class="p-1 rounded-lg hover:bg-elevated text-fg-muted hover:text-fg transition-colors">
          <X size={18} />
        </Dialog.Close>
      </div>
      <div>
        {@render children()}
      </div>
      {#if footer}
        <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-border">
          {@render footer()}
        </div>
      {/if}
    </Dialog.Content>
</Dialog.Root>
```

- [ ] **Step 4: Create Tooltip.svelte**

```svelte
<script lang="ts">
  import { Tooltip as BitsTooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';

  interface Props {
    text: string;
    children: Snippet;
  }

  let { text, children }: Props = $props();
</script>

<BitsTooltip.Root openDelay={200}>
  <BitsTooltip.Trigger>
    {@render children()}
  </BitsTooltip.Trigger>
  <BitsTooltip.Content
    class="bg-elevated border border-border text-xs text-fg px-2.5 py-1.5 rounded-md shadow-lg z-[var(--z-tooltip)]"
  >
    {text}
  </BitsTooltip.Content>
</BitsTooltip.Root>
```

- [ ] **Step 5: Export all new components**

Add to `packages/ui/src/index.ts`:
```typescript
export { default as Toggle } from './components/Toggle.svelte';
export { default as Select } from './components/Select.svelte';
export { default as Modal } from './components/Modal.svelte';
export { default as Tooltip } from './components/Tooltip.svelte';
```

- [ ] **Step 6: Commit**

```bash
git add packages/ui/src/components/Toggle.svelte packages/ui/src/components/Select.svelte packages/ui/src/components/Modal.svelte packages/ui/src/components/Tooltip.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Toggle, Select, Modal, and Tooltip components with Bits UI"
```

---

### Task 21b: Toast Component

**Files:**
- Create: `packages/ui/src/components/Toast.svelte`

- [ ] **Step 1: Create Toast.svelte**

```svelte
<script lang="ts">
  import type { Component } from 'svelte';
  import { X, Check, AlertCircle, Info } from 'lucide-svelte';

  interface Props {
    variant?: 'success' | 'error' | 'warning' | 'info';
    title: string;
    description?: string;
    open?: boolean;
    onClose?: () => void;
    duration?: number;
  }

  let { variant = 'info', title, description, open = $bindable(true), onClose, duration = 5000 }: Props = $props();

  const icons: Record<string, Component> = { success: Check, error: AlertCircle, warning: AlertCircle, info: Info };
  const colors: Record<string, string> = {
    success: 'border-l-success text-success',
    error: 'border-l-error text-error',
    warning: 'border-l-warning text-warning',
    info: 'border-l-info text-info',
  };

  const Icon = $derived(icons[variant]);

  $effect(() => {
    if (open && duration > 0) {
      const timer = setTimeout(() => { open = false; onClose?.(); }, duration);
      return () => clearTimeout(timer);
    }
  });
</script>

{#if open}
  <div class="fixed bottom-4 right-4 z-[var(--z-toast)] bg-surface border border-border border-l-4 {colors[variant]} rounded-lg shadow-xl p-4 min-w-[320px] max-w-[420px] flex gap-3 items-start animate-in slide-in-from-right">
    <Icon size={18} />
    <div class="flex-1">
      <p class="text-sm font-medium text-fg">{title}</p>
      {#if description}
        <p class="text-xs text-fg-muted mt-1">{description}</p>
      {/if}
    </div>
    <button onclick={() => { open = false; onClose?.(); }} class="text-fg-muted hover:text-fg transition-colors">
      <X size={14} />
    </button>
  </div>
{/if}
```

- [ ] **Step 2: Export**

Add: `export { default as Toast } from './components/Toast.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Toast.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Toast notification component"
```

---

### Task 21c: Table Component

**Files:**
- Create: `packages/ui/src/components/Table.svelte`

- [ ] **Step 1: Create Table.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    columns: string[];
    children: Snippet;
  }

  let { columns, children }: Props = $props();
</script>

<div class="bg-surface border border-border rounded-xl overflow-hidden">
  <div class="grid gap-0" style="grid-template-columns: repeat({columns.length}, minmax(0, 1fr));">
    <!-- Header -->
    {#each columns as col}
      <div class="px-4 py-2.5 bg-[var(--color-elevated-50)] border-b border-border">
        <span class="text-caption text-fg-muted">{col}</span>
      </div>
    {/each}
  </div>
  <!-- Body rows provided via children snippet -->
  {@render children()}
</div>
```

Note: This is a minimal table shell. Each page will compose table rows directly using the design token classes (`px-4 py-3 border-b border-border text-sm text-fg` etc.) for flexibility. Complex tables (sorting, filtering) will use inline patterns rather than over-abstracting.

- [ ] **Step 2: Export**

Add: `export { default as Table } from './components/Table.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/components/Table.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Table component shell"
```

---

## Chunk 4: Layout Components (Layer 4)

### Task 22: Sidebar Layout Component

**Files:**
- Create: `packages/ui/src/layouts/Sidebar.svelte`

- [ ] **Step 1: Create Sidebar.svelte**

```svelte
<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ChevronRight, LogOut } from 'lucide-svelte';

  interface NavItem {
    id: string;
    label: string;
    icon: Component;
    badge?: string;
  }

  interface NavGroup {
    label: string;
    items: NavItem[];
  }

  interface Props {
    groups: NavGroup[];
    activePage: string;
    onNavigate: (id: string) => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
    userName?: string;
    userInitials?: string;
    onLogout?: () => void;
    headerSlot?: Snippet;
  }

  let {
    groups,
    activePage,
    onNavigate,
    collapsed = false,
    onToggleCollapse,
    userName,
    userInitials = 'U',
    onLogout,
    headerSlot,
  }: Props = $props();
</script>

<aside
  class="h-screen bg-surface border-r border-border flex flex-col transition-all z-[var(--z-sidebar)]
         {collapsed ? 'w-16' : 'w-60'}"
>
  <!-- Header -->
  <div class="px-3 pt-4 pb-5 flex items-center gap-2.5 {collapsed ? 'justify-center' : ''}">
    <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shrink-0">
      <span class="text-white text-xs font-bold">A</span>
    </div>
    {#if !collapsed}
      <div>
        <span class="text-sm font-semibold text-fg">AmanClaw</span>
        <p class="text-[10px] text-fg-muted">Community Bot</p>
      </div>
    {/if}
    {#if headerSlot}
      {@render headerSlot()}
    {/if}
  </div>

  <!-- Nav Groups -->
  <nav class="flex-1 overflow-y-auto px-2 space-y-4">
    {#each groups as group}
      {#if !collapsed}
        <p class="text-caption text-fg-muted px-2.5">{group.label}</p>
      {/if}
      <div class="space-y-0.5">
        {#each group.items as item}
          {@const active = activePage === item.id}
          <button
            onclick={() => onNavigate(item.id)}
            class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg transition-colors text-left
                   {active ? 'bg-primary-500/10 text-primary-500' : 'text-fg-secondary hover:bg-elevated/50 hover:text-fg'}
                   {collapsed ? 'justify-center' : ''}"
          >
            <item.icon size={16} class={active ? 'text-primary-500' : 'text-fg-muted'} />
            {#if !collapsed}
              <span class="text-body-sm font-medium">{item.label}</span>
              {#if item.badge}
                <span class="ml-auto text-[10px] font-semibold bg-accent-500/15 text-accent-500 px-1.5 py-0.5 rounded">
                  {item.badge}
                </span>
              {/if}
            {/if}
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  <!-- Collapse toggle -->
  {#if onToggleCollapse}
    <button
      onclick={onToggleCollapse}
      class="mx-2 mb-2 p-2 rounded-lg text-fg-muted hover:bg-elevated/50 hover:text-fg transition-colors
             {collapsed ? 'self-center' : 'self-end'}"
    >
      <ChevronRight size={16} class="transition-transform {collapsed ? '' : 'rotate-180'}" />
    </button>
  {/if}

  <!-- User Profile -->
  {#if userName || onLogout}
    <div class="px-3 py-3 border-t border-border flex items-center gap-2.5 {collapsed ? 'justify-center' : ''}">
      <div class="w-7 h-7 rounded-full bg-gradient-to-br from-accent-500 to-accent-700 flex items-center justify-center shrink-0">
        <span class="text-[11px] font-bold text-[#1a0f00]">{userInitials}</span>
      </div>
      {#if !collapsed}
        <span class="text-xs font-medium text-fg flex-1">{userName}</span>
        {#if onLogout}
          <button onclick={onLogout} class="text-fg-muted hover:text-fg transition-colors">
            <LogOut size={14} />
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</aside>
```

- [ ] **Step 2: Export**

Add to `packages/ui/src/index.ts`:
```typescript
export { default as Sidebar } from './layouts/Sidebar.svelte';
```

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/layouts/Sidebar.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add Sidebar layout component with collapse and nav groups"
```

---

### Task 23: TopBar Layout Component

**Files:**
- Create: `packages/ui/src/layouts/TopBar.svelte`

- [ ] **Step 1: Create TopBar.svelte**

```svelte
<script lang="ts">
  import { Search, Moon, Sun, ChevronRight } from 'lucide-svelte';
  import { theme } from '../stores/theme.js';

  interface Props {
    breadcrumbs?: { label: string; active?: boolean }[];
    onSearch?: () => void;
    class?: string;
  }

  let { breadcrumbs = [], onSearch, class: className = '' }: Props = $props();
</script>

<header class="sticky top-0 h-12 px-6 flex items-center gap-4 border-b border-border bg-base/80 backdrop-blur-sm z-[var(--z-sticky)] {className}">
  <!-- Breadcrumbs -->
  <div class="flex items-center gap-1.5 flex-1">
    {#each breadcrumbs as crumb, i}
      {#if i > 0}
        <ChevronRight size={12} class="text-fg-muted" />
      {/if}
      <span class="text-body-sm {crumb.active ? 'text-fg font-medium' : 'text-fg-muted'}">
        {crumb.label}
      </span>
    {/each}
  </div>

  <!-- Search -->
  {#if onSearch}
    <button
      onclick={onSearch}
      class="flex items-center gap-1.5 bg-elevated border border-border rounded-lg px-3 py-1.5 w-60 text-left hover:border-primary-500/30 transition-colors"
    >
      <Search size={14} class="text-fg-muted" />
      <span class="text-body-sm text-fg-muted flex-1">Search...</span>
      <kbd class="text-[11px] text-fg-muted bg-base px-1.5 py-0.5 rounded">⌘K</kbd>
    </button>
  {/if}

  <!-- Theme Toggle -->
  <button
    onclick={() => theme.toggle()}
    class="w-8 h-8 rounded-lg bg-elevated/50 flex items-center justify-center text-fg-muted hover:text-fg transition-colors"
  >
    {#if $theme === 'dark'}
      <Moon size={16} />
    {:else}
      <Sun size={16} />
    {/if}
  </button>
</header>
```

- [ ] **Step 2: Export**

Add: `export { default as TopBar } from './layouts/TopBar.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/layouts/TopBar.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add TopBar layout with breadcrumbs, search, and theme toggle"
```

---

### Task 24: PageHeader Layout Component

**Files:**
- Create: `packages/ui/src/layouts/PageHeader.svelte`

- [ ] **Step 1: Create PageHeader.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: string;
    action?: Snippet;
  }

  let { title, subtitle, action }: Props = $props();
</script>

<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-h1 text-fg">{title}</h1>
    {#if subtitle}
      <p class="text-body-sm text-fg-muted mt-1">{subtitle}</p>
    {/if}
  </div>
  {#if action}
    {@render action()}
  {/if}
</div>
```

- [ ] **Step 2: Export**

Add: `export { default as PageHeader } from './layouts/PageHeader.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/layouts/PageHeader.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add PageHeader layout component"
```

---

### Task 25: BottomNav Layout Component (Mobile)

**Files:**
- Create: `packages/ui/src/layouts/BottomNav.svelte`

- [ ] **Step 1: Create BottomNav.svelte**

```svelte
<script lang="ts">
  import type { Component } from 'svelte';
  import { MoreHorizontal } from 'lucide-svelte';

  interface NavItem {
    id: string;
    label: string;
    icon: Component;
  }

  interface Props {
    items: NavItem[];
    activePage: string;
    onNavigate: (id: string) => void;
    moreItems?: NavItem[];
    onMore?: () => void;
  }

  let { items, activePage, onNavigate, moreItems, onMore }: Props = $props();

  let showMore = $state(false);
</script>

<!-- Bottom Nav - visible only on mobile -->
<nav class="md:hidden fixed bottom-0 left-0 right-0 bg-surface border-t border-border px-3 py-2 flex justify-around z-[var(--z-sticky)]">
  {#each items as item}
    {@const active = activePage === item.id}
    <button
      onclick={() => onNavigate(item.id)}
      class="flex flex-col items-center gap-0.5 px-2 py-1 {active ? 'text-primary-500' : 'text-fg-muted'}"
    >
      <item.icon size={20} />
      <span class="text-[10px] font-medium">{item.label}</span>
    </button>
  {/each}
  {#if moreItems && moreItems.length > 0}
    <button
      onclick={() => { showMore = !showMore; onMore?.(); }}
      class="flex flex-col items-center gap-0.5 px-2 py-1 text-fg-muted"
    >
      <MoreHorizontal size={20} />
      <span class="text-[10px] font-medium">More</span>
    </button>
  {/if}
</nav>

<!-- More sheet -->
{#if showMore && moreItems}
  <div class="md:hidden fixed inset-0 bg-base/80 z-[var(--z-modal-backdrop)]" onclick={() => showMore = false}></div>
  <div class="md:hidden fixed bottom-0 left-0 right-0 bg-surface border-t border-border rounded-t-2xl p-4 z-[var(--z-modal)]">
    <div class="w-10 h-1 bg-border rounded-full mx-auto mb-4"></div>
    <div class="grid grid-cols-4 gap-3">
      {#each moreItems as item}
        <button
          onclick={() => { onNavigate(item.id); showMore = false; }}
          class="flex flex-col items-center gap-1.5 p-3 rounded-xl hover:bg-elevated/50 text-fg-secondary"
        >
          <item.icon size={20} />
          <span class="text-[11px] font-medium">{item.label}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}
```

- [ ] **Step 2: Export**

Add: `export { default as BottomNav } from './layouts/BottomNav.svelte';`

- [ ] **Step 3: Commit**

```bash
git add packages/ui/src/layouts/BottomNav.svelte packages/ui/src/index.ts
git commit -m "feat(ui): add BottomNav mobile layout with 'More' sheet"
```

---

## Chunk 5: Page Polish — Desktop App (Layer 5a)

### Task 26: Desktop — Integrate New Layout Shell

**Files:**
- Modify: `apps/desktop/src/routes/+layout.svelte`
- Modify: `apps/desktop/src/lib/components/Sidebar.svelte`

- [ ] **Step 1: Read current +layout.svelte and Sidebar.svelte**

Read both files to understand current structure.

- [ ] **Step 2: Replace the desktop layout with the new shell**

Update `+layout.svelte` to use the shared `Sidebar`, `TopBar`, and `BottomNav` from `@amanclaw/ui`. The layout structure becomes:

```svelte
<script lang="ts">
  import { Sidebar, TopBar, BottomNav } from '@amanclaw/ui';
  import {
    LayoutDashboard, Bot, Users, Zap, Globe, Clock,
    Webhook, Radio, GitBranch, BookOpen, FileText,
    User, Hash, Server, ScrollText, Settings
  } from '@amanclaw/ui';

  // ... define nav groups, active page state, navigation handler
</script>

<div class="flex h-screen bg-base">
  <!-- Sidebar (hidden on mobile) -->
  <div class="hidden md:block">
    <Sidebar {groups} {activePage} {onNavigate} {collapsed} {onToggleCollapse} userName="Admin" userInitials="AM" />
  </div>

  <!-- Main Content -->
  <div class="flex-1 flex flex-col overflow-hidden">
    <TopBar breadcrumbs={[...]} onSearch={...} />
    <main class="flex-1 overflow-y-auto p-6">
      {@render children()}
    </main>
  </div>

  <!-- Mobile Bottom Nav -->
  <BottomNav items={mobileItems} moreItems={moreItems} {activePage} {onNavigate} />
</div>
```

Define navigation groups matching the spec:
- **Main:** Dashboard, Agents, Communities, Skills, Marketplace
- **System:** Cron Jobs, Webhooks, Gateway, Sub-Agents, Knowledge Bases, Content, Users, Channels, MCP Servers, Logs, Settings

- [ ] **Step 3: Update the old Sidebar.svelte to re-export or remove**

Either delete the old `Sidebar.svelte` and import from `@amanclaw/ui` directly in the layout, or keep it as a thin wrapper that passes desktop-specific nav config.

- [ ] **Step 4: Verify the desktop shell renders**

Run: `cd apps/desktop && pnpm dev`
Expected: New sidebar with teal active state, Lucide icons, grouped nav, topbar with breadcrumbs, dark background.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/routes/+layout.svelte apps/desktop/src/lib/components/Sidebar.svelte
git commit -m "feat(desktop): integrate new layout shell with shared Sidebar and TopBar"
```

---

### Task 27: Desktop — Polish Each Page (Pattern)

This task covers all 16 existing pages + 1 new Dashboard page. Each page follows the same pattern:

**For each page file in `apps/desktop/src/lib/pages/`:**

- [ ] **Step 1: Read the current page**
- [ ] **Step 2: Apply the design system**

Pattern for every page:
1. Read the existing page file first. Preserve all data-fetching logic and Tauri IPC commands.
2. Import `PageHeader`, `Button`, `Card`, `Badge`, `Input`, `EmptyState`, `Skeleton`, `Table` etc. from `@amanclaw/ui`
3. Import relevant Lucide icons from `@amanclaw/ui`
4. Replace all hardcoded Tailwind colors with design token classes:
   - `bg-gray-50` → `bg-surface`
   - `bg-gray-900` → `bg-primary-500` or `bg-elevated`
   - `text-gray-700` → `text-fg-secondary`
   - `text-gray-500` → `text-fg-muted`
   - `border-gray-200` → `border-border`
   - `bg-white` → `bg-surface` (or `bg-base`)
   - `dark:bg-gray-800` → (remove, tokens handle dark/light)
5. Replace tiny text sizes:
   - `text-[10px]`, `text-[11px]` → `text-body-sm` (13px) minimum
   - `text-xs` for labels → `text-caption`
6. Add `<PageHeader>` at the top of each page
7. Replace bare `<button>` elements with `<Button>` component
8. Replace bare `<input>` elements with `<Input>` component
9. Replace inline status badges with `<Badge>` component
10. Add empty states where data lists can be empty
11. Add skeleton loaders for async data
12. For tables: use `<Table>` shell or apply token classes directly to existing `<table>` markup

- [ ] **Step 3: Commit each page individually**

```bash
git commit -m "feat(desktop): polish {PageName} page with design system"
```

**Page order (do each one as a sub-task):**
1. Create `Dashboard.svelte` (NEW) — stat cards (StatCard), recent activity list
2. `Agents.svelte` — SOUL.md textarea, routing rules table
3. `Communities.svelte` — table with avatars, badges, CRUD
4. `Skills.svelte` — toggle cards grid
5. `Marketplace.svelte` — discovery grid
6. `CronJobs.svelte` — schedule table, history
7. `Webhooks.svelte` — endpoint config table
8. `Gateway.svelte` — WebSocket config, event stream
9. `SubAgents.svelte` — active agent list
10. `KnowledgeBases.svelte` — RAG config
11. `Content.svelte` — read-only tabs
12. `Users.svelte` — user table
13. `Channels.svelte` — channel cards
14. `McpServers.svelte` — server config table
15. `Logs.svelte` — log viewer with filters
16. `Settings.svelte` — config forms
17. `Wizard.svelte` — onboarding (stub, future)

---

## Chunk 6: Page Polish — Dashboard + HTML Apps (Layer 5b)

### Task 28: Dashboard — Migrate to Svelte 5 + Integrate New Layout Shell

**Files:**
- Modify: `apps/dashboard/src/App.svelte`
- Modify: `apps/dashboard/src/lib/components/Sidebar.svelte`
- Modify: `apps/dashboard/src/lib/components/MobileNav.svelte`
- Modify: All files in `apps/dashboard/src/lib/components/` and `apps/dashboard/src/lib/pages/`

**IMPORTANT:** The dashboard currently uses Svelte 4 syntax (`export let`, `on:click`, `$:` reactive statements). Before integrating shared components, migrate all files to Svelte 5 runes:
- `export let foo` → `let { foo } = $props()`
- `$: derived = ...` → `const derived = $derived(...)`
- `let x = 0` (reactive) → `let x = $state(0)`
- `on:click={handler}` → `onclick={handler}`
- `<slot />` → `{@render children()}` with `Snippet` prop

- [ ] **Step 1: Read all dashboard component and page files**
- [ ] **Step 2: Migrate each file from Svelte 4 to Svelte 5 syntax**

Start with components (Sidebar, MobileNav, StatCard, StatusBadge, ChannelCard, QrCodeDisplay), then pages. Commit after each file or batch of related files.

- [ ] **Step 3: Replace layout with shared components**

Same pattern as Task 26 but adapted for the dashboard's hash-based routing. Replace the existing sidebar and mobile nav with `@amanclaw/ui` `Sidebar` and `BottomNav`.

- [ ] **Step 4: Verify dashboard renders**
- [ ] **Step 5: Commit**

```bash
git commit -m "feat(dashboard): integrate new layout shell with shared components"
```

---

### Task 29: Dashboard — Polish Each Page

**Files:** All files in `apps/dashboard/src/lib/pages/`

Same pattern as Task 27. Apply to all 10 dashboard pages:
1. `Login.svelte` — brand-styled login with teal/gold gradient
2. `Dashboard.svelte` — stat cards, activity
3. `Users.svelte` — user table
4. `Channels.svelte` — channel cards with QR
5. `Communities.svelte` — table with badges
6. `Content.svelte` — read-only tabs
7. `Skills.svelte` — toggle cards
8. `McpServers.svelte` — server config
9. `Logs.svelte` — log viewer
10. `Settings.svelte` — config forms

Each page: import shared components, replace colors/typography, add empty states, commit individually.

---

### Task 30: Cloud Chat — Apply Design Tokens

**Files:**
- Modify: `apps/cloud/src/chat.html`

- [ ] **Step 1: Read current chat.html**

Read: `apps/cloud/src/chat.html`

- [ ] **Step 2: Replace inline CSS variables with design tokens**

Replace the existing CSS variables in the `<style>` block with imports from `amanclaw-tokens.css` or inline the token values. Key changes:
- Background: `--color-base` (#0a0f1a)
- Chat surface: `--color-surface` (#0f172a)
- User message bg: `--color-primary-500` (#14b8a6)
- Text colors: use `--text-primary`, `--text-secondary`, `--text-muted`
- Font: `var(--font-sans)`
- Code blocks: `var(--font-mono)`
- Add a skeleton loader for initial load state

- [ ] **Step 3: Verify chat renders correctly**
- [ ] **Step 4: Commit**

```bash
git commit -m "feat(cloud): apply design tokens to chat embed"
```

---

### Task 31: Landing Page — Rebrand

**Files:**
- Modify: `products/communitybot/index.html`

- [ ] **Step 1: Read current index.html**
- [ ] **Step 2: Apply design system branding**

Key changes:
- Background: dark navy (`--color-base`)
- Accent colors: teal (`--color-primary-500`) and gold (`--color-accent-500`)
- Font: Inter via `--font-sans`
- Add inline SVG Lucide icons to feature cards (instead of no icons)
- Align typography to the system scale
- Link `amanclaw-tokens.css` or inline the token values

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(landing): rebrand with design system colors and typography"
```

---

### Task 32: CLI Playground — Apply Tokens

**Files:**
- Modify: `apps/cli/static/playground.html`

- [ ] **Step 1: Read current playground.html**
- [ ] **Step 2: Replace inline styles with design tokens**

Key changes:
- Replace hardcoded colors with CSS variables
- Fix font sizes (no 11px, minimum 12px)
- Apply `--font-sans` and `--font-mono`
- Use `--color-base`, `--color-surface`, `--color-elevated` for surfaces

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(cli): apply design tokens to playground"
```

---

### Task 33: Final Verification and Cleanup

- [ ] **Step 1: Run desktop dev server and visually verify all pages**

Run: `cd apps/desktop && pnpm dev`
Check: Each page uses design system colors, Lucide icons, readable typography, consistent components.

- [ ] **Step 2: Run dashboard dev server and verify**

Run: `cd apps/dashboard && pnpm dev`
Check: Same design system, login page branded, all pages consistent.

- [ ] **Step 3: Open chat.html in browser and verify**

Check: Dark theme with teal/gold, readable fonts, skeleton loader.

- [ ] **Step 4: Open landing page and verify**

Check: Branded with design system, feature cards have icons, typography aligned.

- [ ] **Step 5: Open playground.html and verify**

Check: Design tokens applied, no tiny text, consistent colors.

- [ ] **Step 6: Add .superpowers/ to .gitignore if not already there**

```bash
echo ".superpowers/" >> .gitignore
git add .gitignore
git commit -m "chore: add .superpowers/ to gitignore"
```

- [ ] **Step 7: Final commit — tag the design system release**

```bash
git tag v0.1.0-design-system
```
