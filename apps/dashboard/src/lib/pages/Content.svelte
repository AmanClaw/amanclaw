<script lang="ts">
  import { PageHeader, Card } from '@amanclaw/ui'
  import { FileText } from '@amanclaw/ui'
</script>

<PageHeader title="Content Management" />

<Card class="text-center py-8">
  <div class="flex flex-col items-center gap-3">
    <div class="w-12 h-12 rounded-xl bg-[var(--color-primary-500-10)] flex items-center justify-center">
      <FileText size={24} class="text-primary-500" />
    </div>
    <p class="text-fg-muted">Content management (doa, zakat rates, khutbah) will be available in a future update.</p>
  </div>
</Card>
