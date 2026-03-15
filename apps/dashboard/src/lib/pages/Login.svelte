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
