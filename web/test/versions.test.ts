// @vitest-environment node
import { readFileSync } from 'node:fs';
import { lockedVersion } from '../scripts/generate.mjs';

const root = (path: string) => readFileSync(`../${path}`, 'utf8');

test('lockedVersion reads Cargo.lock entries', () => {
  const lock = '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.129"\nsource = "x"\n';
  expect(lockedVersion(lock, 'wasm-bindgen')).toBe('0.2.129');
  expect(lockedVersion(lock, 'wasm-bindgen-macro')).toBeNull();
  expect(lockedVersion(root('Cargo.lock'), 'wasm-bindgen')).toMatch(/^\d+\.\d+\.\d+$/);
});
