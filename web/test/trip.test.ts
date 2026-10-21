import { parseKilometres, parseMinutes } from '../src/lib/trip';
import { formatMoney } from '../src/lib/money';

test('kilometres accept a point, a comma, whole numbers and metres', () => {
  expect(parseKilometres('7.4')).toBe(7_400);
  expect(parseKilometres(' 7,4 ')).toBe(7_400);
  expect(parseKilometres('12')).toBe(12_000);
  expect(parseKilometres('12.345')).toBe(12_345);
  expect(parseKilometres(',5')).toBe(500);
  expect(parseKilometres('2000')).toBe(2_000_000);
  expect(parseKilometres('0.1')).toBe(100);
});

test.each(['', '7.4.1', '7,4,1', 'abc', '-3', '0', '0.05', '2000.001', '12.3456', '1e3', '7 4', '١٢'])(
  'kilometres refuse %j',
  (text) => expect(parseKilometres(text)).toBeNull(),
);

test('minutes are whole, bounded, returned as seconds', () => {
  expect(parseMinutes(' 22 ')).toBe(1_320);
  expect(parseMinutes('1440')).toBe(86_400);
  for (const text of ['', '0', '1441', '2.5', '-1', 'x', '00000']) expect(parseMinutes(text)).toBeNull();
});

test('money is euros in the language style', () => {
  expect(formatMoney(969, 'en')).toBe('€9.69');
  expect(formatMoney(969, 'de')).toBe('9,69 €');
  expect(formatMoney(0, 'en')).toBe('€0.00');
});
