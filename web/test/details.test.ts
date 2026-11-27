import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Trashed } from '../src/app/node.ts';
import { stubBrowser } from './browser.ts';

stubBrowser('en');
const { chooseLocale } = await import('../src/app/i18n/language.ts');
const { describeLink } = await import('../src/app/share.ts');
const { describeTrashed } = await import('../src/app/files/trash-text.ts');

const plain = (text: string): string => text.replace(/[\u00a0\u202f]/g, ' ');
const share = {
  id: 'a',
  node_id: 'n',
  name: 'notes.txt',
  has_password: true,
  expires_at: '2020-04-04T10:00:00Z',
  created_at: '2020-01-01T10:00:00Z',
  last_used_at: '2026-03-05T10:00:00Z',
};
const trashed: Trashed = {
  id: 't',
  owner_id: 'o',
  parent_id: null,
  name: 'old.txt',
  kind: 'file',
  size: 512,
  etag: 'e',
  created_at: '2026-03-01T10:00:00Z',
  updated_at: '2026-03-01T10:00:00Z',
  deleted_at: '2026-03-05T11:00:00Z',
  expires_at: '2026-04-04T12:00:00Z',
};

test('a share summary is split by middle dots, one phrase each', () => {
  chooseLocale('en');
  assert.equal(
    describeLink(share),
    'Expired Apr 4, 2020 · Last opened Mar 5 · Password protected',
  );
  chooseLocale('fr');
  assert.equal(
    describeLink(share),
    'Expiré le 4 avr. 2020 · Dernière ouverture le 5 mars · Protégé par mot de passe',
  );
});

test('a trash summary has no seconds and no comma between date and time', () => {
  chooseLocale('en');
  assert.equal(
    plain(describeTrashed(trashed)),
    `512 B · deleted March 5, 2026 at 11:00 AM · permanent deletion April 4, 2026 at 12:00 PM`,
  );
  chooseLocale('fr');
  assert.equal(
    plain(describeTrashed(trashed)),
    `512 o · supprimé le 5 mars 2026 à 11:00 · suppression définitive le 4 avril 2026 à 12:00`,
  );
});
