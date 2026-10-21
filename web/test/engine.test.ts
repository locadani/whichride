import { readFileSync } from 'node:fs';
import { seedEngine, serverEngine } from '../src/lib/engine';

const SEED = readFileSync('src/generated/catalogue.json', 'utf8');
const TRIP = [7_400, 22 * 60] as const;

function respond(body: string, status = 200): typeof fetch {
  return vi.fn(async () => new Response(body, { status })) as unknown as typeof fetch;
}

test('the seed engine prices every offer, cheapest first', async () => {
  const engine = await seedEngine();
  const offers = engine.offers(...TRIP);
  const count = engine.providers().reduce((n, p) => n + p.offers.length, 0);
  expect(offers).toHaveLength(count);
  expect(offers.map((o) => o.totalCents)).toEqual([...offers.map((o) => o.totalCents)].sort((a, b) => a - b));
  expect(Number.isInteger(offers[0].totalCents)).toBe(true);
});

test('options rank only the ticked offers', async () => {
  const engine = await seedEngine();
  const ranking = engine.options(...TRIP, ['miles.s', 'bolt.vw-polo', 'gone.car']);
  expect(ranking.ranked.map((o) => o.offerId).sort()).toEqual(['bolt.vw-polo', 'miles.s']);
  expect(ranking.cheaperNotAdded.every((o) => o.totalCents < ranking.ranked[0].totalCents)).toBe(true);
});

test('the server engine reads /v1/catalogue', async () => {
  const fetchFn = respond(JSON.stringify({ catalogue: JSON.parse(SEED), offers: [], proposals: [] }));
  const engine = await serverEngine(fetchFn);
  expect(fetchFn).toHaveBeenCalledWith('/v1/catalogue', expect.objectContaining({ signal: expect.any(AbortSignal) }));
  expect(engine.offers(...TRIP)).toEqual((await seedEngine()).offers(...TRIP));
});

test('the server engine rejects errors and invalid bodies', async () => {
  await expect(serverEngine(respond('{}', 503))).rejects.toThrow();
  await expect(serverEngine(respond('<html>'))).rejects.toThrow();
  await expect(serverEngine(respond('{"catalogue": {"providers": 1}}'))).rejects.toThrow();
});
