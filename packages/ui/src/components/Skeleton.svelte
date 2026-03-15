<script lang="ts">
  interface Props { class?: string; width?: string; height?: string; rounded?: 'sm' | 'md' | 'lg' | 'full'; }
  let { class: className = '', width, height = '12px', rounded = 'md' }: Props = $props();
  const r: Record<string, string> = { sm: 'rounded-sm', md: 'rounded', lg: 'rounded-lg', full: 'rounded-full' };
</script>

<div class="bg-elevated animate-pulse {r[rounded]} {className}" style="width: {width ?? '100%'}; height: {height};"></div>
