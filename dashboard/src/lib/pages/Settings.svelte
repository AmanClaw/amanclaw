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
