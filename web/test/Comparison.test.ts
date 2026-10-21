import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
import Comparison from '../src/components/Comparison.svelte';
import { loadShared } from '../src/i18n/android';
import { format, orList, type Messages } from '../src/i18n/format';
import { seedEngine } from '../src/lib/engine';
import { formatMoney } from '../src/lib/money';
import { KEY, TTL_MS } from '../src/lib/memory';

const SEED = readFileSync('src/generated/catalogue.json', 'utf8');
const messages: Messages = { ...loadShared('en'), close: 'Close' };
const providerNames = (JSON.parse(SEED) as { providers: { name: string }[] }).providers.map((p) => p.name);
const TRIP = [7_400, 1_320] as const;

function offline() {
  vi.stubGlobal('fetch', vi.fn(async () => { throw new TypeError('offline'); }));
}

beforeEach(() => {
  localStorage.clear();
  offline();
});
afterEach(() => vi.unstubAllGlobals());

async function page() {
  const user = userEvent.setup();
  render(Comparison, { lang: 'en', messages, providerNames });
  await screen.findByText('built-in prices');
  return user;
}

async function typeTrip(user: ReturnType<typeof userEvent.setup>, km = '7.4', minutes = '22') {
  await user.type(screen.getByLabelText('Distance (km)'), km);
  await user.type(screen.getByLabelText('Time (min)'), minutes);
}

function tick(dialog: HTMLElement, offerId: string) {
  const box = dialog.querySelector<HTMLInputElement>(`input[value="${offerId}"]`);
  if (!box) throw new Error(`no checkbox for ${offerId}`);
  return box;
}

test('a first visit asks for the cars near you', async () => {
  await page();
  expect(screen.getByRole('heading', { name: 'Which cars are near you?' })).toBeInTheDocument();
  expect(screen.getByText(format(messages.intro_body, 'en', orList(providerNames, messages, 'en')))).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toBeInTheDocument();
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
});

test('ticked cars are ranked cheapest first, as the core ranks them', async () => {
  const user = await page();
  await typeTrip(user);
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  await user.click(tick(dialog, 'miles.s'));
  await user.click(tick(dialog, 'bolt.vw-polo'));
  await user.click(within(dialog).getByRole('button', { name: 'Add 2 cars' }));

  const expected = (await seedEngine()).options(...TRIP, ['miles.s', 'bolt.vw-polo']);
  const rows = within(screen.getByRole('list')).getAllByRole('listitem');
  expect(rows).toHaveLength(2);
  expected.ranked.forEach((offer, i) => {
    expect(rows[i]).toHaveTextContent(offer.offerName);
    expect(rows[i]).toHaveTextContent(formatMoney(offer.totalCents, 'en'));
  });
  expect(rows[0]).toHaveTextContent('Cheapest of your options');
  expect(rows[1]).not.toHaveTextContent('Cheapest of your options');
  const cheaper = expected.cheaperNotAdded;
  const providers = (await seedEngine()).providers();
  const line = cheaper.length
    ? format(messages.cheaper_not_added, 'en', cheaper.length, formatMoney(cheaper[0].totalCents, 'en'),
        `${providers.find((p) => p.id === cheaper[0].providerId)!.name} ${cheaper[0].offerName}`)
    : 'No car you did not add is cheaper.';
  expect(screen.getByText(line)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add more cars' })).toHaveFocus();
});

test('the sheet button reads Done when nothing is ticked', async () => {
  const user = await page();
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  expect(within(dialog).getByRole('button', { name: 'Done' })).toBeInTheDocument();
  await user.click(tick(dialog, 'miles.s'));
  expect(within(dialog).getByRole('button', { name: 'Add 1 car' })).toBeInTheDocument();
});

test('closing the sheet drops its ticks and returns focus', async () => {
  const user = await page();
  const opener = screen.getByRole('button', { name: 'Add the cars near you' });
  await user.click(opener);
  const dialog = screen.getByRole('dialog');
  await user.click(tick(dialog, 'miles.s'));
  await fireEvent(dialog, new Event('cancel', { cancelable: true }));
  expect(dialog).not.toHaveAttribute('open');
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toHaveFocus();

  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  expect(dialog).toHaveAttribute('open');
  expect(tick(dialog, 'miles.s')).not.toBeChecked();
  await user.click(within(dialog).getByRole('button', { name: 'Close' }));
  expect(dialog).not.toHaveAttribute('open');
});

test('an invalid trip lists the cars without prices', async () => {
  const user = await page();
  await typeTrip(user, 'abc', '22');
  const km = screen.getByLabelText('Distance (km)');
  expect(km).toHaveAttribute('aria-invalid', 'true');
  expect(screen.getByText('Between 0.1 and 2000 km')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  expect(dialog).not.toHaveTextContent('€');
  await user.click(tick(dialog, 'miles.s'));
  await user.click(within(dialog).getByRole('button', { name: 'Add 1 car' }));
  const rows = within(screen.getByRole('list')).getAllByRole('listitem');
  expect(rows).toHaveLength(1);
  expect(rows[0]).not.toHaveTextContent('€');
  expect(rows[0]).not.toHaveTextContent('Cheapest of your options');
  expect(screen.queryByText(/did not add/)).not.toBeInTheDocument();
});

test('an empty field shows no error', async () => {
  await page();
  expect(screen.getByLabelText('Distance (km)')).not.toHaveAttribute('aria-invalid', 'true');
  expect(screen.queryByText('Between 0.1 and 2000 km')).not.toBeInTheDocument();
});

test('server prices replace the built-in ones', async () => {
  const catalogue = JSON.parse(SEED);
  for (const card of catalogue.providers[0].offers[0].cards) card.per_km = 1;
  const body = JSON.stringify({ catalogue, offers: [], proposals: [] });
  vi.stubGlobal('fetch', vi.fn(async () => new Response(body)));
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: [catalogue.providers[0].offers[0].id], savedAt: Date.now() }));
  render(Comparison, { lang: 'en', messages, providerNames });
  const { Prices } = await import('../src/generated/wasm/whichride_wasm.js');
  const server = Prices.fromServer(body).options(...TRIP, [catalogue.providers[0].offers[0].id]);
  await waitFor(() =>
    expect(screen.getByRole('list')).toHaveTextContent(formatMoney(server.ranked[0].totalCents, 'en')),
  );
  expect(screen.queryByText('built-in prices')).not.toBeInTheDocument();
  expect(screen.queryByText('Loading prices…')).not.toBeInTheDocument();
});

test('built-in prices when the server answers an error', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 503 })));
  render(Comparison, { lang: 'en', messages, providerNames });
  expect(await screen.findByText('built-in prices')).toBeInTheDocument();
});

test('the trip and cars come back within 15 minutes', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['miles.s'], savedAt: Date.now() - 60_000 }));
  await page();
  expect(screen.getByLabelText('Distance (km)')).toHaveValue('7.4');
  expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
});

test('remembered cars that no longer exist are ignored', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['gone.car', 'miles.s'], savedAt: Date.now() }));
  const user = await page();
  expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
  await user.click(screen.getByRole('button', { name: 'Add more cars' }));
  expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Add 1 car' })).toBeInTheDocument();
});

test('memory older than 15 minutes is not used', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['miles.s'], savedAt: Date.now() - TTL_MS - 1 }));
  await page();
  expect(screen.getByLabelText('Distance (km)')).toHaveValue('');
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toBeInTheDocument();
});

test('changes are remembered', async () => {
  const user = await page();
  await typeTrip(user);
  const saved = JSON.parse(localStorage.getItem(KEY) ?? '{}');
  expect(saved).toMatchObject({ km: '7.4', minutes: '22', offers: [] });
});

test('a car remembered twice is listed and counted once, even without a trip', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '', minutes: '', offers: ['miles.s', 'miles.s'], savedAt: Date.now() }));
  const user = await page();
  expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
  await user.click(screen.getByRole('button', { name: 'Add more cars' }));
  expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Add 1 car' })).toBeInTheDocument();
});
