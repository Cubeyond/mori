/** Typed frontend module for browser-side Mori behavior. */
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
export default defineConfig({
  build: {
    rolldownOptions: {
      output: {
        banner:
          '/*! Mori distribution. Copyright (c) 2026 Cubeyond. Mori-authored code: AGPL-3.0-or-later; see /LICENSE.txt and /NOTICE.txt. Third-party terms: /THIRD-PARTY-NOTICES.json. */',
      },
    },
  },
  plugins: [
    tailwindcss(),
    sveltekit({
      adapter: adapter({ fallback: 'index.html' }),
      preprocess: vitePreprocess(),
    }),
  ],
  worker: { format: 'es' },
  server: { proxy: { '/v1': 'http://127.0.0.1:3000' } },
});
