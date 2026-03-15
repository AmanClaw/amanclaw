<script lang="ts">
  import StatusBadge from './StatusBadge.svelte'

  interface ChannelStatus {
    id: string
    platform: string
    configured: boolean
    enabled: boolean
    running: boolean
    error: string | null
  }

  interface Props {
    channel: ChannelStatus
    label: string
    description: string
    fields: { key: string; label: string; type: string; placeholder: string; required?: boolean }[]
    configValues: Record<string, string>
    onSave: (values: Record<string, string>) => Promise<void>
    onStart: () => Promise<void>
    onStop: () => Promise<void>
    showQr?: boolean
  }

  let {
    channel,
    label,
    description,
    fields,
    configValues = {},
    onSave,
    onStart,
    onStop,
    showQr = false,
  }: Props = $props()

  let editing = $state(false)
  let saving = $state(false)
  let starting = $state(false)
  let formValues = $state<Record<string, string>>({})
  let showPassword = $state<Record<string, boolean>>({})

  function openForm() {
    formValues = { ...configValues }
    editing = true
  }

  function cancelForm() {
    editing = false
  }

  async function handleSave() {
    saving = true
    try {
      await onSave(formValues)
      editing = false
    } catch (e) {
      console.error('Save failed:', e)
    } finally {
      saving = false
    }
  }

  async function handleStart() {
    starting = true
    try {
      await onStart()
    } catch (e) {
      console.error('Start failed:', e)
    } finally {
      starting = false
    }
  }

  async function handleStop() {
    try {
      await onStop()
    } catch (e) {
      console.error('Stop failed:', e)
    }
  }

  const statusBadge = $derived(
    channel.running ? 'online' :
    channel.error ? 'offline' :
    channel.configured ? 'warning' :
    'offline'
  )

  const statusLabel = $derived(
    channel.running ? 'Connected' :
    channel.error ? 'Error' :
    channel.configured ? 'Configured' :
    'Not configured'
  )

  const borderColor = $derived(
    channel.running ? 'border-green-300 dark:border-green-700' :
    channel.error ? 'border-red-300 dark:border-red-700' :
    channel.configured ? 'border-yellow-300 dark:border-yellow-700' :
    'border-gray-200 dark:border-gray-700'
  )
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border {borderColor}">
  <div class="flex items-center justify-between mb-2">
    <h3 class="font-semibold text-gray-900 dark:text-white">{label}</h3>
    <StatusBadge status={statusBadge} label={statusLabel} />
  </div>
  <p class="text-sm text-gray-500 mb-4">{description}</p>

  {#if channel.error}
    <div class="mb-3 p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs text-red-700 dark:text-red-400">
      {channel.error}
    </div>
  {/if}

  {#if editing}
    <div class="space-y-3 mb-4">
      {#each fields as field}
        <div>
          <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{field.label}</label>
          <div class="relative">
            <input
              type={field.type === 'password' && !showPassword[field.key] ? 'password' : 'text'}
              bind:value={formValues[field.key]}
              placeholder={field.placeholder}
              class="w-full px-3 py-2 text-sm border border-gray-200 dark:border-gray-600 dark:bg-gray-700 dark:text-white rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            {#if field.type === 'password'}
              <button
                type="button"
                onclick={() => showPassword[field.key] = !showPassword[field.key]}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-gray-400 hover:text-gray-600"
              >
                {showPassword[field.key] ? 'Hide' : 'Show'}
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
    <div class="flex gap-2">
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 transition-colors"
      >
        {saving ? 'Saving...' : 'Save & Connect'}
      </button>
      <button
        onclick={cancelForm}
        class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
      >
        Cancel
      </button>
    </div>
  {:else}
    <div class="flex gap-2">
      {#if !channel.configured}
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700 transition-colors"
        >
          Setup
        </button>
      {:else if channel.running}
        <button
          onclick={handleStop}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-300 text-red-700 hover:bg-red-50 transition-colors"
        >
          Stop
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 transition-colors"
        >
          Edit
        </button>
      {:else}
        <button
          onclick={handleStart}
          disabled={starting}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
        >
          {starting ? 'Starting...' : 'Start'}
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 transition-colors"
        >
          Edit
        </button>
      {/if}
    </div>
  {/if}
</div>
