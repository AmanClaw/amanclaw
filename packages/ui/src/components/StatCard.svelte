<script lang="ts">
  import type { Component } from 'svelte';

  interface Props {
    label: string;
    value: string | number;
    icon: Component;
    iconColor?: string;
    trend?: string;
    trendPositive?: boolean;
  }

  let { label, value, icon: Icon, iconColor = 'text-fg-muted', trend, trendPositive }: Props = $props();
</script>

<div class="bg-base border border-border rounded-xl p-5">
  <div class="flex items-center gap-2 mb-3">
    <Icon size={16} class="{iconColor}" />
    <span class="text-sm text-fg-muted">{label}</span>
  </div>
  <p class="text-3xl font-bold text-fg">{value}</p>
  {#if trend}
    <p class="text-xs mt-2 flex items-center gap-1 {trendPositive ? 'text-success' : 'text-fg-muted'}">
      {#if trendPositive}
        <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none"><path d="M2 8 L6 4 L8 6 L10 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
      {/if}
      {trend}
    </p>
  {/if}
</div>
