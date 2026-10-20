import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { checkContrast, floor2 } from './lib/contrast.mjs';
import { loadModel } from './lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const model = await loadModel(here('tokens.json'));
const rules = JSON.parse(await readFile(here('contrast.json'), 'utf8'));
const results = checkContrast(model, rules);
for (const r of results) {
  const line = `${r.ok ? 'ok  ' : 'FAIL'} ${r.scope.padEnd(5)} ${r.fg.padEnd(17)} on ${r.bg.padEnd(16)} ${floor2(r.ratio).padStart(5)}:1 (needs ${r.min}, ${r.kind})`;
  console.log(line);
}
const failed = results.filter((r) => !r.ok).length;
if (failed) {
  console.error(`${failed} colour pair(s) below WCAG 2.2 AA`);
  process.exitCode = 1;
}
