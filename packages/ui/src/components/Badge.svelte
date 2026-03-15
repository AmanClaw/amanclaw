<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'success' | 'warning' | 'error' | 'info' | 'accent' | 'muted' | 'telegram' | 'discord' | 'whatsapp' | 'slack';
    children: Snippet;
  }

  let { variant = 'muted', children }: Props = $props();

  const styles: Record<string, string> = {
    success: 'bg-[var(--color-success-15)] text-green-400',
    warning: 'bg-[var(--color-warning-15)] text-amber-400',
    error: 'bg-[var(--color-error-15)] text-red-400',
    info: 'bg-[var(--color-info-15)] text-blue-400',
    accent: 'bg-[var(--color-accent-500-15)] text-accent-500',
    muted: 'bg-[var(--color-elevated-50)] text-fg-secondary',
    telegram: 'bg-[var(--color-primary-500-15)] text-primary-500',
    discord: 'bg-violet-500/15 text-violet-400',
    whatsapp: 'bg-[var(--color-success-15)] text-green-400',
    slack: 'bg-[var(--color-warning-15)] text-amber-400',
  };
</script>

<span class="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-medium {styles[variant]}">
  {@render children()}
</span>
