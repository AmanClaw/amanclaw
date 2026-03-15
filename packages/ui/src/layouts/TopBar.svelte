<script lang="ts">
  import { Search, Moon, Sun, ChevronRight } from 'lucide-svelte';
  import { theme } from '../stores/theme.js';

  interface Props {
    breadcrumbs?: { label: string; active?: boolean }[];
    onSearch?: () => void;
    class?: string;
  }

  let { breadcrumbs = [], onSearch, class: className = '' }: Props = $props();
</script>

<header class="sticky top-0 h-14 px-8 flex items-center gap-4 border-b border-border bg-surface shadow-[0_1px_3px_rgba(0,0,0,0.04)] z-[var(--z-sticky)] {className}">
  <div class="flex items-center gap-2 flex-1">
    {#each breadcrumbs as crumb, i}
      {#if i > 0}
        <ChevronRight size={14} class="text-fg-muted" />
      {/if}
      <span class="text-sm {crumb.active ? 'text-fg font-medium' : 'text-fg-muted'}">{crumb.label}</span>
    {/each}
  </div>

  {#if onSearch}
    <button
      onclick={onSearch}
      class="flex items-center gap-1.5 bg-base border border-border rounded-lg px-3 py-1.5 w-60 text-left hover:border-primary-300 transition-colors"
    >
      <Search size={14} class="text-fg-muted" />
      <span class="text-[13px] text-fg-muted flex-1">Search...</span>
      <kbd class="text-[11px] text-fg-muted bg-elevated px-1.5 py-0.5 rounded border border-border">⌘K</kbd>
    </button>
  {/if}

  <button
    onclick={() => theme.toggle()}
    class="w-8 h-8 rounded-lg border border-border bg-base flex items-center justify-center text-fg-muted hover:text-fg hover:border-primary-300 transition-colors"
  >
    {#if $theme === 'dark'}
      <Moon size={16} />
    {:else}
      <Sun size={16} />
    {/if}
  </button>
</header>
