<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    columns: string[];
    children: Snippet;
  }

  let { columns, children }: Props = $props();
</script>

<div class="bg-surface border border-border rounded-xl overflow-hidden">
  <div class="grid" style="grid-template-columns: repeat({columns.length}, minmax(0, 1fr));">
    {#each columns as col}
      <div class="px-4 py-2.5 bg-[var(--color-elevated-50)] border-b border-border">
        <span class="text-xs font-semibold uppercase tracking-wide text-fg-muted">{col}</span>
      </div>
    {/each}
  </div>
  {@render children()}
</div>
