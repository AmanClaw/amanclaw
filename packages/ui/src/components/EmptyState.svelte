<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  interface Props {
    icon: Component;
    title: string;
    description: string;
    action?: Snippet;
  }

  let { icon: Icon, title, description, action }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-12 px-6 text-center">
  <div class="w-12 h-12 rounded-xl bg-[var(--color-primary-500-10)] flex items-center justify-center mb-4">
    <Icon size={24} class="text-primary-500" />
  </div>
  <h3 class="text-base font-semibold text-fg mb-1.5">{title}</h3>
  <p class="text-[13px] text-fg-muted mb-4">{description}</p>
  {#if action}
    {@render action()}
  {/if}
</div>
