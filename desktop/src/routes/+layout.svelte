<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<div class="flex h-screen bg-white select-none">
	{@render children()}
</div>
