// Cloud Run sends files as they are: write .br and .gz next to each compressible file in dist/,
// which the server's ServeDir picks by Accept-Encoding.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { extname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

export const COMPRESSIBLE = new Set(['.html', '.css', '.js', '.mjs', '.json', '.svg', '.xml', '.txt', '.wasm', '.ttf']);
export const MIN_BYTES = 1024;

export function compressDir(dir) {
  let count = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (!entry.isFile() || !COMPRESSIBLE.has(extname(entry.name))) continue;
    const path = join(entry.parentPath, entry.name);
    const data = readFileSync(path);
    if (data.length < MIN_BYTES) continue;
    writeFileSync(`${path}.br`, brotliCompressSync(data, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: data.length },
    }));
    writeFileSync(`${path}.gz`, gzipSync(data, { level: 9 }));
    count += 1;
  }
  return count;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const dist = fileURLToPath(new URL('../dist/', import.meta.url));
  console.log(`compressed ${compressDir(dist)} files`);
}
