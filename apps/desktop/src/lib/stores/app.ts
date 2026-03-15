import { writable } from 'svelte/store';

export const botStatus = writable({
	engine_status: 'stopped' as 'stopped' | 'starting' | 'running' | 'error',
	bot_running: false,
	mode: 'local',
	uptime_seconds: 0,
	error: null as string | null,
	communities: 0,
	users: 0,
	skills: 0,
});

export const currentPage = writable('dashboard');
export const isFirstRun = writable(false);
