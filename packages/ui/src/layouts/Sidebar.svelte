<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ChevronRight, LogOut } from 'lucide-svelte';

  interface NavItem {
    id: string;
    label: string;
    icon: Component;
    badge?: string;
  }

  interface NavGroup {
    label: string;
    items: NavItem[];
  }

  interface Props {
    groups: NavGroup[];
    activePage: string;
    onNavigate: (id: string) => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
    userName?: string;
    userInitials?: string;
    onLogout?: () => void;
    logoUrl?: string;
    headerSlot?: Snippet;
  }

  let {
    groups, activePage, onNavigate, collapsed = false, onToggleCollapse,
    userName, userInitials = 'U', onLogout, logoUrl, headerSlot,
  }: Props = $props();
</script>

<aside
  class="h-screen bg-surface border-r border-border flex flex-col z-[var(--z-sidebar)]"
  style="width: {collapsed ? '64px' : '240px'}; transition: width var(--transition-normal);"
>
  <div class="px-3 pt-4 pb-5 flex items-center gap-2.5 {collapsed ? 'justify-center' : ''}">
    {#if logoUrl}
      <img src={logoUrl} alt="AmanClaw" class="w-8 h-8 rounded-lg object-cover shrink-0" />
    {:else}
      <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shrink-0">
        <span class="text-white text-xs font-bold">A</span>
      </div>
    {/if}
    {#if !collapsed}
      <div>
        <span class="text-sm font-semibold text-fg">AmanClaw</span>
        <p class="text-[10px] text-fg-muted">Community Bot</p>
      </div>
    {/if}
    {#if headerSlot}
      {@render headerSlot()}
    {/if}
  </div>

  <nav class="flex-1 overflow-y-auto px-2 space-y-4">
    {#each groups as group}
      {#if !collapsed}
        <p class="text-xs font-semibold uppercase tracking-wide text-fg-muted px-2.5">{group.label}</p>
      {/if}
      <div class="space-y-0.5">
        {#each group.items as item}
          {@const active = activePage === item.id}
          <button
            onclick={() => onNavigate(item.id)}
            class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg transition-colors text-left
                   {active ? 'bg-[var(--color-primary-500-10)] text-primary-500' : 'text-fg-secondary hover:bg-[var(--color-elevated-50)] hover:text-fg'}
                   {collapsed ? 'justify-center' : ''}"
          >
            <item.icon size={16} class="{active ? 'text-primary-500' : 'text-fg-muted'} shrink-0" />
            {#if !collapsed}
              <span class="text-[13px] font-medium">{item.label}</span>
              {#if item.badge}
                <span class="ml-auto text-[10px] font-semibold bg-[var(--color-accent-500-15)] text-accent-500 px-1.5 py-0.5 rounded">
                  {item.badge}
                </span>
              {/if}
            {/if}
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  {#if onToggleCollapse}
    <button
      onclick={onToggleCollapse}
      class="mx-2 mb-2 p-2 rounded-lg text-fg-muted hover:bg-[var(--color-elevated-50)] hover:text-fg transition-colors
             {collapsed ? 'self-center' : 'self-end'}"
    >
      <ChevronRight size={16} class="transition-transform {collapsed ? '' : 'rotate-180'}" />
    </button>
  {/if}

  {#if userName || onLogout}
    <div class="px-3 py-3 border-t border-border flex items-center gap-2.5 {collapsed ? 'justify-center' : ''}">
      <div class="w-7 h-7 rounded-full bg-gradient-to-br from-accent-500 to-accent-700 flex items-center justify-center shrink-0">
        <span class="text-[11px] font-bold text-[#1a0f00]">{userInitials}</span>
      </div>
      {#if !collapsed}
        <span class="text-xs font-medium text-fg flex-1">{userName}</span>
        {#if onLogout}
          <button onclick={onLogout} class="text-fg-muted hover:text-fg transition-colors">
            <LogOut size={14} />
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</aside>
