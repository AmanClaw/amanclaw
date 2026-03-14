<script lang="ts">
  export let currentPage: string

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊' },
    { id: 'users', label: 'Users', icon: '👥' },
    { id: 'skills', label: 'Skills', icon: '⚡' },
    { id: 'channels', label: 'Channels', icon: '📡' },
    { id: 'communities', label: 'Communities', icon: '🏘️' },
    { id: 'content', label: 'Content', icon: '📝' },
    { id: 'mcp', label: 'MCP Servers', icon: '🔌' },
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
