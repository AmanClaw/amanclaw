<script lang="ts">
  import {
    LayoutDashboard, User, Zap, Hash, Users,
    FileText, Server, ScrollText, Settings
  } from '@amanclaw/ui';

  let { currentPage }: { currentPage: string } = $props();

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { id: 'users', label: 'Users', icon: User },
    { id: 'skills', label: 'Skills', icon: Zap },
    { id: 'channels', label: 'Channels', icon: Hash },
    { id: 'communities', label: 'Communities', icon: Users },
    { id: 'content', label: 'Content', icon: FileText },
    { id: 'mcp', label: 'MCP Servers', icon: Server },
    { id: 'logs', label: 'Logs', icon: ScrollText },
    { id: 'settings', label: 'Settings', icon: Settings },
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
        onclick={() => window.location.hash = `#/${item.id}`}
        class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors
          {currentPage === item.id
            ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 font-medium'
            : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        <item.icon size={16} class="shrink-0" />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>
