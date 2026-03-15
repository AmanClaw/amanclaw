<script lang="ts">
  import { Badge } from '@amanclaw/ui'
  import { Eye, EyeOff } from '@amanclaw/ui'

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

  const statusBadge = $derived<'success' | 'error' | 'warning'>(
    channel.running ? 'success' :
    channel.error ? 'error' :
    channel.configured ? 'warning' :
    'error'
  )

  const statusLabel = $derived(
    channel.running ? 'Connected' :
    channel.error ? 'Error' :
    channel.configured ? 'Configured' :
    'Not configured'
  )

  const borderColor = $derived(
    channel.running ? 'border-green-500/30' :
    channel.error ? 'border-red-500/30' :
    channel.configured ? 'border-amber-500/30' :
    'border-border'
  )
</script>

<div class="bg-base rounded-xl p-6 border {borderColor}">
  <div class="flex items-center justify-between mb-2">
    <h3 class="font-semibold text-fg">{label}</h3>
    <Badge variant={statusBadge}>
      <span class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full {channel.running ? 'bg-green-400' : channel.error ? 'bg-red-400' : channel.configured ? 'bg-amber-400' : 'bg-red-400'}"></span>
        {statusLabel}
      </span>
    </Badge>
  </div>
  <p class="text-[13px] text-fg-muted mb-4">{description}</p>

  {#if channel.error}
    <div class="mb-3 p-2 bg-[var(--color-error-15)] rounded text-xs text-red-400">
      {channel.error}
    </div>
  {/if}

  {#if editing}
    <div class="space-y-3 mb-4">
      {#each fields as field}
        <div>
          <label class="block text-xs font-medium text-fg-secondary mb-1">{field.label}</label>
          <div class="relative">
            <input
              type={field.type === 'password' && !showPassword[field.key] ? 'password' : 'text'}
              bind:value={formValues[field.key]}
              placeholder={field.placeholder}
              class="w-full px-3 py-2 text-sm border border-border bg-elevated text-fg rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500/50 focus:border-transparent"
            />
            {#if field.type === 'password'}
              <button
                type="button"
                onclick={() => showPassword[field.key] = !showPassword[field.key]}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-fg-muted hover:text-fg-secondary"
              >
                {#if showPassword[field.key]}
                  <EyeOff size={14} />
                {:else}
                  <Eye size={14} />
                {/if}
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
        class="px-3 py-1.5 text-xs font-medium rounded-md bg-primary-600 text-white hover:bg-primary-500 disabled:opacity-50 transition-colors"
      >
        {saving ? 'Saving...' : 'Save & Connect'}
      </button>
      <button
        onclick={cancelForm}
        class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors"
      >
        Cancel
      </button>
    </div>
  {:else}
    <div class="flex gap-2">
      {#if !channel.configured}
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-primary-600 text-white hover:bg-primary-500 transition-colors"
        >
          Setup
        </button>
      {:else if channel.running}
        <button
          onclick={handleStop}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-500/30 text-red-400 hover:bg-[var(--color-error-15)] transition-colors"
        >
          Stop
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors"
        >
          Edit
        </button>
      {:else}
        <button
          onclick={handleStart}
          disabled={starting}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 text-white hover:bg-green-500 disabled:opacity-50 transition-colors"
        >
          {starting ? 'Starting...' : 'Start'}
        </button>
        <button
          onclick={openForm}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors"
        >
          Edit
        </button>
      {/if}
    </div>
  {/if}
</div>
