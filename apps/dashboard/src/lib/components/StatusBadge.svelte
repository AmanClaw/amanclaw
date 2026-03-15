<script lang="ts">
  interface Props {
    status?: 'online' | 'offline' | 'warning'
    label?: string
  }

  let { status = 'offline', label = '' }: Props = $props()

  const colors: Record<string, string> = {
    online: 'bg-[var(--color-success-15)] text-green-400',
    offline: 'bg-[var(--color-error-15)] text-red-400',
    warning: 'bg-[var(--color-warning-15)] text-amber-400',
  }

  const dots: Record<string, string> = {
    online: 'bg-green-400',
    offline: 'bg-red-400',
    warning: 'bg-amber-400',
  }
</script>

<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {colors[status]}">
  <span class="w-2 h-2 rounded-full {dots[status]}"></span>
  {label || status}
</span>
