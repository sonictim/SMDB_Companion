export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set(["favicon.png","svelte.svg","tauri.svg","vite.svg"]),
	mimeTypes: {".png":"image/png",".svg":"image/svg+xml"},
	_: {
		client: {start:"_app/immutable/entry/start.DCJqlKJT.js",app:"_app/immutable/entry/app.BxkZxaUX.js",imports:["_app/immutable/entry/start.DCJqlKJT.js","_app/immutable/chunks/C__v_t7C.js","_app/immutable/chunks/4B8pOMzx.js","_app/immutable/chunks/DuSSP3kn.js","_app/immutable/entry/app.BxkZxaUX.js","_app/immutable/chunks/4B8pOMzx.js","_app/immutable/chunks/saORRO85.js","_app/immutable/chunks/mBltkime.js","_app/immutable/chunks/DGF-fiXx.js","_app/immutable/chunks/DuSSP3kn.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js'))
		],
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			},
			{
				id: "/preferences",
				pattern: /^\/preferences\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
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
