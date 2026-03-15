<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    label?: string;
    error?: string;
    leadingIcon?: Snippet;
  }

  let { label, error, leadingIcon, class: className = '', ...rest }: Props = $props();
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <label class="text-[13px] font-medium {error ? 'text-error' : 'text-fg-secondary'}">{label}</label>
  {/if}
  <div class="relative flex items-center">
    {#if leadingIcon}
      <div class="absolute left-3 text-fg-muted">{@render leadingIcon()}</div>
    {/if}
    <input
      class="w-full bg-elevated border rounded-lg px-3.5 py-2.5 text-sm text-fg placeholder:text-fg-muted
             outline-none {error ? 'border-error focus:border-error focus:ring-[3px] focus:ring-[var(--color-error-15)]' : 'border-border focus:border-primary-500 focus:ring-[3px] focus:ring-[var(--color-primary-500-10)]'}
             {leadingIcon ? 'pl-10' : ''} {className}"
      {...rest}
    />
  </div>
  {#if error}
    <p class="text-xs text-error">{error}</p>
  {/if}
</div>
