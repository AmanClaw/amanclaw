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

<header class="sticky top-0 h-12 px-6 flex items-center gap-4 border-b border-border bg-[var(--color-base-80)] backdrop-blur-sm z-[var(--z-sticky)] {className}">
  <div class="flex items-center gap-1.5 flex-1">
    {#each breadcrumbs as crumb, i}
      {#if i > 0}
        <ChevronRight size={12} class="text-fg-muted" />
      {/if}
      <span class="text-[13px] {crumb.active ? 'text-fg font-medium' : 'text-fg-muted'}">{crumb.label}</span>
    {/each}
  </div>

  {#if onSearch}
    <button
      onclick={onSearch}
      class="flex items-center gap-1.5 bg-elevated border border-border rounded-lg px-3 py-1.5 w-60 text-left hover:border-[var(--color-primary-500-10)] transition-colors"
    >
      <Search size={14} class="text-fg-muted" />
      <span class="text-[13px] text-fg-muted flex-1">Search...</span>
      <kbd class="text-[11px] text-fg-muted bg-base px-1.5 py-0.5 rounded">⌘K</kbd>
    </button>
  {/if}

  <button
    onclick={() => theme.toggle()}
    class="w-8 h-8 rounded-lg bg-[var(--color-elevated-50)] flex items-center justify-center text-fg-muted hover:text-fg transition-colors"
  >
    {#if $theme === 'dark'}
      <Moon size={16} />
    {:else}
      <Sun size={16} />
    {/if}
  </button>
</header>
