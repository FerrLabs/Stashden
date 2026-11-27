import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isLocale, negotiate } from '../src/app/i18n/locale.ts';

test('French wins when a French variant comes first among the supported languages', () => {
  assert.equal(negotiate(['fr']), 'fr');
  assert.equal(negotiate(['fr-CA', 'en']), 'fr');
  assert.equal(negotiate(['FR-be']), 'fr');
});

test('an unsupported language before French is skipped, an English one is not', () => {
  assert.equal(negotiate(['de-DE', 'fr']), 'fr');
  assert.equal(negotiate(['en-GB', 'fr']), 'en');
});

test('English is the fallback', () => {
  assert.equal(negotiate([]), 'en');
  assert.equal(negotiate(['de', 'ja']), 'en');
  assert.equal(negotiate(['']), 'en');
});

test('only supported codes count as a remembered choice', () => {
  assert.equal(isLocale('fr'), true);
  assert.equal(isLocale('de'), false);
  assert.equal(isLocale(null), false);
  assert.equal(isLocale('FR'), false);
});
