<script lang="ts">
  interface Props {
    label: string
    value: string | number
    icon?: string
  }

  let { label, value, icon = '' }: Props = $props()
</script>

<div class="bg-base border border-border rounded-xl p-6">
  <div class="flex items-center justify-between">
    <div>
      <p class="text-sm text-fg-muted">{label}</p>
      <p class="text-3xl font-bold text-fg mt-1">{value}</p>
    </div>
    {#if icon}
      <span class="text-3xl opacity-50">{icon}</span>
    {/if}
  </div>
</div>
