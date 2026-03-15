<script lang="ts">
  import type { Component } from 'svelte';
  import { MoreHorizontal } from 'lucide-svelte';

  interface NavItem { id: string; label: string; icon: Component; }

  interface Props {
    items: NavItem[];
    activePage: string;
    onNavigate: (id: string) => void;
    moreItems?: NavItem[];
  }

  let { items, activePage, onNavigate, moreItems }: Props = $props();
  let showMore = $state(false);
</script>

<nav class="md:hidden fixed bottom-0 left-0 right-0 bg-surface border-t border-border px-3 py-2 flex justify-around z-[var(--z-sticky)]">
  {#each items as item}
    {@const active = activePage === item.id}
    <button
      onclick={() => onNavigate(item.id)}
      class="flex flex-col items-center gap-0.5 px-2 py-1 {active ? 'text-primary-500' : 'text-fg-muted'}"
    >
      <item.icon size={20} />
      <span class="text-[10px] font-medium">{item.label}</span>
    </button>
  {/each}
  {#if moreItems && moreItems.length > 0}
    <button
      onclick={() => { showMore = !showMore; }}
      class="flex flex-col items-center gap-0.5 px-2 py-1 text-fg-muted"
    >
      <MoreHorizontal size={20} />
      <span class="text-[10px] font-medium">More</span>
    </button>
  {/if}
</nav>

{#if showMore && moreItems}
  <div class="md:hidden fixed inset-0 bg-[var(--color-base-80)] z-[var(--z-modal-backdrop)]" onclick={() => showMore = false}></div>
  <div class="md:hidden fixed bottom-0 left-0 right-0 bg-surface border-t border-border rounded-t-2xl p-4 z-[var(--z-modal)]">
    <div class="w-10 h-1 bg-border rounded-full mx-auto mb-4"></div>
    <div class="grid grid-cols-4 gap-3">
      {#each moreItems as item}
        <button
          onclick={() => { onNavigate(item.id); showMore = false; }}
          class="flex flex-col items-center gap-1.5 p-3 rounded-xl hover:bg-[var(--color-elevated-50)] text-fg-secondary"
        >
          <item.icon size={20} />
          <span class="text-[11px] font-medium">{item.label}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}
