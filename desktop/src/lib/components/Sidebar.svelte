<script lang="ts">
	import { currentPage } from '$lib/stores/app';

	const pages = [
		{ id: 'dashboard', label: 'Dashboard', icon: '⊞' },
		{ id: 'communities', label: 'Communities', icon: '⊡' },
		{ id: 'skills', label: 'Skills', icon: '⚡' },
		{ id: 'users', label: 'Users', icon: '⊙' },
		{ id: 'content', label: 'Content', icon: '☰' },
		{ id: 'logs', label: 'Logs', icon: '▤' },
	];

	const bottomPages = [
		{ id: 'settings', label: 'Settings', icon: '⚙' },
	];
</script>

<aside class="w-56 h-screen bg-gray-50/80 backdrop-blur-xl border-r border-gray-200 flex flex-col justify-between p-3">
	<div>
		<div class="px-3 py-4 mb-2">
			<h1 class="text-sm font-semibold text-gray-900 tracking-tight">AmanClaw</h1>
		</div>
		<nav class="space-y-0.5">
			{#each pages as page}
				<button
					class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] transition-colors
						{$currentPage === page.id
							? 'bg-gray-200/80 text-gray-900 font-medium'
							: 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'}"
					onclick={() => currentPage.set(page.id)}
				>
					<span class="text-base leading-none">{page.icon}</span>
					{page.label}
				</button>
			{/each}
		</nav>
	</div>

	<div>
		<div class="border-t border-gray-200 pt-2 mb-2">
			{#each bottomPages as page}
				<button
					class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] text-gray-600 hover:bg-gray-100 hover:text-gray-900 transition-colors"
					onclick={() => currentPage.set(page.id)}
				>
					<span class="text-base leading-none">{page.icon}</span>
					{page.label}
				</button>
			{/each}
		</div>
		<div class="mx-2 p-2.5 bg-white rounded-lg border border-gray-200 shadow-sm">
			<div class="flex items-center gap-2">
				<span class="w-2 h-2 rounded-full bg-green-500"></span>
				<span class="text-[11px] font-medium text-gray-700">Bot Running</span>
			</div>
		</div>
	</div>
</aside>
