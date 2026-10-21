// The Rust core in WebAssembly, with the seed catalogue built in and the server's on top.
import init, { Prices } from '../generated/wasm/whichride_wasm.js';
import wasmUrl from '../generated/wasm/whichride_wasm_bg.wasm?url';
import seed from '../generated/catalogue.json?raw';

export type Source = 'loading' | 'server' | 'built-in';
export interface OfferInfo { id: string; name: string }
export interface ProviderInfo { id: string; name: string; offers: OfferInfo[] }
export interface Priced { providerId: string; offerId: string; offerName: string; cardLabel: string; totalCents: number }
export interface Ranking { ranked: Priced[]; cheaperNotAdded: Priced[] }
export interface Engine {
  providers(): ProviderInfo[];
  offers(metres: number, seconds: number): Priced[];
  options(metres: number, seconds: number, offerIds: string[]): Ranking;
}

/** A slow server must never hold the page; the seed is already showing. */
export const CATALOGUE_TIMEOUT_MS = 10_000;

function wrap(prices: Prices): Engine {
  return {
    providers: () => prices.providers() as ProviderInfo[],
    offers: (metres, seconds) => prices.offers(metres, seconds) as Priced[],
    options: (metres, seconds, offerIds) => prices.options(metres, seconds, offerIds) as Ranking,
  };
}

export async function seedEngine(): Promise<Engine> {
  await init({ module_or_path: wasmUrl });
  return wrap(Prices.fromSeed(seed));
}

/** Prices from `GET /v1/catalogue` (same origin). Rejects on any failure. */
export async function serverEngine(fetchFn: typeof fetch = fetch): Promise<Engine> {
  await init({ module_or_path: wasmUrl });
  const response = await fetchFn('/v1/catalogue', {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(CATALOGUE_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`catalogue: HTTP ${response.status}`);
  return wrap(Prices.fromServer(await response.text()));
}
