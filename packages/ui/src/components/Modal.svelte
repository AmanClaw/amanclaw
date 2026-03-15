<script lang="ts">
  import { Dialog } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { X } from 'lucide-svelte';

  interface Props {
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
    title: string;
    description?: string;
    children: Snippet;
    footer?: Snippet;
  }

  let { open = $bindable(false), onOpenChange, title, description, children, footer }: Props = $props();
</script>

<Dialog.Root bind:open {onOpenChange}>
  <Dialog.Overlay class="fixed inset-0 bg-[var(--color-base-80)] backdrop-blur-sm z-[var(--z-modal-backdrop)]" />
  <Dialog.Content
    class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg
           bg-surface border border-border rounded-2xl shadow-2xl z-[var(--z-modal)] p-6"
  >
    <div class="flex items-start justify-between mb-4">
      <div>
        <Dialog.Title class="text-xl font-semibold text-fg">{title}</Dialog.Title>
        {#if description}
          <Dialog.Description class="text-[13px] text-fg-muted mt-1">{description}</Dialog.Description>
        {/if}
      </div>
      <Dialog.Close class="p-1 rounded-lg hover:bg-elevated text-fg-muted hover:text-fg transition-colors">
        <X size={18} />
      </Dialog.Close>
    </div>
    <div>{@render children()}</div>
    {#if footer}
      <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-border">{@render footer()}</div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
