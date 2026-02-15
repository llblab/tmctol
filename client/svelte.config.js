import adapter from '@sveltejs/adapter-static';
import { fileURLToPath } from 'url';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			fallback: 'index.html'
		}),
		alias: {
			'$simulator': fileURLToPath(new URL('../simulator', import.meta.url))
		}
	}
};

export default config;
