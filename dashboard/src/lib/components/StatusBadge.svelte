<script lang="ts">
  export let status: 'online' | 'offline' | 'warning' = 'offline'
  export let label: string = ''

  const colors: Record<string, string> = {
    online: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
    offline: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    warning: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
  }

  const dots: Record<string, string> = {
    online: 'bg-green-500',
    offline: 'bg-red-500',
    warning: 'bg-yellow-500',
  }
</script>

<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {colors[status]}">
  <span class="w-2 h-2 rounded-full {dots[status]}"></span>
  {label || status}
</span>
