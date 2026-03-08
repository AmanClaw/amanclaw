import { invoke } from '@tauri-apps/api/core';

export const api = {
	getStatus: () => invoke('get_status'),
	getCommunities: () => invoke('get_communities'),
	getSkills: () => invoke('get_skills'),
	getUsers: () => invoke('get_users'),
	getMode: () => invoke('get_mode'),
	setMode: (mode: string, url?: string, token?: string) =>
		invoke('set_mode', { mode, url, token }),
};
