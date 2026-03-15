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
  class="h-screen bg-surface border-r border-border flex flex-col z-[var(--z-sidebar)] shadow-[1px_0_3px_rgba(0,0,0,0.04)]"
  style="width: {collapsed ? '64px' : '260px'}; transition: width var(--transition-normal);"
>
  <!-- Logo header -->
  <div class="px-5 pt-5 pb-4 flex items-center gap-3 border-b border-border mb-2 {collapsed ? 'justify-center px-3' : ''}">
    {#if logoUrl}
      <img src={logoUrl} alt="AmanClaw" class="shrink-0 object-contain" style="width: 32px; height: 32px;" />
    {:else}
      <div class="shrink-0 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center" style="width: 32px; height: 32px;">
        <span class="text-white text-sm font-bold">A</span>
      </div>
    {/if}
    {#if !collapsed}
      <div>
        <span class="text-sm font-bold text-fg tracking-tight">AmanClaw</span>
        <p class="text-[11px] text-fg-muted">Community Bot</p>
      </div>
    {/if}
    {#if headerSlot}
      {@render headerSlot()}
    {/if}
  </div>

  <!-- Navigation -->
  <nav class="flex-1 overflow-y-auto px-3 space-y-6">
    {#each groups as group}
      {#if !collapsed}
        <div>
          <p class="text-[10px] font-semibold uppercase tracking-widest text-fg-muted/70 px-3 mb-2">{group.label}</p>
          <div class="space-y-0.5">
            {#each group.items as item}
              {@const active = activePage === item.id}
              <button
                onclick={() => onNavigate(item.id)}
                class="w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-all text-left
                       {active ? 'bg-primary-100 text-primary-900 font-medium shadow-sm' : 'text-fg-secondary hover:bg-elevated hover:text-fg'}"
              >
                <item.icon size={18} class="{active ? 'text-primary-700' : 'text-fg-muted'} shrink-0" />
                <span class="text-[13px]">{item.label}</span>
                {#if item.badge}
                  <span class="ml-auto text-[10px] font-semibold bg-[var(--color-accent-500-15)] text-accent-700 px-1.5 py-0.5 rounded">
                    {item.badge}
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <div class="space-y-0.5">
          {#each group.items as item}
            {@const active = activePage === item.id}
            <button
              onclick={() => onNavigate(item.id)}
              class="w-full flex items-center justify-center p-2.5 rounded-lg transition-all
                     {active ? 'bg-primary-100 text-primary-900 shadow-sm' : 'text-fg-muted hover:bg-elevated hover:text-fg'}"
            >
              <item.icon size={18} />
            </button>
          {/each}
        </div>
      {/if}
    {/each}
  </nav>

  <!-- Collapse toggle -->
  {#if onToggleCollapse}
    <button
      onclick={onToggleCollapse}
      class="mx-3 mb-3 p-2 rounded-lg text-fg-muted hover:bg-elevated hover:text-fg transition-colors
             {collapsed ? 'self-center' : 'self-end'}"
    >
      <ChevronRight size={16} class="transition-transform {collapsed ? '' : 'rotate-180'}" />
    </button>
  {/if}

  <!-- User profile -->
  {#if userName || onLogout}
    <div class="px-4 py-4 border-t border-border flex items-center gap-3 {collapsed ? 'justify-center px-3' : ''}">
      <div class="w-8 h-8 rounded-full bg-gradient-to-br from-accent-500 to-accent-700 flex items-center justify-center shrink-0">
        <span class="text-xs font-bold text-[#1a0f00]">{userInitials}</span>
      </div>
      {#if !collapsed}
        <span class="text-sm font-medium text-fg flex-1">{userName}</span>
        {#if onLogout}
          <button onclick={onLogout} class="text-fg-muted hover:text-fg transition-colors">
            <LogOut size={16} />
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</aside>
