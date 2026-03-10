# Web Management Dashboard — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an embedded Svelte + Tailwind web dashboard served from the AmanClaw Rust binary at `/admin`, with password login and full bot management UI.

**Architecture:** Svelte SPA builds to static files in `dashboard/dist/`. Rust embeds them via `include_dir!` crate and serves via Axum alongside existing REST API. Auth uses password + JWT cookie. Dashboard is disabled unless `ADMIN_PASSWORD` env var is set.

**Tech Stack:** Svelte 5, Tailwind CSS 4, Vite, Axum, jsonwebtoken (Rust), include_dir (Rust)

---

### Task 1: Scaffold Svelte + Tailwind Dashboard Project

**Files:**
- Create: `dashboard/package.json`
- Create: `dashboard/vite.config.ts`
- Create: `dashboard/svelte.config.js`
- Create: `dashboard/tailwind.config.js`
- Create: `dashboard/tsconfig.json`
- Create: `dashboard/src/main.ts`
- Create: `dashboard/src/App.svelte`
- Create: `dashboard/src/app.css`
- Create: `dashboard/index.html`

**Step 1: Create the dashboard directory and initialize**

```bash
cd dashboard
npm create vite@latest . -- --template svelte-ts
npm install
npm install -D tailwindcss @tailwindcss/vite
```

**Step 2: Configure Vite for base path `/admin`**

In `vite.config.ts`:
```ts
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  base: '/admin/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
```

**Step 3: Set up Tailwind in `src/app.css`**

```css
@import "tailwindcss";
```

**Step 4: Create minimal App.svelte**

```svelte
<script lang="ts">
  import './app.css'
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900">
  <h1 class="text-2xl font-bold p-8 text-gray-900 dark:text-white">AmanClaw Dashboard</h1>
</div>
```

**Step 5: Build and verify output**

Run: `npm run build`
Expected: `dist/` directory with `index.html`, JS, and CSS files

**Step 6: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): scaffold Svelte + Tailwind project"
```

---

### Task 2: Rust — Embed Static Files & Serve Dashboard

**Files:**
- Modify: `rust/crates/amanclaw-api/Cargo.toml`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`

**Step 1: Add `include_dir` dependency**

In `rust/crates/amanclaw-api/Cargo.toml`, add:
```toml
include_dir = "0.7"
```

**Step 2: Add static file serving to `lib.rs`**

Add a new function that serves embedded files at `/admin/*`:

```rust
use axum::http::{header, StatusCode as HttpStatus, Uri};
use axum::response::{Html, IntoResponse as _};
use include_dir::{include_dir, Dir};

static DASHBOARD_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../dashboard/dist");

async fn serve_dashboard(uri: Uri) -> impl axum::response::IntoResponse {
    let path = uri.path().strip_prefix("/admin/").unwrap_or("");
    let path = if path.is_empty() { "index.html" } else { path };

    match DASHBOARD_DIR.get_file(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                HttpStatus::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                file.contents(),
            )
                .into_response()
        }
        None => {
            // SPA fallback — serve index.html for client-side routing
            match DASHBOARD_DIR.get_file("index.html") {
                Some(index) => Html(std::str::from_utf8(index.contents()).unwrap_or("")).into_response(),
                None => (HttpStatus::NOT_FOUND, "Dashboard not found").into_response(),
            }
        }
    }
}
```

**Step 3: Add `mime_guess` dependency**

In `Cargo.toml`:
```toml
mime_guess = "2"
```

**Step 4: Mount the dashboard route in `api_router`**

In the `api_router` function, add before the final `Router::new()`:

```rust
let dashboard_routes = Router::new()
    .route("/admin/{*path}", get(serve_dashboard))
    .route("/admin", get(serve_dashboard));
```

And merge it:
```rust
Router::new()
    .merge(authed)
    .merge(webhook_routes)
    .merge(metrics_routes)
    .merge(ws_routes)
    .merge(dashboard_routes)
    .layer(CorsLayer::permissive())
    .layer(TraceLayer::new_for_http())
```

**Step 5: Build dashboard first, then compile Rust**

Run: `cd dashboard && npm run build && cd ../rust && cargo check`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-api/ dashboard/dist/
git commit -m "feat(api): serve embedded dashboard at /admin"
```

---

### Task 3: Auth — Password Login & JWT Cookie

**Files:**
- Modify: `rust/crates/amanclaw-api/Cargo.toml`
- Modify: `rust/crates/amanclaw-api/src/auth.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/src/state.rs`

**Step 1: Add `jsonwebtoken` dependency**

In `Cargo.toml`:
```toml
jsonwebtoken = "9"
rand = "0.9"
```

**Step 2: Add `admin_password` and `jwt_secret` to `ApiState`**

In `state.rs`, add fields:
```rust
pub admin_password: Option<String>,  // From ADMIN_PASSWORD env var
pub jwt_secret: String,              // Auto-generated or loaded
```

**Step 3: Add login endpoint and cookie-based auth to `auth.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode};
use axum::response::IntoResponse;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub role: String,
    pub exp: usize,
}

pub async fn login(
    State(state): State<ApiState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let admin_pw = state.admin_password.as_ref().ok_or(StatusCode::FORBIDDEN)?;
    if body.password != *admin_pw {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims { role: "admin".into(), exp };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cookie = format!(
        "amanclaw_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400",
        token
    );

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie)],
        Json(serde_json::json!({ "ok": true })),
    ))
}
```

**Step 4: Update `require_auth` to also accept JWT cookie**

```rust
pub async fn require_auth(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check Bearer token first (existing behavior)
    let bearer_ok = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t == state.api_token)
        .unwrap_or(false);

    if bearer_ok {
        return Ok(next.run(request).await);
    }

    // Check JWT cookie
    let cookie_ok = request
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';')
                .find_map(|c| c.trim().strip_prefix("amanclaw_token="))
        })
        .map(|token| {
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
                &Validation::default(),
            ).is_ok()
        })
        .unwrap_or(false);

    if cookie_ok {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
```

**Step 5: Add login route to `api_router`** (no auth middleware)

```rust
let login_route = Router::new()
    .route("/api/login", post(auth::login))
    .with_state(state.clone());
```

Merge it with the other routes.

**Step 6: Initialize JWT secret and admin password in `main.rs`**

In `amanclaw-cli/src/main.rs`, when constructing `ApiState`:
```rust
let admin_password = std::env::var("ADMIN_PASSWORD").ok();
let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
    // Generate random secret
    use rand::Rng;
    let secret: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    secret
});
```

**Step 7: Compile and verify**

Run: `cd rust && cargo check`
Expected: Compiles

**Step 8: Commit**

```bash
git add rust/crates/amanclaw-api/ rust/crates/amanclaw-cli/
git commit -m "feat(api): add password login with JWT cookie auth"
```

---

### Task 4: Frontend — API Client & Auth Store

**Files:**
- Create: `dashboard/src/lib/stores/auth.ts`
- Create: `dashboard/src/lib/stores/api.ts`
- Create: `dashboard/src/lib/pages/Login.svelte`

**Step 1: Create API client**

`dashboard/src/lib/stores/api.ts`:
```ts
const BASE = '/api'

export async function apiFetch(path: string, opts: RequestInit = {}) {
  const res = await fetch(`${BASE}${path}`, {
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json', ...opts.headers as any },
    ...opts,
  })
  if (res.status === 401) {
    window.location.hash = '#/login'
    throw new Error('Unauthorized')
  }
  return res.json()
}

export async function login(password: string) {
  const res = await fetch(`${BASE}/login`, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password }),
  })
  if (!res.ok) throw new Error('Invalid password')
  return res.json()
}
```

**Step 2: Create auth store**

`dashboard/src/lib/stores/auth.ts`:
```ts
import { writable } from 'svelte/store'

export const isLoggedIn = writable(false)
```

**Step 3: Create Login page**

`dashboard/src/lib/pages/Login.svelte`:
```svelte
<script lang="ts">
  import { login } from '../stores/api'
  import { isLoggedIn } from '../stores/auth'

  let password = ''
  let error = ''
  let loading = false

  async function handleLogin() {
    loading = true
    error = ''
    try {
      await login(password)
      $isLoggedIn = true
      window.location.hash = '#/'
    } catch (e: any) {
      error = e.message
    } finally {
      loading = false
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900">
  <div class="w-full max-w-sm p-8 bg-white dark:bg-gray-800 rounded-xl shadow-lg">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">AmanClaw</h1>
    <form on:submit|preventDefault={handleLogin} class="space-y-4">
      <input
        type="password"
        bind:value={password}
        placeholder="Admin password"
        class="w-full px-4 py-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 outline-none"
      />
      {#if error}
        <p class="text-red-500 text-sm">{error}</p>
      {/if}
      <button
        type="submit"
        disabled={loading}
        class="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium disabled:opacity-50"
      >
        {loading ? 'Logging in...' : 'Login'}
      </button>
    </form>
  </div>
</div>
```

**Step 4: Build and verify**

Run: `cd dashboard && npm run build`
Expected: Builds without errors

**Step 5: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): add login page and API client"
```

---

### Task 5: Frontend — App Shell (Sidebar + Router)

**Files:**
- Create: `dashboard/src/lib/components/Sidebar.svelte`
- Create: `dashboard/src/lib/components/MobileNav.svelte`
- Modify: `dashboard/src/App.svelte`

**Step 1: Create Sidebar**

`dashboard/src/lib/components/Sidebar.svelte`:
```svelte
<script lang="ts">
  export let currentPage: string

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊' },
    { id: 'users', label: 'Users', icon: '👥' },
    { id: 'skills', label: 'Skills', icon: '⚡' },
    { id: 'channels', label: 'Channels', icon: '📡' },
    { id: 'communities', label: 'Communities', icon: '🏘️' },
    { id: 'content', label: 'Content', icon: '📝' },
    { id: 'logs', label: 'Logs', icon: '📋' },
    { id: 'settings', label: 'Settings', icon: '⚙️' },
  ]
</script>

<aside class="hidden md:flex md:w-64 md:flex-col bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700">
  <div class="p-6">
    <h1 class="text-xl font-bold text-gray-900 dark:text-white">AmanClaw</h1>
    <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Management Dashboard</p>
  </div>
  <nav class="flex-1 px-3 space-y-1">
    {#each navItems as item}
      <button
        on:click={() => window.location.hash = `#/${item.id}`}
        class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors
          {currentPage === item.id
            ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 font-medium'
            : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        <span>{item.icon}</span>
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>
```

**Step 2: Create MobileNav**

`dashboard/src/lib/components/MobileNav.svelte`:
```svelte
<script lang="ts">
  export let currentPage: string

  const navItems = [
    { id: 'dashboard', label: 'Home', icon: '📊' },
    { id: 'users', label: 'Users', icon: '👥' },
    { id: 'skills', label: 'Skills', icon: '⚡' },
    { id: 'logs', label: 'Logs', icon: '📋' },
    { id: 'settings', label: 'More', icon: '⚙️' },
  ]
</script>

<nav class="md:hidden fixed bottom-0 left-0 right-0 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 z-50">
  <div class="flex justify-around py-2">
    {#each navItems as item}
      <button
        on:click={() => window.location.hash = `#/${item.id}`}
        class="flex flex-col items-center gap-1 px-3 py-1 text-xs
          {currentPage === item.id
            ? 'text-blue-600 dark:text-blue-400'
            : 'text-gray-500 dark:text-gray-400'}"
      >
        <span class="text-lg">{item.icon}</span>
        <span>{item.label}</span>
      </button>
    {/each}
  </div>
</nav>
```

**Step 3: Update App.svelte with hash router**

```svelte
<script lang="ts">
  import './app.css'
  import Sidebar from './lib/components/Sidebar.svelte'
  import MobileNav from './lib/components/MobileNav.svelte'
  import Login from './lib/pages/Login.svelte'
  import Dashboard from './lib/pages/Dashboard.svelte'
  import Users from './lib/pages/Users.svelte'
  import Skills from './lib/pages/Skills.svelte'
  import Channels from './lib/pages/Channels.svelte'
  import Communities from './lib/pages/Communities.svelte'
  import Content from './lib/pages/Content.svelte'
  import Logs from './lib/pages/Logs.svelte'
  import Settings from './lib/pages/Settings.svelte'
  import { isLoggedIn } from './lib/stores/auth'

  let currentPage = 'dashboard'

  function updatePage() {
    const hash = window.location.hash.slice(2) || 'dashboard'
    currentPage = hash
  }

  updatePage()
  window.addEventListener('hashchange', updatePage)
</script>

{#if currentPage === 'login' || !$isLoggedIn}
  <Login />
{:else}
  <div class="flex h-screen bg-gray-50 dark:bg-gray-900">
    <Sidebar {currentPage} />
    <main class="flex-1 overflow-auto pb-16 md:pb-0">
      {#if currentPage === 'dashboard'}
        <Dashboard />
      {:else if currentPage === 'users'}
        <Users />
      {:else if currentPage === 'skills'}
        <Skills />
      {:else if currentPage === 'channels'}
        <Channels />
      {:else if currentPage === 'communities'}
        <Communities />
      {:else if currentPage === 'content'}
        <Content />
      {:else if currentPage === 'logs'}
        <Logs />
      {:else if currentPage === 'settings'}
        <Settings />
      {:else}
        <Dashboard />
      {/if}
    </main>
    <MobileNav {currentPage} />
  </div>
{/if}
```

**Step 4: Create stub pages** (Dashboard, Users, Skills, Channels, Communities, Content, Logs, Settings)

Each page starts as a simple placeholder:
```svelte
<script lang="ts">
  // Page logic will go here
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Page Title</h2>
  <p class="text-gray-500">Coming soon...</p>
</div>
```

**Step 5: Build and verify**

Run: `cd dashboard && npm run build`
Expected: Builds without errors

**Step 6: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): add app shell with sidebar, mobile nav, and router"
```

---

### Task 6: Dashboard Page

**Files:**
- Modify: `dashboard/src/lib/pages/Dashboard.svelte`
- Create: `dashboard/src/lib/components/StatCard.svelte`
- Create: `dashboard/src/lib/components/StatusBadge.svelte`

**Step 1: Create StatCard component**

```svelte
<script lang="ts">
  export let label: string
  export let value: string | number
  export let icon: string = ''
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
  <div class="flex items-center justify-between">
    <div>
      <p class="text-sm text-gray-500 dark:text-gray-400">{label}</p>
      <p class="text-3xl font-bold text-gray-900 dark:text-white mt-1">{value}</p>
    </div>
    {#if icon}
      <span class="text-3xl opacity-50">{icon}</span>
    {/if}
  </div>
</div>
```

**Step 2: Create StatusBadge component**

```svelte
<script lang="ts">
  export let status: 'online' | 'offline' | 'warning' = 'offline'
  export let label: string = ''

  const colors = {
    online: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
    offline: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    warning: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
  }
</script>

<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {colors[status]}">
  <span class="w-2 h-2 rounded-full {status === 'online' ? 'bg-green-500' : status === 'warning' ? 'bg-yellow-500' : 'bg-red-500'}"></span>
  {label || status}
</span>
```

**Step 3: Implement Dashboard page**

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatCard from '../components/StatCard.svelte'
  import StatusBadge from '../components/StatusBadge.svelte'

  let status: any = null
  let loading = true

  onMount(async () => {
    try {
      status = await apiFetch('/status')
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })

  function formatUptime(seconds: number): string {
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    return h > 0 ? `${h}h ${m}m` : `${m}m`
  }
</script>

<div class="p-6 md:p-8">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h2>
    {#if status}
      <StatusBadge status={status.running ? 'online' : 'offline'} label={status.running ? 'Running' : 'Stopped'} />
    {/if}
  </div>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if status}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <StatCard label="Uptime" value={formatUptime(status.uptime_seconds)} icon="⏱️" />
      <StatCard label="Users" value={status.users_count} icon="👥" />
      <StatCard label="Communities" value={status.communities_count} icon="🏘️" />
      <StatCard label="Skills" value={status.skills_count} icon="⚡" />
    </div>
  {:else}
    <p class="text-red-500">Failed to load status</p>
  {/if}
</div>
```

**Step 4: Build and verify**

Run: `cd dashboard && npm run build`

**Step 5: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): implement dashboard page with stats"
```

---

### Task 7: Users Page

**Files:**
- Modify: `dashboard/src/lib/pages/Users.svelte`

**Step 1: Implement Users page**

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let users: any[] = []
  let loading = true
  let filter = ''

  onMount(loadUsers)

  async function loadUsers() {
    loading = true
    try {
      const data = await apiFetch('/users')
      users = data.users
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  async function approveUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/approve`, { method: 'POST' })
    await loadUsers()
  }

  async function blockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/block`, { method: 'POST' })
    await loadUsers()
  }

  $: filteredUsers = users.filter(u =>
    u.user_id.includes(filter) || u.platform.includes(filter) || u.state.toLowerCase().includes(filter.toLowerCase())
  )
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Users</h2>

  <input
    type="text"
    bind:value={filter}
    placeholder="Search users..."
    class="w-full max-w-md mb-6 px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
  />

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-200 dark:border-gray-700">
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">User ID</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Platform</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Status</th>
              <th class="text-right px-4 py-3 text-xs font-medium text-gray-500 uppercase">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredUsers as user}
              <tr class="border-b border-gray-100 dark:border-gray-700/50">
                <td class="px-4 py-3 text-sm text-gray-900 dark:text-white font-mono">{user.user_id}</td>
                <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-300">{user.platform}</td>
                <td class="px-4 py-3">
                  <StatusBadge
                    status={user.state === 'Approved' ? 'online' : user.state === 'Blocked' ? 'offline' : 'warning'}
                    label={user.state}
                  />
                </td>
                <td class="px-4 py-3 text-right space-x-2">
                  {#if user.state !== 'Approved'}
                    <button on:click={() => approveUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                      Approve
                    </button>
                  {/if}
                  {#if user.state !== 'Blocked'}
                    <button on:click={() => blockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                      Block
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <p class="text-sm text-gray-500 mt-3">{filteredUsers.length} users</p>
  {/if}
</div>
```

**Step 2: Build and verify**

Run: `cd dashboard && npm run build`

**Step 3: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): implement users page with approve/block"
```

---

### Task 8: Skills Page

**Files:**
- Modify: `dashboard/src/lib/pages/Skills.svelte`

**Step 1: Implement Skills page**

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'

  let skills: any[] = []
  let loading = true

  onMount(async () => {
    try {
      const data = await apiFetch('/skills')
      skills = data.skills
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Skills</h2>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each skills as skill}
        <div class="bg-white dark:bg-gray-800 rounded-xl p-5 shadow-sm border border-gray-200 dark:border-gray-700">
          <div class="flex items-center justify-between mb-2">
            <h3 class="font-semibold text-gray-900 dark:text-white">{skill.name}</h3>
            <span class="w-2 h-2 rounded-full bg-green-500"></span>
          </div>
          <p class="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">{skill.description}</p>
        </div>
      {/each}
    </div>
    <p class="text-sm text-gray-500 mt-4">{skills.length} skills registered</p>
  {/if}
</div>
```

**Step 2: Build, commit**

```bash
cd dashboard && npm run build
git add dashboard/
git commit -m "feat(dashboard): implement skills page"
```

---

### Task 9: Channels Page

**Files:**
- Modify: `dashboard/src/lib/pages/Channels.svelte`

**Note:** The channels status isn't currently exposed via a dedicated API endpoint. We'll use `GET /api/status` and extend it in a future iteration. For now, show static channel info based on what's configured.

**Step 1: Implement Channels page**

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let status: any = null
  let loading = true

  // For now, detect channels from env vars via status endpoint
  // Future: dedicated /api/channels endpoint
  onMount(async () => {
    try {
      status = await apiFetch('/status')
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  })
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Channels</h2>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if status}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Telegram</h3>
          <StatusBadge status={status.running ? 'online' : 'offline'} />
        </div>
        <p class="text-sm text-gray-500">Bot messaging via Telegram</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">WhatsApp Web</h3>
          <StatusBadge status={status.running ? 'online' : 'offline'} />
        </div>
        <p class="text-sm text-gray-500">Via wa-bridge (WAHA-compatible)</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Discord</h3>
          <StatusBadge status="offline" label="Not configured" />
        </div>
        <p class="text-sm text-gray-500">Discord bot integration</p>
      </div>

      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-white">Slack</h3>
          <StatusBadge status="offline" label="Not configured" />
        </div>
        <p class="text-sm text-gray-500">Slack workspace integration</p>
      </div>
    </div>
  {/if}
</div>
```

**Step 2: Build, commit**

```bash
cd dashboard && npm run build
git add dashboard/
git commit -m "feat(dashboard): implement channels page"
```

---

### Task 10: Communities Page

**Files:**
- Modify: `dashboard/src/lib/pages/Communities.svelte`

**Step 1: Implement Communities page with CRUD**

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'

  let communities: any[] = []
  let loading = true
  let showForm = false
  let form = { name: '', zone: 'SGR01', language: 'ms', platform: 'telegram', platform_group_id: '' }

  onMount(loadCommunities)

  async function loadCommunities() {
    loading = true
    try {
      const data = await apiFetch('/communities')
      communities = data.communities
    } catch (e) { console.error(e) }
    finally { loading = false }
  }

  async function createCommunity() {
    await apiFetch('/communities', { method: 'POST', body: JSON.stringify(form) })
    showForm = false
    form = { name: '', zone: 'SGR01', language: 'ms', platform: 'telegram', platform_group_id: '' }
    await loadCommunities()
  }

  async function deleteCommunity(id: string) {
    if (!confirm('Delete this community?')) return
    await apiFetch(`/communities/${id}`, { method: 'DELETE' })
    await loadCommunities()
  }
</script>

<div class="p-6 md:p-8">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Communities</h2>
    <button on:click={() => showForm = !showForm}
      class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm">
      {showForm ? 'Cancel' : '+ Add'}
    </button>
  </div>

  {#if showForm}
    <form on:submit|preventDefault={createCommunity}
      class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700 mb-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
      <input bind:value={form.name} placeholder="Community name" required
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <input bind:value={form.zone} placeholder="Zone (e.g. SGR01)"
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <input bind:value={form.platform_group_id} placeholder="Group ID" required
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
      <select bind:value={form.platform}
        class="px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white">
        <option value="telegram">Telegram</option>
        <option value="whatsapp-web">WhatsApp</option>
        <option value="discord">Discord</option>
        <option value="slack">Slack</option>
      </select>
      <button type="submit" class="sm:col-span-2 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg">Create</button>
    </form>
  {/if}

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if communities.length === 0}
    <p class="text-gray-500">No communities yet.</p>
  {:else}
    <div class="space-y-3">
      {#each communities as c}
        <div class="bg-white dark:bg-gray-800 rounded-xl p-5 shadow-sm border border-gray-200 dark:border-gray-700 flex items-center justify-between">
          <div>
            <h3 class="font-semibold text-gray-900 dark:text-white">{c.name}</h3>
            <p class="text-sm text-gray-500">{c.platform} · {c.zone} · {c.language}</p>
            {#if c.enabled_skills?.length}
              <p class="text-xs text-gray-400 mt-1">Skills: {c.enabled_skills.join(', ')}</p>
            {/if}
          </div>
          <button on:click={() => deleteCommunity(c.id)}
            class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">Delete</button>
        </div>
      {/each}
    </div>
  {/if}
</div>
```

**Step 2: Build, commit**

```bash
cd dashboard && npm run build
git add dashboard/
git commit -m "feat(dashboard): implement communities page with CRUD"
```

---

### Task 11: Logs Page (WebSocket)

**Files:**
- Modify: `dashboard/src/lib/pages/Logs.svelte`

**Note:** The existing WebSocket at `/ws` uses JSON-RPC. For live logs, we need a simpler approach. We'll add a new SSE (Server-Sent Events) endpoint `/api/logs/stream` to the Rust API, or use the existing WebSocket. For the MVP, we'll poll `docker logs` output via a new endpoint.

**Step 1: Add a `/api/logs` endpoint to the Rust API**

In `rust/crates/amanclaw-api/src/routes/mod.rs`, add:
```rust
pub mod logs;
```

Create `rust/crates/amanclaw-api/src/routes/logs.rs`:
```rust
use crate::state::ApiState;
use axum::{Json, extract::State};

/// Returns recent log entries stored in a ring buffer.
/// For MVP, returns the last N tracing events captured by the subscriber.
pub async fn get_logs(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let logs = state.log_buffer.read().await;
    Json(serde_json::json!({ "logs": *logs }))
}
```

Add `log_buffer: Arc<RwLock<Vec<serde_json::Value>>>` to `ApiState`.

This requires a custom tracing layer that captures events into a shared buffer — implement in a follow-up. For the MVP Logs page, poll `/api/logs` every 2 seconds.

**Step 2: Implement Logs page with polling**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { apiFetch } from '../stores/api'

  let logs: any[] = []
  let loading = true
  let filter = ''
  let autoScroll = true
  let interval: any

  onMount(() => {
    loadLogs()
    interval = setInterval(loadLogs, 3000)
  })

  onDestroy(() => clearInterval(interval))

  async function loadLogs() {
    try {
      const data = await apiFetch('/logs')
      logs = data.logs || []
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  $: filteredLogs = filter
    ? logs.filter(l => JSON.stringify(l).toLowerCase().includes(filter.toLowerCase()))
    : logs
</script>

<div class="p-6 md:p-8 flex flex-col h-full">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Logs</h2>
    <label class="flex items-center gap-2 text-sm text-gray-500">
      <input type="checkbox" bind:checked={autoScroll} />
      Auto-scroll
    </label>
  </div>

  <input
    type="text"
    bind:value={filter}
    placeholder="Filter logs..."
    class="w-full max-w-md mb-4 px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
  />

  <div class="flex-1 bg-gray-900 rounded-xl p-4 overflow-auto font-mono text-xs text-gray-300 min-h-[400px]">
    {#if loading}
      <p class="text-gray-500">Loading...</p>
    {:else if filteredLogs.length === 0}
      <p class="text-gray-500">No logs yet. Logs will appear here as events occur.</p>
    {:else}
      {#each filteredLogs as log}
        <div class="py-0.5 hover:bg-gray-800/50">
          <span class="text-gray-500">{log.timestamp || ''}</span>
          <span class="{log.level === 'ERROR' ? 'text-red-400' : log.level === 'WARN' ? 'text-yellow-400' : 'text-green-400'}">[{log.level || 'INFO'}]</span>
          <span>{log.message || JSON.stringify(log)}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>
```

**Step 3: Build, commit**

```bash
cd dashboard && npm run build
git add dashboard/ rust/crates/amanclaw-api/
git commit -m "feat(dashboard): implement logs page with polling"
```

---

### Task 12: Content & Settings Pages (Stubs)

**Files:**
- Modify: `dashboard/src/lib/pages/Content.svelte`
- Modify: `dashboard/src/lib/pages/Settings.svelte`

**Step 1: Content page stub** (functional placeholder)

```svelte
<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Content Management</h2>
  <div class="bg-white dark:bg-gray-800 rounded-xl p-8 shadow-sm border border-gray-200 dark:border-gray-700 text-center">
    <p class="text-gray-500 dark:text-gray-400">Content management (doa, zakat rates, khutbah) will be available in a future update.</p>
  </div>
</div>
```

**Step 2: Settings page** (shows current config, logout)

```svelte
<script lang="ts">
  import { isLoggedIn } from '../stores/auth'

  function logout() {
    document.cookie = 'amanclaw_token=; Max-Age=0; Path=/'
    $isLoggedIn = false
    window.location.hash = '#/login'
  }
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Settings</h2>

  <div class="space-y-4 max-w-lg">
    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Account</h3>
      <button on:click={logout}
        class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
        Logout
      </button>
    </div>

    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <h3 class="font-semibold text-gray-900 dark:text-white mb-2">About</h3>
      <p class="text-sm text-gray-500">AmanClaw Management Dashboard</p>
      <p class="text-xs text-gray-400 mt-1">LLM config, bot settings, and advanced options coming in a future update.</p>
    </div>
  </div>
</div>
```

**Step 3: Build, commit**

```bash
cd dashboard && npm run build
git add dashboard/
git commit -m "feat(dashboard): add content stub and settings with logout"
```

---

### Task 13: Update Dockerfile for Multi-Stage Build

**Files:**
- Modify: `rust/Dockerfile`

**Step 1: Add Node.js build stage before Rust build**

```dockerfile
# Stage 1: Build Svelte dashboard
FROM node:20-slim AS dashboard-builder
WORKDIR /dashboard
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ .
RUN npm run build

# Stage 2: Build Rust binary (with embedded dashboard)
FROM rust:1.88-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY rust/ ./
COPY --from=dashboard-builder /dashboard/dist /app/../dashboard/dist
RUN cargo build --release -p amanclaw-cli

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home amanclaw
WORKDIR /home/amanclaw
COPY --from=builder /app/target/release/amanclaw /usr/local/bin/amanclaw
RUN mkdir -p plugins data
RUN chown -R amanclaw:amanclaw /home/amanclaw
USER amanclaw
EXPOSE 8443
CMD ["amanclaw"]
```

**Step 2: Update `.dockerignore`** to exclude `node_modules`

```
dashboard/node_modules
dashboard/.svelte-kit
```

**Step 3: Test build locally**

Run: `docker build -t amanclaw:test -f rust/Dockerfile .`
Expected: Builds successfully with dashboard embedded

**Step 4: Commit**

```bash
git add rust/Dockerfile .dockerignore
git commit -m "feat(docker): multi-stage build with dashboard frontend"
```

---

### Task 14: Update .env.example and config.example.yaml

**Files:**
- Modify: `.env.example`
- Modify: `config.example.yaml`

**Step 1: Add `ADMIN_PASSWORD` and `API_PORT` to `.env.example`**

Add:
```
# Web Dashboard
# ADMIN_PASSWORD=your_admin_password
# API_PORT=8443
```

**Step 2: Add dashboard section to `config.example.yaml`**

Add comment section:
```yaml
# --- Web Dashboard (optional) ---
# Set ADMIN_PASSWORD in .env to enable the web dashboard.
# Access at http://your-host:8443/admin
# API_PORT defaults to 8443.
```

**Step 3: Commit**

```bash
git add .env.example config.example.yaml
git commit -m "docs: add dashboard config to .env.example and config.example.yaml"
```

---

## Summary

| Task | What | Files |
| --- | --- | --- |
| 1 | Scaffold Svelte + Tailwind | `dashboard/*` |
| 2 | Embed static files in Rust | `amanclaw-api` |
| 3 | Password login + JWT cookie | `amanclaw-api/auth.rs`, `state.rs` |
| 4 | API client + auth store + Login page | `dashboard/src/lib/*` |
| 5 | App shell (sidebar, mobile nav, router) | `dashboard/src/*` |
| 6 | Dashboard page (stats) | `Dashboard.svelte`, components |
| 7 | Users page (approve/block) | `Users.svelte` |
| 8 | Skills page | `Skills.svelte` |
| 9 | Channels page | `Channels.svelte` |
| 10 | Communities page (CRUD) | `Communities.svelte` |
| 11 | Logs page (polling) | `Logs.svelte`, `routes/logs.rs` |
| 12 | Content stub + Settings/Logout | `Content.svelte`, `Settings.svelte` |
| 13 | Multi-stage Dockerfile | `Dockerfile` |
| 14 | Update .env.example + config docs | `.env.example`, `config.example.yaml` |
