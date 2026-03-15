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

  let { label, value, icon: Icon, iconColor = 'text-primary-500 bg-[var(--color-primary-500-10)]', trend, trendPositive }: Props = $props();
</script>

<div class="bg-surface border border-border rounded-xl p-5">
  <div class="flex items-center justify-between mb-3">
    <span class="text-xs font-medium uppercase tracking-wide text-fg-muted">{label}</span>
    <div class="w-8 h-8 rounded-lg flex items-center justify-center {iconColor}">
      <Icon size={16} />
    </div>
  </div>
  <p class="text-[28px] font-bold text-fg">{value}</p>
  {#if trend}
    <p class="text-xs mt-1 {trendPositive ? 'text-success' : 'text-fg-muted'}">{trend}</p>
  {/if}
</div>
