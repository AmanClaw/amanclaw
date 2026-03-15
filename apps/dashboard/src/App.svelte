<script lang="ts">
  import './app.css'
  import { Sidebar as SharedSidebar, TopBar, BottomNav } from '@amanclaw/ui'
  import {
    LayoutDashboard, User, Zap, Hash, Users,
    FileText, Server, ScrollText, Settings
  } from '@amanclaw/ui'
  import Login from './lib/pages/Login.svelte'
  import Dashboard from './lib/pages/Dashboard.svelte'
  import UsersPage from './lib/pages/Users.svelte'
  import Skills from './lib/pages/Skills.svelte'
  import Channels from './lib/pages/Channels.svelte'
  import Communities from './lib/pages/Communities.svelte'
  import Content from './lib/pages/Content.svelte'
  import Logs from './lib/pages/Logs.svelte'
  import McpServers from './lib/pages/McpServers.svelte'
  import SettingsPage from './lib/pages/Settings.svelte'
  import { isLoggedIn } from './lib/stores/auth'

  let currentPage = $state('dashboard')

  function updatePage() {
    currentPage = window.location.hash.slice(2) || 'dashboard'
  }

  updatePage()
  window.addEventListener('hashchange', updatePage)

  function handleNavigate(id: string) {
    window.location.hash = `#/${id}`
  }

  const navGroups = [
    {
      label: 'Main',
      items: [
        { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
        { id: 'users', label: 'Users', icon: User },
        { id: 'skills', label: 'Skills', icon: Zap },
        { id: 'channels', label: 'Channels', icon: Hash },
        { id: 'communities', label: 'Communities', icon: Users },
      ]
    },
    {
      label: 'System',
      items: [
        { id: 'content', label: 'Content', icon: FileText },
        { id: 'mcp', label: 'MCP Servers', icon: Server },
        { id: 'logs', label: 'Logs', icon: ScrollText },
        { id: 'settings', label: 'Settings', icon: Settings },
      ]
    }
  ]

  const mobileItems = [
    { id: 'dashboard', label: 'Home', icon: LayoutDashboard },
    { id: 'communities', label: 'Groups', icon: Users },
    { id: 'skills', label: 'Skills', icon: Zap },
    { id: 'settings', label: 'Settings', icon: Settings },
  ]

  const moreItems = [
    { id: 'users', label: 'Users', icon: User },
    { id: 'channels', label: 'Channels', icon: Hash },
    { id: 'content', label: 'Content', icon: FileText },
    { id: 'mcp', label: 'MCP Servers', icon: Server },
    { id: 'logs', label: 'Logs', icon: ScrollText },
  ]

  const pageTitles: Record<string, string> = {
    dashboard: 'Dashboard', users: 'Users', skills: 'Skills',
    channels: 'Channels', communities: 'Communities', content: 'Content',
    mcp: 'MCP Servers', logs: 'Logs', settings: 'Settings',
  }
</script>

{#if currentPage === 'login' || !$isLoggedIn}
  <Login />
{:else}
  <div class="flex h-screen bg-base select-none">
    <div class="hidden md:block">
      <SharedSidebar
        groups={navGroups}
        activePage={currentPage}
        onNavigate={handleNavigate}
        userName="Admin"
        userInitials="AD"
        logoUrl="/admin/logo.png"
      />
    </div>
    <div class="flex-1 flex flex-col overflow-hidden">
      <TopBar
        breadcrumbs={[
          { label: navGroups.find(g => g.items.some(i => i.id === currentPage))?.label ?? 'Main' },
          { label: pageTitles[currentPage] ?? currentPage, active: true }
        ]}
      />
      <main class="flex-1 overflow-y-auto p-6">
        {#if currentPage === 'dashboard'}
          <Dashboard />
        {:else if currentPage === 'users'}
          <UsersPage />
        {:else if currentPage === 'skills'}
          <Skills />
        {:else if currentPage === 'channels'}
          <Channels />
        {:else if currentPage === 'communities'}
          <Communities />
        {:else if currentPage === 'content'}
          <Content />
        {:else if currentPage === 'mcp'}
          <McpServers />
        {:else if currentPage === 'logs'}
          <Logs />
        {:else if currentPage === 'settings'}
          <SettingsPage />
        {:else}
          <Dashboard />
        {/if}
      </main>
    </div>
  </div>
  <!-- BottomNav outside flex container, hidden on desktop -->
  <div class="md:hidden">
    <BottomNav items={mobileItems} {moreItems} activePage={currentPage} onNavigate={handleNavigate} />
  </div>
{/if}
