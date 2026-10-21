/// <reference types="vitest/config" />
import { getViteConfig } from 'astro/config';
import { svelteTesting } from '@testing-library/svelte/vite';

export default getViteConfig({
  plugins: [svelteTesting()],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['test/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
  },
});
