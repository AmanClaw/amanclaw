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
