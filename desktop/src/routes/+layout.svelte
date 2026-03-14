<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { isFirstRun } from '$lib/stores/app';
	let { children } = $props();
</script>

{#if $isFirstRun}
	<main class="h-screen overflow-y-auto bg-white dark:bg-gray-900">
		{@render children()}
	</main>
{:else}
	<div class="flex h-screen bg-white dark:bg-gray-900 select-none">
		<Sidebar />
		<main class="flex-1 overflow-y-auto">
			{@render children()}
		</main>
	</div>
{/if}
