import { invoke } from '@tauri-apps/api/core';

export const api = {
	// First-run
	checkFirstRun: () => invoke('check_first_run') as Promise<boolean>,

	// Config
	getConfig: () => invoke('get_config'),
	saveConfig: (params: {
		llm_base_url: string;
		llm_model: string;
		llm_api_key: string;
		max_tokens?: number;
		temperature?: number;
		rate_limit?: number;
		telegram_token?: string;
		discord_token?: string;
		slack_bot_token?: string;
		slack_app_token?: string;
	}) => invoke('save_config', params),

	// Engine lifecycle
	startEngine: () => invoke('start_engine'),
	stopEngine: () => invoke('stop_engine'),
	restartEngine: () => invoke('restart_engine'),

	// Status & data
	getStatus: () => invoke('get_status'),
	getCommunities: () => invoke('get_communities'),
	getSkills: () => invoke('get_skills'),
	getUsers: () => invoke('get_users'),
	getMode: () => invoke('get_mode'),
	setMode: (mode: string, url?: string, token?: string) =>
		invoke('set_mode', { mode, url, token }),
	getDataDir: () => invoke('get_data_dir') as Promise<string>,
};
