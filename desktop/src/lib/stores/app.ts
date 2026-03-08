import { writable } from 'svelte/store';

export const botStatus = writable({
	running: false,
	mode: 'local',
	communities: 0,
	users: 0,
	skills: 0,
});

export const currentPage = writable('dashboard');
