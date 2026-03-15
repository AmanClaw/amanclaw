<script lang="ts">
  import { login } from '../stores/api'
  import { isLoggedIn } from '../stores/auth'
  import { Key, Loader2 } from '@amanclaw/ui'

  let password = $state('')
  let error = $state('')
  let loading = $state(false)

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

<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-[#0b1628] via-[#0f1d35] to-[#0b1628] relative overflow-hidden">
  <!-- Decorative background elements -->
  <div class="absolute inset-0 overflow-hidden">
    <div class="absolute -top-40 -right-40 w-80 h-80 bg-primary-500/5 rounded-full blur-3xl"></div>
    <div class="absolute -bottom-40 -left-40 w-80 h-80 bg-accent-500/5 rounded-full blur-3xl"></div>
  </div>

  <div class="w-full max-w-sm p-8 relative z-10">
    <!-- Brand Header -->
    <div class="text-center mb-8">
      <img src="/admin/logo.png" alt="AmanClaw" class="w-20 h-20 mx-auto mb-4 drop-shadow-[0_0_20px_rgba(20,184,166,0.3)]" />
      <h1 class="text-2xl font-bold text-white">AmanClaw</h1>
      <p class="text-sm text-[#6b7f9e] mt-1">Management Dashboard</p>
    </div>

    <!-- Login Card -->
    <div class="bg-[#111d33]/80 backdrop-blur-sm rounded-2xl p-6 border border-[#1e2d45] shadow-xl">
      <form onsubmit={(e: Event) => { e.preventDefault(); handleLogin() }} class="space-y-4">
        <div>
          <label class="block text-xs font-medium text-[#6b7f9e] uppercase tracking-wider mb-2">Password</label>
          <div class="relative">
            <div class="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a5f7e]">
              <Key size={16} />
            </div>
            <input
              type="password"
              bind:value={password}
              placeholder="Enter admin password"
              class="w-full pl-10 pr-4 py-3 rounded-xl border border-[#1e2d45] bg-[#0b1628] text-white placeholder-[#4a5f7e] focus:ring-2 focus:ring-primary-500/50 focus:border-primary-500/50 outline-none transition-colors"
            />
          </div>
        </div>
        {#if error}
          <div class="p-3 bg-red-500/10 border border-red-500/20 rounded-lg">
            <p class="text-red-400 text-sm">{error}</p>
          </div>
        {/if}
        <button
          type="submit"
          disabled={loading}
          class="w-full py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500 text-white rounded-xl font-medium disabled:opacity-50 transition-all shadow-lg shadow-primary-500/20 flex items-center justify-center gap-2"
        >
          {#if loading}
            <Loader2 size={16} class="animate-spin" />
            Logging in...
          {:else}
            Login
          {/if}
        </button>
      </form>
    </div>

    <!-- Footer accent -->
    <div class="mt-6 text-center">
      <p class="text-xs text-[#4a5f7e]">Powered by <span class="text-accent-500 font-medium">AmanClaw</span></p>
    </div>
  </div>
</div>
