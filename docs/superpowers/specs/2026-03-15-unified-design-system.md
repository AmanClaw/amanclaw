# AmanClaw Unified Design System

Comprehensive UI/UX redesign across all 5 app surfaces with a shared design system.

## Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Visual direction | Dark + Islamic Hybrid | Distinctive brand identity; teal + gold on dark navy. Modern dev-tool feel with cultural resonance for Malaysian Muslim communities. |
| Icon system | Lucide Icons | Best Svelte integration (lucide-svelte), 1500+ icons, tree-shakeable, MIT license. Colors applied via CSS. |
| Build strategy | Layered Build-Out | 5 progressive layers, each shippable. Design tokens first, then icons/type, then components, then layouts, then page-by-page polish. |
| Component primitives | Bits UI | Headless, accessible Svelte 5 primitives. Style with design tokens for full brand control. |
| Fonts | Inter (UI) + JetBrains Mono (code) | Clean geometric sans-serif + developer-friendly monospace. Both free. |

## Scope

All 5 UI surfaces:

| App | Location | Pages | Priority |
|-----|----------|-------|----------|
| Desktop App | `apps/desktop/` | 16 + Dashboard (new) | Highest — primary admin UI |
| Web Dashboard | `apps/dashboard/` | 10 | High — shares components with Desktop |
| Cloud Chat Embed | `apps/cloud/src/chat.html` | 1 | Medium — already decent, needs token alignment |
| Landing Page | `products/communitybot/index.html` | 1 | Medium — needs brand alignment |
| CLI Playground | `apps/cli/static/playground.html` | 1 | Lower — dev tool |

## Layer 1: Design Tokens + Theme

CSS custom properties for the entire color system, applied via Tailwind CSS 4 config. All apps consume the same tokens.

### Color System

**Dark Mode (Default)**

| Token | Value | Usage |
|-------|-------|-------|
| `--color-base` | `#0a0f1a` | Page background |
| `--color-surface` | `#0f172a` | Card/sidebar background |
| `--color-elevated` | `#1e293b` | Input background, hover states |
| `--color-border` | `#334155` | Borders, dividers |

**Primary — Teal**

| Token | Value | Usage |
|-------|-------|-------|
| `--color-primary-900` | `#0d3d38` | Dark tint |
| `--color-primary-700` | `#115e56` | Hover state |
| `--color-primary-500` | `#14b8a6` | Primary buttons, active nav, links |
| `--color-primary-300` | `#5eead4` | Light tint |
| `--color-primary-100` | `#ccfbf1` | Subtle backgrounds |

**Accent — Gold**

| Token | Value | Usage |
|-------|-------|-------|
| `--color-accent-900` | `#78350f` | Dark tint |
| `--color-accent-700` | `#a16207` | Hover state |
| `--color-accent-500` | `#d4a574` | Badges, highlights, premium features |
| `--color-accent-300` | `#e8c9a0` | Light tint |
| `--color-accent-100` | `#fef3e2` | Subtle backgrounds |

**Semantic**

| Token | Value | Usage |
|-------|-------|-------|
| `--color-success` | `#22c55e` | Active, online, positive |
| `--color-error` | `#ef4444` | Error, destructive, offline |
| `--color-warning` | `#f59e0b` | Warning, degraded |
| `--color-info` | `#3b82f6` | Informational |

**Text on Dark**

| Token | Value | Usage |
|-------|-------|-------|
| `--text-primary` | `#f1f5f9` | Headings, primary content |
| `--text-secondary` | `#94a3b8` | Body text, descriptions |
| `--text-muted` | `#64748b` | Labels, timestamps, placeholders |
| `--text-accent` | `#14b8a6` | Active nav, links |

**Light Mode**

| Token | Value | Usage |
|-------|-------|-------|
| `--color-base` | `#ffffff` | Page background |
| `--color-surface` | `#f8fafc` | Card background |
| `--color-elevated` | `#f1f5f9` | Input background |
| `--color-border` | `#e2e8f0` | Borders |
| `--text-primary` | `#0f172a` | Headings |
| `--text-secondary` | `#475569` | Body text |
| `--text-muted` | `#64748b` | Labels (adjusted for WCAG AA on white: 4.7:1) |
| `--text-accent` | `#0d9488` | Active nav, links |

Primary, accent, and semantic color scales remain unchanged between modes — they have sufficient contrast on both dark and light surfaces. The exception: `--text-accent` shifts from `#14b8a6` (dark) to `#0d9488` (light) for better contrast on white.

**Theme Switching Mechanism:**
- CSS class toggle on `<html>`: `class="dark"` (default) or `class="light"`
- User preference stored in `localStorage` key `amanclaw-theme`
- On load: check localStorage → fall back to `prefers-color-scheme` → fall back to dark
- Tokens defined in CSS using `html.dark { ... }` and `html.light { ... }` selectors
- Toggle button in TopBar switches class and persists to localStorage

**Accessibility Target:** WCAG 2.1 AA (4.5:1 for normal text, 3:1 for large text). `--text-muted` on `--color-elevated` in dark mode (4.64:1) passes AA. All other pairings exceed this threshold.

**Usage Rules:**
- Teal = primary actions, navigation active state, links, CTAs
- Gold = accents, badges, highlights, premium features, "New" tags
- Gold is never used for primary CTAs
- Semantic colors for status indication only

### Spacing Scale

Consistent spacing used across all components and layouts:

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Tight gaps (icon-text) |
| `--space-2` | 8px | Small gaps (badge padding, inline spacing) |
| `--space-3` | 12px | Medium gaps (card grid, form field spacing) |
| `--space-4` | 16px | Standard padding (card content, table cells) |
| `--space-5` | 20px | Card padding, section spacing |
| `--space-6` | 24px | Page padding, between sections |
| `--space-8` | 32px | Large section gaps |
| `--space-12` | 48px | Page-level vertical rhythm |

### Transition Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--transition-fast` | `150ms ease` | Hover states, toggles, icon color |
| `--transition-normal` | `250ms ease` | Sidebar collapse, page transitions |
| `--transition-slow` | `400ms ease` | Modal open/close, toast entrance |

### Z-Index Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--z-base` | 0 | Page content |
| `--z-sidebar` | 10 | Sidebar |
| `--z-sticky` | 20 | TopBar (sticky) |
| `--z-dropdown` | 30 | Select menus, popovers |
| `--z-modal-backdrop` | 40 | Modal overlay |
| `--z-modal` | 50 | Modal content |
| `--z-toast` | 60 | Toast notifications |
| `--z-tooltip` | 70 | Tooltips (always on top) |

### Tailwind CSS 4 Integration

Tailwind CSS 4 uses CSS-based configuration with `@theme` instead of `tailwind.config.js`. Tokens are defined in a shared CSS file:

```css
/* packages/ui/src/tokens/theme.css */
@import "tailwindcss";

@theme {
  --color-base: var(--color-base);
  --color-surface: var(--color-surface);
  --color-elevated: var(--color-elevated);
  --color-border: var(--color-border);
  --color-primary-*: var(--color-primary-*);
  --color-accent-*: var(--color-accent-*);
  /* ... maps all tokens to Tailwind utilities */
}
```

Both `apps/desktop/src/app.css` and `apps/dashboard/src/app.css` import this shared theme file:
```css
@import "@amanclaw/ui/tokens/theme.css";
```

This enables Tailwind utilities like `bg-surface`, `text-primary`, `border-border` across both apps.

## Layer 2: Icon System + Typography

### Icons

Replace all Unicode symbols and emoji with Lucide Icons.

**Current state (broken):**
- Desktop sidebar: `⊞ ⚡ ◈ ⏱` (Unicode)
- Dashboard sidebar: `📊 👥 ⚡ 🏘️` (emoji)
- Other apps: no icons

**Target state:**
- All apps use `lucide-svelte` (Desktop + Dashboard) or inline SVG (HTML apps)
- Icon size: 16px in nav, 20px in mobile nav, 24px in empty states
- Icon color: inherits from parent text color via `currentColor`

**Icon mapping (sidebar):**

| Page | Icon |
|------|------|
| Dashboard | `LayoutDashboard` |
| Agents | `Bot` |
| Communities | `Users` |
| Skills | `Zap` |
| Marketplace | `Globe` |
| Cron Jobs | `Clock` |
| Webhooks | `Webhook` |
| Gateway | `Radio` |
| Sub-Agents | `GitBranch` |
| Knowledge Bases | `BookOpen` |
| Content | `FileText` |
| Users | `User` |
| Channels | `Hash` |
| MCP Servers | `Server` |
| Logs | `ScrollText` |
| Settings | `Settings` |

### Typography

**Font stack:**
- UI: `Inter, system-ui, -apple-system, sans-serif`
- Code: `'JetBrains Mono', ui-monospace, monospace`

**Type scale:**

| Name | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| `display` | 30px | 700 | 1.2 | Page titles (Dashboard, Settings) |
| `h1` | 24px | 600 | 1.3 | Section headings |
| `h2` | 20px | 600 | 1.3 | Card headings |
| `h3` | 16px | 600 | 1.4 | Sub headings |
| `body` | 14px | 400 | 1.5 | Paragraphs, descriptions |
| `body-sm` | 13px | 400 | 1.5 | Table cells, form labels |
| `caption` | 12px | 500 | 1.4 | Section labels (uppercase, tracking) |
| `code` | 13px | 400 | 1.6 | Code, API keys, IDs (mono) |

**Hard rule:** No text below 12px anywhere in the system. The current 10-11px text throughout Desktop and Dashboard must be eliminated.

## Layer 3: Core Components

Built on Bits UI primitives, styled with design tokens. Shared between Desktop and Dashboard via a shared component directory or package.

### Button

5 variants, 2 sizes:

| Variant | Background | Text | Border | Usage |
|---------|-----------|------|--------|-------|
| Primary | `linear-gradient(135deg, primary-500, primary-700)` | white | none | Main CTAs: "Add Community", "Save" |
| Secondary | `elevated` with opacity | `text-primary` | `border` | Secondary actions: "Cancel", "Edit" |
| Ghost | transparent | `text-secondary` | none | Tertiary: "View all", inline actions |
| Destructive | `error/15%` | `error` | `error/20%` | Delete, remove, disconnect |
| Accent | `linear-gradient(135deg, accent-500, accent-700)` | dark | none | Premium features, special actions |

Sizes: default (padding 9px 18px, 13px text), small (5px 12px, 12px text). Supports icon-only and icon+text.

### Input

States: default, focused (teal ring), error (red ring + message), disabled.
Background: `elevated`. Border: `border`. Focus border: `primary-500` with `0 0 0 3px primary-500/10%` ring.
Supports: leading icon, trailing icon, prefix text.

### Select

Bits UI `Select` primitive. Same visual treatment as Input. Chevron-down indicator.

### Toggle

Bits UI `Switch` primitive. Track: `border` off / `primary-500` on. Thumb: white with shadow.

### Card

Background: `surface`. Border: `border`. Border-radius: 12px. Padding: 20px.

### StatCard

Card with: label (caption), icon in colored container (32px, rounded-lg, color/10% bg), value (28px bold), optional trend line.

### Badge

Pill shape. 12px text, 500 weight. Background: `color/15%`. Text: lighter shade. Variants: success, warning, error, info, accent, muted, and platform-specific (Telegram=teal, Discord=violet, WhatsApp=green, Slack=amber).

### Table

Header: `caption` style, uppercase, `text-muted`. Rows: 12-16px padding, `border` bottom. Hover: subtle `elevated` background. Supports avatar + name + subtitle in first column.

### EmptyState

Centered layout: icon (48px in colored container), title (h2), description (body, muted), primary action button.

### Skeleton

Animated placeholder blocks matching content shape. Background: `elevated`. Pulse animation.

### Toast

Bits UI `Toast` primitive. Positioned bottom-right. Variants match semantic colors. Auto-dismiss with progress bar.

### Modal / Dialog

Bits UI `Dialog` primitive. Backdrop: `base/80%` with blur. Content: `surface` background, `border`, 16px radius. Header + body + footer layout.

### Tooltip

Bits UI `Tooltip` primitive. Background: `elevated`. Border: `border`. 12px text. Arrow indicator. 200ms delay.

## Layer 4: Layout Components

### Sidebar

- Width: 240px (expanded), 64px (collapsed, icons only)
- Background: `surface`
- Grouped navigation with `caption`-style section labels (Main, System)
- Active item: `primary-500/10%` background, teal text + icon
- Inactive: `text-secondary` text, `text-muted` icon
- "New" badge on items using gold accent
- User profile at bottom: avatar, name, logout icon
- Collapse toggle button
- Desktop app: includes Tauri drag region at top
- Dashboard: identical minus Tauri-specific features

### TopBar

- Height: ~48px. Sticky.
- Left: breadcrumb navigation (group > page)
- Center/right: global search input (⌘K shortcut)
- Right: theme toggle (sun/moon icon)
- Desktop app: Tauri drag region + window controls integrated
- Mobile: simplified to logo + page title + search icon

### PageHeader

- Title (`h1`), optional subtitle (`body`, muted)
- Right-aligned primary action button
- Optional tab bar for sub-navigation (e.g., Skills > Built-in / Plugins / Marketplace)
- Consistent on every page

### BottomNav (Mobile)

- Visible only on `<768px`
- 5 destinations: Home, Communities, Skills, Settings, More
- "More" opens a slide-up sheet with remaining nav items (Logs, Webhooks, MCP Servers, Cron Jobs, etc.)
- Active: teal icon + label. Inactive: muted.
- Replaces sidebar entirely on mobile

### Responsive Breakpoints

| Breakpoint | Layout |
|-----------|--------|
| `≥1024px` | Full sidebar (240px) + content |
| `768-1023px` | Collapsed sidebar (64px) + content |
| `<768px` | No sidebar, bottom nav, simplified top bar |

## Layer 5: Page-by-Page Polish

Apply the component library and layout system to every page. Order:

### Desktop App (16 existing + 1 new)
1. Dashboard (NEW) — stat cards, activity feed
2. Agents — SOUL.md editor, routing rules table
3. Communities — CRUD table, detail view
4. Skills — toggle cards, enable/disable
5. Marketplace — discovery grid, install flow
6. Cron Jobs — schedule table, history
7. Webhooks — endpoint config, history log
8. Gateway — WebSocket config, live event stream
9. Sub-Agents — active agent monitor, cancel
10. Knowledge Bases — RAG config, embedding status
11. Content — read-only: Doa, Zakat, Khutbah
12. Users — user table
13. Channels — channel cards with QR
14. MCP Servers — server config
15. Logs — log viewer with filters
16. Settings — config forms
17. Wizard — onboarding flow (future)

### Dashboard App (10 pages)
1. Login — brand-styled authentication page
2. Dashboard — stat cards, activity feed
3. Users — user table
4. Channels — channel cards with QR
5. Communities — CRUD table
6. Content — read-only: Doa, Zakat, Khutbah
7. Skills — toggle cards
8. MCP Servers — server config
9. Logs — log viewer
10. Settings — config forms

### Cloud Chat Embed
Align colors to design tokens. Keep existing layout. Add loading skeleton. Improve code block styling. The embed is responsive and fills its container width (min 320px, max 100%). Height is set by the host page via iframe or container. Mobile viewports get simplified message bubbles with no side padding reduction.

### Landing Page
Rebrand with dark navy + teal/gold palette. Add Lucide icons to feature cards. Align typography to system scale.

### CLI Playground
Apply dark theme tokens. Replace inline styles with consistent classes. Fix font sizes.

## Architecture: Shared Component Package

### Package Structure

```
packages/
  ui/
    src/
      tokens/
        theme.css      # @theme definitions for Tailwind CSS 4
        colors.css     # CSS custom property definitions (dark/light)
        typography.css # Font imports, type scale classes
      components/      # Svelte 5 components (Button, Input, Card, etc.)
      layouts/         # Sidebar, TopBar, PageHeader, BottomNav
      icons.ts         # Re-export Lucide icons used across apps
      index.ts         # Main entry: re-exports all components
    package.json
```

### Bootstrap Setup

1. **Create `packages/ui/`** with `package.json`:
   - Name: `@amanclaw/ui`
   - Type: `module`
   - Exports: `./src/index.ts` (components), `./tokens/theme.css` (CSS)
   - Peer dependencies: `svelte@^5`, `bits-ui`, `lucide-svelte`, `tailwindcss@^4`

2. **Configure pnpm workspace** (root `pnpm-workspace.yaml`):
   ```yaml
   packages:
     - "apps/*"
     - "packages/*"
   ```

3. **Wire consumers** — add `@amanclaw/ui` as workspace dependency in both `apps/desktop/package.json` and `apps/dashboard/package.json`:
   ```json
   "@amanclaw/ui": "workspace:*"
   ```

4. **Import in apps**:
   - CSS: `@import "@amanclaw/ui/tokens/theme.css";` in app.css
   - Components: `import { Button, Input, Card } from "@amanclaw/ui";`

5. **HTML-only apps** (chat, landing, playground) get a compiled `amanclaw-tokens.css` file built from the token definitions, containing only CSS custom properties and utility classes. No Svelte dependency needed.

## Non-Goals

- Custom Islamic geometric icon set (too much effort for marginal gain)
- Animation library or complex page transitions (keep it simple)
- Redesigning the Rust backend APIs
- Mobile native app
- Internationalization (i18n) in this phase
