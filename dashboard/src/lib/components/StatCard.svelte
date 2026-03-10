<script lang="ts">
  export let label: string
  export let value: string | number
  export let icon: string = ''
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
  <div class="flex items-center justify-between">
    <div>
      <p class="text-sm text-gray-500 dark:text-gray-400">{label}</p>
      <p class="text-3xl font-bold text-gray-900 dark:text-white mt-1">{value}</p>
    </div>
    {#if icon}
      <span class="text-3xl opacity-50">{icon}</span>
    {/if}
  </div>
</div>
