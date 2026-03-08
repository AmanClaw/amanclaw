import { invoke } from '@tauri-apps/api/core';

export const api = {
	// First-run
	checkFirstRun: () => invoke('check_first_run') as Promise<boolean>,

	// Config
	getConfig: () => invoke('get_config'),
	saveConfig: (params: {
		llmBaseUrl: string;
		llmModel: string;
		llmApiKey: string;
		maxTokens?: number;
		temperature?: number;
		rateLimit?: number;
		telegramToken?: string;
		discordToken?: string;
		slackBotToken?: string;
		slackAppToken?: string;
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
	getLogs: () => invoke('get_logs') as Promise<any[]>,
	approveUser: (userId: string, platform: string) =>
		invoke('approve_user', { userId, platform }),
	blockUser: (userId: string, platform: string) =>
		invoke('block_user', { userId, platform }),

	// Skills management
	disableSkill: (name: string) => invoke('disable_skill', { name }),
	enableSkill: (name: string) => invoke('enable_skill', { name }),
	getDisabledSkills: () => invoke('get_disabled_skills') as Promise<string[]>,

	// MCP Servers
	getMcpServers: () => invoke('get_mcp_servers'),
	saveMcpServer: (params: {
		name: string;
		command?: string;
		args?: string[];
		env?: Record<string, string>;
		url?: string;
	}) => invoke('save_mcp_server', params),
	deleteMcpServer: (name: string) => invoke('delete_mcp_server', { name }),
};
