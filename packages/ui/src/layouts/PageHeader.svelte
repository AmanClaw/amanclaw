<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: string;
    action?: Snippet;
  }

  let { title, subtitle, action }: Props = $props();
</script>

<div class="flex items-start justify-between mb-8">
  <div>
    <h1 class="text-3xl font-bold text-fg tracking-tight">{title}</h1>
    {#if subtitle}
      <p class="text-sm text-fg-muted mt-1.5">{subtitle}</p>
    {/if}
  </div>
  {#if action}
    <div class="pt-1">
      {@render action()}
    </div>
  {/if}
</div>
