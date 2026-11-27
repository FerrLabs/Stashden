import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dateIn, dayIn, joinDetails, momentIn, sizeIn } from '../src/app/i18n/format.ts';

const NBSP = ' ';

test('sizes carry the unit of the active language', () => {
  assert.equal(sizeIn('en', 4_200_000), `4.2${NBSP}MB`);
  assert.equal(sizeIn('fr', 4_200_000), `4,2${NBSP}Mo`);
  assert.equal(sizeIn('en', 512), `512${NBSP}B`);
  assert.equal(sizeIn('fr', 512), `512${NBSP}o`);
  assert.equal(sizeIn('fr', 18_000), `18${NBSP}ko`);
  assert.equal(sizeIn('en', 3_000_000_000_000), `3${NBSP}TB`);
});

test('dates follow the language and the year can be left out', () => {
  assert.equal(dateIn('en', '2026-03-05T10:00:00Z'), 'Mar 5, 2026');
  assert.equal(dateIn('fr', '2026-03-05T10:00:00Z'), '5 mars 2026');
  assert.equal(dayIn('fr', new Date('2026-03-05T10:00:00Z'), false), '5 mars');
  assert.equal(dayIn('en', new Date('2026-03-05T10:00:00Z'), false), 'Mar 5');
});

test('moments carry the date and the time without seconds', () => {
  assert.match(momentIn('fr', '2026-03-05T10:04:09Z'), /^5 mars 2026 à 10:04$/);
  assert.match(momentIn('en', '2026-03-05T10:04:09Z'), /^March 5, 2026 at 10:04\s?AM$/);
});

test('details are joined by a middle dot', () => {
  assert.equal(joinDetails(['one', 'two', 'three']), 'one · two · three');
  assert.equal(joinDetails(['alone']), 'alone');
});

test('an unreadable date renders as nothing', () => {
  assert.equal(dateIn('en', 'not a date'), '');
  assert.equal(momentIn('fr', ''), '');
});
