<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { PageHeader, Button, Card, EmptyState, Badge } from '@amanclaw/ui';

	// --- Embedding config ---
	let baseUrl = $state('');
	let model = $state('');
	let apiKey = $state('');
	let savingEmbedding = $state(false);

	// --- Vector config ---
	let backend = $state<'sqlite-vec' | 'qdrant'>('sqlite-vec');
	let qdrantUrl = $state('');
	let savingVector = $state(false);

	// --- Knowledge bases ---
	let knowledgeBases: any[] = $state([]);
	let loading = $state(true);
	let confirmDelete = $state<string | null>(null);
	let deleting = $state<string | null>(null);

	// --- Add KB form ---
	let showAddForm = $state(false);
	let newName = $state('');
	let newCollection = $state('');
	let newSource = $state('');
	let savingKb = $state(false);

	const isConfigured = $derived(() => baseUrl.trim() !== '' && model.trim() !== '');

	const autoCollection = $derived(() => {
		if (newCollection.trim()) return newCollection;
		return newName.trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');
	});

	// --- Actions ---
	async function loadEmbeddingConfig() {
		try {
			const data = await api.getEmbeddingConfig() as any;
			baseUrl = data.base_url || '';
			model = data.model || '';
			apiKey = data.api_key || '';
		} catch (_) {}
	}

	async function saveEmbeddingConfig() {
		savingEmbedding = true;
		try {
			await api.saveEmbeddingConfig({
				baseUrl: baseUrl.trim(),
				model: model.trim(),
				apiKey: apiKey.trim() || undefined,
			});
		} catch (_) {}
		savingEmbedding = false;
	}

	async function loadVectorConfig() {
		try {
			const data = await api.getVectorConfig() as any;
			backend = data.backend || 'sqlite-vec';
			qdrantUrl = data.qdrant_url || '';
		} catch (_) {}
	}

	async function saveVectorConfig() {
		savingVector = true;
		try {
			await api.saveVectorConfig({
				backend,
				qdrantUrl: backend === 'qdrant' ? qdrantUrl.trim() || undefined : undefined,
			});
		} catch (_) {}
		savingVector = false;
	}

	async function loadKnowledgeBases() {
		try {
			const data = await api.listKnowledgeBases() as any;
			knowledgeBases = data.knowledge_bases || data || [];
		} catch (_) {}
		loading = false;
	}

	async function addKnowledgeBase() {
		if (!newName.trim() || !newSource.trim()) return;
		savingKb = true;
		try {
			await api.saveKnowledgeBase(
				newName.trim(),
				autoCollection(),
				newSource.trim()
			);
			newName = '';
			newCollection = '';
			newSource = '';
			showAddForm = false;
			await loadKnowledgeBases();
		} catch (_) {}
		savingKb = false;
	}

	async function deleteKb(name: string) {
		deleting = name;
		try {
			await api.deleteKnowledgeBase(name);
			confirmDelete = null;
			await loadKnowledgeBases();
		} catch (_) {}
		deleting = null;
	}

	function resetAddForm() {
		newName = '';
		newCollection = '';
		newSource = '';
		showAddForm = false;
	}

	onMount(() => {
		loadEmbeddingConfig();
		loadVectorConfig();
		loadKnowledgeBases();
	});
</script>

<div class="max-w-4xl">
	<PageHeader title="Knowledge Bases" subtitle="Configure embeddings and manage RAG knowledge sources" />

	<!-- Embedding Configuration -->
	<div class="bg-base rounded-xl border border-border p-5 mb-6">
		<div class="flex items-center justify-between mb-4">
			<h3 class="text-sm font-medium text-fg">Embedding Configuration</h3>
			<span class="inline-flex items-center gap-1.5 text-[10px] font-medium
				{isConfigured() ? 'text-success' : 'text-fg-muted'}">
				<span class="w-1.5 h-1.5 rounded-full {isConfigured() ? 'bg-success' : 'bg-border'}"></span>
				{isConfigured() ? 'Configured' : 'Not configured'}
			</span>
		</div>

		<div class="space-y-4">
			<div class="grid grid-cols-2 gap-4">
				<div>
					<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Base URL</label>
					<input type="text" bind:value={baseUrl} placeholder="http://localhost:11434/v1"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>
				<div>
					<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Model</label>
					<input type="text" bind:value={model} placeholder="nomic-embed-text"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>
			</div>
			<div>
				<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">API Key <span class="normal-case text-fg-muted">(optional)</span></label>
				<input type="password" bind:value={apiKey} placeholder="sk-..."
					class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
			</div>
			<div>
				<button onclick={saveEmbeddingConfig} disabled={savingEmbedding}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
					{savingEmbedding ? 'Saving...' : 'Save Embedding Config'}
				</button>
			</div>
		</div>
	</div>

	<!-- Vector Backend Configuration -->
	<div class="bg-base rounded-xl border border-border p-5 mb-6">
		<h3 class="text-sm font-medium text-fg mb-4">Vector Backend</h3>

		<div class="space-y-4">
			<div>
				<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-2">Backend</label>
				<div class="flex gap-2">
					<button onclick={() => backend = 'sqlite-vec'}
						class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
							{backend === 'sqlite-vec' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
						sqlite-vec
					</button>
					<button onclick={() => backend = 'qdrant'}
						class="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors
							{backend === 'qdrant' ? 'bg-gradient-to-br from-primary-500 to-primary-700 text-white border-primary-500' : 'border-border text-fg-secondary hover:bg-[var(--color-elevated-50)]'}">
						Qdrant
					</button>
				</div>
			</div>

			{#if backend === 'qdrant'}
				<div>
					<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Qdrant URL</label>
					<input type="text" bind:value={qdrantUrl} placeholder="http://localhost:6334"
						class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>
			{/if}

			<div>
				<button onclick={saveVectorConfig} disabled={savingVector}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
					{savingVector ? 'Saving...' : 'Save Vector Config'}
				</button>
			</div>
		</div>
	</div>

	<!-- Knowledge Bases Table -->
	<div class="flex items-center justify-between mb-4">
		<h3 class="text-sm font-medium text-fg">Knowledge Bases</h3>
		{#if !showAddForm}
			<button onclick={() => showAddForm = true}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors">
				Add KB
			</button>
		{/if}
	</div>

	<!-- Add KB Form -->
	{#if showAddForm}
		<div class="bg-base rounded-xl border border-border p-5 mb-4">
			<h4 class="text-sm font-medium text-fg mb-4">Add Knowledge Base</h4>
			<div class="space-y-4">
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Name</label>
						<input type="text" bind:value={newName} placeholder="e.g. Company Docs"
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
					</div>
					<div>
						<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Collection <span class="normal-case text-fg-muted">(auto-suggested)</span></label>
						<input type="text" bind:value={newCollection} placeholder={autoCollection()}
							class="w-full px-3 py-2 text-sm border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
						{#if newName.trim() && !newCollection.trim()}
							<p class="text-[10px] text-fg-muted mt-1">Will use: {autoCollection()}</p>
						{/if}
					</div>
				</div>
				<div>
					<label class="block text-[11px] font-medium text-fg-muted uppercase tracking-wider mb-1">Source File Path</label>
					<input type="text" bind:value={newSource} placeholder="/path/to/documents.txt"
						class="w-full px-3 py-2 text-sm font-mono border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
				</div>
			</div>

			<div class="flex gap-2 mt-5">
				<button onclick={addKnowledgeBase} disabled={savingKb || !newName.trim() || !newSource.trim()}
					class="px-4 py-1.5 text-xs font-medium rounded-md bg-gradient-to-br from-primary-500 to-primary-700 text-white hover:from-primary-400 hover:to-primary-600 transition-colors disabled:opacity-50">
					{savingKb ? 'Saving...' : 'Add'}
				</button>
				<button onclick={resetAddForm}
					class="px-4 py-1.5 text-xs font-medium rounded-md border border-border text-fg-secondary hover:bg-[var(--color-elevated-50)] transition-colors">
					Cancel
				</button>
			</div>
		</div>
	{/if}

	{#if loading}
		<p class="text-sm text-fg-muted">Loading...</p>
	{:else if knowledgeBases.length === 0 && !showAddForm}
		<div class="text-center py-16 bg-base rounded-xl border border-border">
			<p class="text-sm text-fg-muted">No knowledge bases configured</p>
			<p class="text-xs text-fg-muted mt-1">Add document sources for RAG-powered responses</p>
		</div>
	{:else if knowledgeBases.length > 0}
		<div class="bg-base rounded-xl border border-border overflow-hidden">
			<table class="w-full text-xs">
				<thead>
					<tr class="border-b border-border bg-base">
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Name</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Collection</th>
						<th class="text-left px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Source</th>
						<th class="text-right px-4 py-2.5 text-[10px] font-medium text-fg-muted uppercase tracking-wider">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each knowledgeBases as kb}
						<tr class="border-b border-border hover:from-primary-400 hover:to-primary-600 transition-colors">
							<td class="px-4 py-2.5 text-fg font-medium">{kb.name}</td>
							<td class="px-4 py-2.5 font-mono text-fg-muted">{kb.collection}</td>
							<td class="px-4 py-2.5 font-mono text-fg-muted max-w-xs truncate" title={kb.source}>{kb.source}</td>
							<td class="px-4 py-2.5 text-right">
								{#if confirmDelete === kb.name}
									<span class="text-[10px] text-fg-muted mr-2">Delete?</span>
									<button onclick={() => deleteKb(kb.name)}
										disabled={deleting === kb.name}
										class="px-2 py-0.5 text-[10px] font-medium rounded border border-[var(--color-error-20)] text-error hover:bg-[var(--color-error-15)] mr-1 disabled:opacity-50">
										{deleting === kb.name ? '...' : 'Yes'}
									</button>
									<button onclick={() => confirmDelete = null}
										class="px-2 py-0.5 text-[10px] font-medium rounded border border-border text-fg-muted hover:bg-[var(--color-elevated-50)]">
										No
									</button>
								{:else}
									<button onclick={() => confirmDelete = kb.name}
										class="text-xs text-error hover:text-error font-medium">
										Delete
									</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
