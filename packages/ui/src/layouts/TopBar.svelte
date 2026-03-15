<script lang="ts">
  import { Settings, Moon, Sun, ChevronRight } from 'lucide-svelte';
  import { theme } from '../stores/theme.js';

  interface Props {
    breadcrumbs?: { label: string; active?: boolean }[];
    onSearch?: () => void;
    onSettings?: () => void;
    class?: string;
  }

  let { breadcrumbs = [], onSearch, onSettings, class: className = '' }: Props = $props();
</script>

<header class="sticky top-0 h-16 px-8 flex items-center gap-4 bg-base border-b border-border/50 z-[var(--z-sticky)] {className}">
  <div class="flex items-center gap-2 flex-1">
    {#each breadcrumbs as crumb, i}
      {#if i > 0}
        <ChevronRight size={14} class="text-fg-muted" />
      {/if}
      <span class="text-sm {crumb.active ? 'text-fg font-medium' : 'text-fg-muted'}">{crumb.label}</span>
    {/each}
  </div>

  <button
    onclick={() => theme.toggle()}
    class="w-8 h-8 rounded-lg flex items-center justify-center text-fg-muted hover:text-fg transition-colors"
  >
    {#if $theme === 'dark'}
      <Moon size={18} />
    {:else}
      <Sun size={18} />
    {/if}
  </button>

  {#if onSettings}
    <button
      onclick={onSettings}
      class="w-8 h-8 rounded-lg flex items-center justify-center text-fg-muted hover:text-fg transition-colors"
    >
      <Settings size={18} />
    </button>
  {/if}
</header>
