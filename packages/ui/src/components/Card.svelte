<script lang="ts">
  import type { Snippet } from 'svelte';
  let { class: className = '', children }: { class?: string; children: Snippet } = $props();
</script>

<div class="bg-surface border border-border rounded-xl p-5 {className}">
  {@render children()}
</div>
