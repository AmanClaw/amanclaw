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
