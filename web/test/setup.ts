import '@testing-library/jest-dom/vitest';
import { readFileSync } from 'node:fs';
// Paths are relative to web/ (vitest runs there; jsdom gives import.meta.url no file scheme).
import { initSync } from '../src/generated/wasm/whichride_wasm.js';

// The page loads the WASM over fetch; tests load the same file from disk. init() is a no-op after this.
initSync({ module: readFileSync('src/generated/wasm/whichride_wasm_bg.wasm') });

// jsdom may lack the modal API; the browser's behaviour that matters here is `open`.
if (typeof HTMLDialogElement !== 'undefined' && !HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.open = false;
    this.dispatchEvent(new Event('close'));
  };
}
