import { KEY, TTL_MS, recall, remember } from '../src/lib/memory';

const VALUE = { km: '7.4', minutes: '22', offers: ['miles.s'] };
const NOW = 1_800_000_000_000;

beforeEach(() => localStorage.clear());

test('remembers for 15 minutes from the last change', () => {
  remember(localStorage, VALUE, NOW);
  expect(recall(localStorage, NOW + TTL_MS - 1)).toEqual(VALUE);
  expect(recall(localStorage, NOW + TTL_MS + 1)).toBeNull();
  expect(localStorage.getItem(KEY)).toBeNull();
});

test.each([
  ['not JSON', '{'],
  ['wrong shape', JSON.stringify({ km: 7, minutes: '22', offers: 'x', savedAt: NOW })],
  ['non-string offers', JSON.stringify({ ...VALUE, offers: [1], savedAt: NOW })],
  ['from the future', JSON.stringify({ ...VALUE, savedAt: NOW + 60_000 })],
  ['too long', JSON.stringify({ ...VALUE, km: 'x'.repeat(100), savedAt: NOW })],
])('drops %s', (_, raw) => {
  localStorage.setItem(KEY, raw);
  expect(recall(localStorage, NOW)).toBeNull();
  expect(localStorage.getItem(KEY)).toBeNull();
});

test('storage that throws is ignored', () => {
  const broken = {
    getItem: () => { throw new Error('denied'); },
    setItem: () => { throw new Error('quota'); },
    removeItem: () => { throw new Error('denied'); },
  } as unknown as Storage;
  expect(() => remember(broken, VALUE, NOW)).not.toThrow();
  expect(recall(broken, NOW)).toBeNull();
  expect(recall(null, NOW)).toBeNull();
});
