<script lang="ts">
  import { Tooltip as BitsTooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';

  interface Props { text: string; children: Snippet; }
  let { text, children }: Props = $props();
</script>

<BitsTooltip.Root openDelay={200}>
  <BitsTooltip.Trigger>
    {@render children()}
  </BitsTooltip.Trigger>
  <BitsTooltip.Content class="bg-elevated border border-border text-xs text-fg px-2.5 py-1.5 rounded-md shadow-lg z-[var(--z-tooltip)]">
    {text}
  </BitsTooltip.Content>
</BitsTooltip.Root>
