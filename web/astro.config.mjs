import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

/** The Cloud Run URL; a domain later is one setting (WHICHRIDE_SITE_URL). */
export const DEFAULT_SITE = 'https://whichride-1034831276867.europe-west3.run.app';

export default defineConfig({
  site: process.env.WHICHRIDE_SITE_URL || DEFAULT_SITE,
  output: 'static',
  trailingSlash: 'always',
  build: { format: 'directory' },
  integrations: [svelte()],
});
