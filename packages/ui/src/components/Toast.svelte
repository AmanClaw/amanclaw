<script lang="ts">
  import type { Component } from 'svelte';
  import { X, Check, AlertCircle, Info } from 'lucide-svelte';

  interface Props {
    variant?: 'success' | 'error' | 'warning' | 'info';
    title: string;
    description?: string;
    open?: boolean;
    onClose?: () => void;
    duration?: number;
  }

  let { variant = 'info', title, description, open = $bindable(true), onClose, duration = 5000 }: Props = $props();

  const icons: Record<string, Component> = { success: Check, error: AlertCircle, warning: AlertCircle, info: Info };
  const colors: Record<string, string> = {
    success: 'border-l-success text-success',
    error: 'border-l-error text-error',
    warning: 'border-l-warning text-warning',
    info: 'border-l-info text-info',
  };
  const Icon = $derived(icons[variant]);

  $effect(() => {
    if (open && duration > 0) {
      const timer = setTimeout(() => { open = false; onClose?.(); }, duration);
      return () => clearTimeout(timer);
    }
  });
</script>

{#if open}
  <div class="fixed bottom-4 right-4 z-[var(--z-toast)] bg-surface border border-border border-l-4 {colors[variant]} rounded-lg shadow-xl p-4 min-w-[320px] max-w-[420px] flex gap-3 items-start">
    <Icon size={18} />
    <div class="flex-1">
      <p class="text-sm font-medium text-fg">{title}</p>
      {#if description}
        <p class="text-xs text-fg-muted mt-1">{description}</p>
      {/if}
    </div>
    <button onclick={() => { open = false; onClose?.(); }} class="text-fg-muted hover:text-fg transition-colors">
      <X size={14} />
    </button>
  </div>
{/if}
