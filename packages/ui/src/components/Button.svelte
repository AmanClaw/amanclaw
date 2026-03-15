<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'primary' | 'secondary' | 'ghost' | 'destructive' | 'accent';
    size?: 'default' | 'sm';
    children: Snippet;
  }

  let { variant = 'primary', size = 'default', children, class: className = '', ...rest }: Props = $props();

  const base = 'inline-flex items-center justify-center gap-1.5 font-medium rounded-lg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed';
  const variants: Record<string, string> = {
    primary: 'bg-gradient-to-br from-primary-500 to-primary-700 text-white shadow-sm hover:from-primary-400 hover:to-primary-600',
    secondary: 'bg-[var(--color-elevated-60)] text-fg border border-border hover:bg-elevated',
    ghost: 'text-fg-secondary hover:text-fg hover:bg-[var(--color-elevated-50)]',
    destructive: 'bg-[var(--color-error-15)] text-error border border-[var(--color-error-20)] hover:bg-[var(--color-error-20)]',
    accent: 'bg-gradient-to-br from-accent-500 to-accent-700 text-[#1a0f00] font-semibold shadow-sm hover:from-accent-300 hover:to-accent-500',
  };
  const sizes: Record<string, string> = {
    default: 'px-4 py-2 text-[13px]',
    sm: 'px-3 py-1 text-xs',
  };
</script>

<button class="{base} {variants[variant]} {sizes[size]} {className}" {...rest}>
  {@render children()}
</button>
