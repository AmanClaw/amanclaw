<script lang="ts">
  import { Select as BitsSelect } from 'bits-ui';
  import { ChevronDown } from 'lucide-svelte';

  interface Option { value: string; label: string; }
  interface Props {
    options: Option[];
    value?: string;
    onValueChange?: (value: string) => void;
    placeholder?: string;
    label?: string;
  }

  let { options, value = $bindable(''), onValueChange, placeholder = 'Select...', label }: Props = $props();
  const selected = $derived(options.find(o => o.value === value));
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <span class="text-[13px] font-medium text-fg-secondary">{label}</span>
  {/if}
  <BitsSelect.Root {value} onValueChange={(v) => { value = v; onValueChange?.(v); }}>
    <BitsSelect.Trigger
      class="flex items-center justify-between w-full bg-elevated border border-border rounded-lg px-3.5 py-2.5 text-sm text-fg
             outline-none focus:border-primary-500 focus:ring-[3px] focus:ring-[var(--color-primary-500-10)] cursor-pointer"
    >
      <span class={selected ? '' : 'text-fg-muted'}>{selected?.label ?? placeholder}</span>
      <ChevronDown size={14} class="text-fg-muted" />
    </BitsSelect.Trigger>
    <BitsSelect.Content class="bg-elevated border border-border rounded-lg shadow-xl py-1 z-[var(--z-dropdown)]">
      {#each options as option}
        <BitsSelect.Item value={option.value} textValue={option.label}
          class="px-3 py-2 text-sm text-fg hover:bg-[var(--color-primary-500-10)] hover:text-primary-500 cursor-pointer transition-colors"
        >
          {option.label}
        </BitsSelect.Item>
      {/each}
    </BitsSelect.Content>
  </BitsSelect.Root>
</div>
