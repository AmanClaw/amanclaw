<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: string;
    action?: Snippet;
  }

  let { title, subtitle, action }: Props = $props();
</script>

<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-2xl font-semibold text-fg">{title}</h1>
    {#if subtitle}
      <p class="text-[13px] text-fg-muted mt-1">{subtitle}</p>
    {/if}
  </div>
  {#if action}
    {@render action()}
  {/if}
</div>
