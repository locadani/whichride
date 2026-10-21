import { defineConfig } from 'vitest/config';

// Checks the built site (npm run build first).
export default defineConfig({ test: { include: ['test-dist/**/*.test.ts'], environment: 'node', globals: true } });
