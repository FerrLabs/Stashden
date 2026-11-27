import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CATALOGS } from '../src/app/i18n/catalogs.ts';
import { render } from '../src/app/i18n/render.ts';

const uploaded = (locale: 'en' | 'fr', count: number): string =>
  render(locale, CATALOGS[locale]['files.announce.uploaded'], { count });

test('English plurals: zero and many take the plural form, one the singular', () => {
  assert.equal(uploaded('en', 0), 'Uploaded 0 files');
  assert.equal(uploaded('en', 1), 'Uploaded 1 file');
  assert.equal(uploaded('en', 2), 'Uploaded 2 files');
});

test('French plurals: zero and one take the singular, many the plural', () => {
  assert.equal(uploaded('fr', 0), '0 fichier envoyé');
  assert.equal(uploaded('fr', 1), '1 fichier envoyé');
  assert.equal(uploaded('fr', 2), '2 fichiers envoyés');
});

test('French treats a fractional count as singular, English does not', () => {
  assert.equal(uploaded('fr', 1.5), '1,5 fichier envoyé');
  assert.equal(uploaded('en', 1.5), 'Uploaded 1.5 files');
});

test('numbers follow the language grouping', () => {
  assert.equal(uploaded('en', 1234), 'Uploaded 1,234 files');
  assert.equal(uploaded('fr', 1234), '1 234 fichiers envoyés');
});

test('string values are inserted as they are, in the order the message sets', () => {
  assert.equal(
    render('fr', CATALOGS.fr['files.failed.upload'], { name: 'a {b}.txt', reason: 'coupé' }),
    'L’envoi de a {b}.txt a échoué : coupé',
  );
  assert.equal(
    render('en', CATALOGS.en['files.announce.saved'], { name: 'a.txt', destination: 'Downloads' }),
    'Saved a.txt to Downloads',
  );
});

test('a missing value fails loudly instead of rendering a blank', () => {
  assert.throws(() => render('en', 'Hello {name}', {}), /\{name\}/);
});

test('a plural message without a numeric count is refused', () => {
  assert.throws(() => render('en', { one: '{count} a', other: '{count} b' }, { count: '2' }));
});
