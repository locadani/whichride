// Builds the core for the browser and copies the seed catalogue into src/generated (git-ignored).
// Needs the wasm32-unknown-unknown target (rust-toolchain.toml) and the wasm-bindgen CLI at the
// version in Cargo.lock: cargo install wasm-bindgen-cli --version <it> --locked
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const OUT = fileURLToPath(new URL('../src/generated/', import.meta.url));

/** The version of a crate in Cargo.lock, or null. */
export function lockedVersion(lockText, name) {
  const match = lockText.match(new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`));
  return match ? match[1] : null;
}

function run(command, args, cwd) {
  execFileSync(command, args, { cwd, stdio: 'inherit' });
}

export function generate() {
  const wanted = lockedVersion(readFileSync(`${ROOT}Cargo.lock`, 'utf8'), 'wasm-bindgen');
  let installed = '';
  try {
    installed = execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim();
  } catch {
    // reported below
  }
  if (installed !== `wasm-bindgen ${wanted}`) {
    throw new Error(
      `wasm-bindgen CLI ${wanted} is needed (found: ${installed || 'none'}).\n` +
        `Run: cargo install wasm-bindgen-cli --version ${wanted} --locked`,
    );
  }
  run('cargo', ['build', '--profile', 'web', '--locked', '-p', 'whichride-wasm', '--target', 'wasm32-unknown-unknown'], ROOT);
  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(`${OUT}wasm`, { recursive: true });
  run('wasm-bindgen', ['--target', 'web', '--out-dir', `${OUT}wasm`, `${ROOT}target/wasm32-unknown-unknown/web/whichride_wasm.wasm`], ROOT);
  copyFileSync(`${ROOT}crates/server/catalogue.json`, `${OUT}catalogue.json`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) generate();
