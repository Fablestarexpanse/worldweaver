import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      // Tauri serves the built files directly — no SPA fallback needed
      fallback: 'index.html',
      pages: '../build',
      assets: '../build',
    }),
    // Disable SSR — Tauri UI is always client-side
    prerender: { entries: [] },
  },
};

export default config;
