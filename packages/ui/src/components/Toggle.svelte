<script lang="ts">
  import { Switch } from 'bits-ui';

  interface Props {
    checked?: boolean;
    onCheckedChange?: (checked: boolean) => void;
    label?: string;
    disabled?: boolean;
  }

  let { checked = $bindable(false), onCheckedChange, label, disabled = false }: Props = $props();
</script>

<div class="flex items-center gap-2.5">
  <Switch.Root
    {checked}
    onCheckedChange={(v) => { checked = v; onCheckedChange?.(v); }}
    {disabled}
    class="w-10 h-[22px] rounded-full transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed
           {checked ? 'bg-primary-500' : 'bg-border'}"
  >
    <Switch.Thumb
      class="block w-[18px] h-[18px] bg-white rounded-full shadow-sm transition-transform
             {checked ? 'translate-x-5' : 'translate-x-0.5'}"
    />
  </Switch.Root>
  {#if label}
    <span class="text-sm text-fg">{label}</span>
  {/if}
</div>
