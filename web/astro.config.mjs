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
  // No Markdown here; Shiki's inline styles would clash with the CSP.
  markdown: { syntaxHighlight: false },
  security: {
    csp: {
      directives: [
        "default-src 'self'",
        "img-src 'self'",
        "font-src 'self'",
        "connect-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
      ],
      // The core runs as WebAssembly; hashes for Astro's own inline scripts are added by Astro.
      scriptDirective: { resources: ["'self'", "'wasm-unsafe-eval'"] },
    },
  },
});
