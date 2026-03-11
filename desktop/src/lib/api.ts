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

	// Agents
	listAgents: () => invoke('list_agents'),
	saveAgent: (params: {
		id: string; name: string; systemPrompt: string;
		soulFile?: string; allowedSkills: string[]; memoryNamespace: string;
	}) => invoke('save_agent', params),
	deleteAgent: (id: string) => invoke('delete_agent', { id }),
	loadSoulFile: (filename: string) => invoke('load_soul_file', { filename }) as Promise<string>,
	saveSoulFile: (filename: string, content: string) => invoke('save_soul_file', { filename, content }),
	previewSoul: (filename: string) => invoke('preview_soul', { filename }),
	getRoutingRules: () => invoke('get_routing_rules'),
	saveRoutingRules: (defaultAgent: string, rules: any[]) =>
		invoke('save_routing_rules', { defaultAgent, rules }),

	// Cron Jobs
	listCronJobs: () => invoke('list_cron_jobs'),
	saveCronJob: (id: string, job: any) => invoke('save_cron_job', { id, job }),
	deleteCronJob: (id: string) => invoke('delete_cron_job', { id }),
	getCronHistory: () => invoke('get_cron_history'),

	// Webhooks
	listWebhookEndpoints: () => invoke('list_webhook_endpoints'),
	saveWebhookEndpoint: (id: string, endpoint: any) => invoke('save_webhook_endpoint', { id, endpoint }),
	deleteWebhookEndpoint: (id: string) => invoke('delete_webhook_endpoint', { id }),
	getWebhookHistory: () => invoke('get_webhook_history'),

	// Gateway
	getGatewayConfig: () => invoke('get_gateway_config'),
	saveGatewayConfig: (params: {
		enabled: boolean; heartbeatIntervalSecs: number;
		maxConnections: number; staleSessionTimeoutSecs: number;
	}) => invoke('save_gateway_config', params),
	getGatewayStatus: () => invoke('get_gateway_status'),

	// Sub-Agents
	getSubagentConfig: () => invoke('get_subagent_config'),
	saveSubagentConfig: (params: {
		enabled: boolean; maxPerSession: number; maxGlobal: number;
		maxDepth: number; defaultTimeoutSecs: number;
	}) => invoke('save_subagent_config', params),
	listSubagents: (sessionFilter?: string) => invoke('list_subagents', { sessionFilter }),
	cancelSubagent: (id: string) => invoke('cancel_subagent', { id }) as Promise<boolean>,
	cancelAllSubagents: (session: string) => invoke('cancel_all_subagents', { session }) as Promise<number>,

	// Marketplace / Registry
	registryListInstalled: () => invoke('registry_list_installed'),
	registryInstallFromPath: (path: string) => invoke('registry_install_from_path', { path }),
	registryUninstall: (name: string) => invoke('registry_uninstall', { name }) as Promise<boolean>,
	registrySearchInstalled: (query: string) => invoke('registry_search_installed', { query }),
	marketplaceBrowse: (query?: string) => invoke('marketplace_browse', { query }),

	// Knowledge Bases
	getEmbeddingConfig: () => invoke('get_embedding_config'),
	saveEmbeddingConfig: (params: { baseUrl: string; model: string; apiKey?: string }) =>
		invoke('save_embedding_config', params),
	getVectorConfig: () => invoke('get_vector_config'),
	saveVectorConfig: (params: { backend: string; qdrantUrl?: string }) =>
		invoke('save_vector_config', params),
	listKnowledgeBases: () => invoke('list_knowledge_bases'),
	saveKnowledgeBase: (name: string, collection: string, source: string) =>
		invoke('save_knowledge_base', { name, collection, source }),
	deleteKnowledgeBase: (name: string) => invoke('delete_knowledge_base', { name }),

	// Communities CRUD
	createCommunity: (params: {
		name: string; platform: string; platformGroupId: string;
		zone: string; language: string; enabledSkills: string[];
	}) => invoke('create_community', params),
	updateCommunity: (params: {
		id: string; name: string; zone: string; language: string; enabledSkills: string[];
	}) => invoke('update_community', params),
	deleteCommunity: (id: string) => invoke('delete_community', { id }),

	// Content
	getDoaCollection: (category?: string) => invoke('get_doa_collection', { category }),
	searchDoa: (query: string) => invoke('search_doa', { query }),
	getZakatRates: () => invoke('get_zakat_rates'),
	getLatestKhutbah: () => invoke('get_latest_khutbah'),
};
