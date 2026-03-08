<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/Sidebar.svelte';
	let { children } = $props();
</script>

<div class="flex h-screen bg-white select-none">
	<Sidebar />
	<main class="flex-1 overflow-y-auto">
		{@render children()}
	</main>
</div>
