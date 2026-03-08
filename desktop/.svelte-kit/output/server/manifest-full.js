export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set([".gitkeep"]),
	mimeTypes: {},
	_: {
		client: {start:"_app/immutable/entry/start.BlaYA2Wu.js",app:"_app/immutable/entry/app.Cefg1wx3.js",imports:["_app/immutable/entry/start.BlaYA2Wu.js","_app/immutable/chunks/jhvywJ_D.js","_app/immutable/chunks/DQ-cvAFX.js","_app/immutable/chunks/B2xRBW8J.js","_app/immutable/entry/app.Cefg1wx3.js","_app/immutable/chunks/DQ-cvAFX.js","_app/immutable/chunks/DyTrK3wT.js","_app/immutable/chunks/B2xRBW8J.js","_app/immutable/chunks/DwfbzHCR.js","_app/immutable/chunks/s748XA2O.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
