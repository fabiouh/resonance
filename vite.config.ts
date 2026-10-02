import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  envPrefix: [],
  plugins: [svelte()],
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  clearScreen: false,
  build: { target: 'es2022' },
});
