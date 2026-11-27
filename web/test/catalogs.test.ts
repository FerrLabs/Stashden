import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CATALOGS } from '../src/app/i18n/catalogs.ts';
import { LOCALES } from '../src/app/i18n/locale.ts';
import type { Message } from '../src/app/i18n/message.ts';
import { account as enAccount } from '../src/app/i18n/en/account.ts';
import { accounts as enAccounts } from '../src/app/i18n/en/accounts.ts';
import { common as enCommon } from '../src/app/i18n/en/common.ts';
import { files as enFiles } from '../src/app/i18n/en/files.ts';
import { history as enHistory } from '../src/app/i18n/en/history.ts';
import { publicLink as enPublic } from '../src/app/i18n/en/public.ts';
import { search as enSearch } from '../src/app/i18n/en/search.ts';
import { sharing as enSharing } from '../src/app/i18n/en/sharing.ts';
import { shell as enShell } from '../src/app/i18n/en/shell.ts';

const PLACEHOLDER = /\{(\w+)\}/g;

function texts(message: Message): string[] {
  return typeof message === 'string' ? [message] : [message.one, message.other];
}

function parameters(message: Message): string[] {
  const names = new Set<string>();
  for (const text of texts(message)) {
    for (const match of text.matchAll(PLACEHOLDER)) {
      names.add(match[1] ?? '');
    }
  }
  if (typeof message !== 'string') {
    names.add('count');
  }
  return [...names].sort();
}

const english = CATALOGS.en as Readonly<Record<string, Message>>;
const french = CATALOGS.fr as Readonly<Record<string, Message>>;

test('English and French declare exactly the same keys', () => {
  assert.deepEqual(Object.keys(french).sort(), Object.keys(english).sort());
});

test('every message takes the same named parameters in both languages', () => {
  for (const [key, message] of Object.entries(english)) {
    const translated = french[key];
    assert.ok(translated !== undefined, `${key} is missing in French`);
    assert.deepEqual(parameters(translated), parameters(message), key);
  }
});

test('a message is plural in both languages or in neither', () => {
  for (const [key, message] of Object.entries(english)) {
    assert.equal(typeof french[key], typeof message, key);
  }
});

test('no message is empty or carries a dash used as punctuation', () => {
  for (const locale of LOCALES) {
    for (const [key, message] of Object.entries(CATALOGS[locale])) {
      for (const text of texts(message)) {
        assert.notEqual(text.trim(), '', `${locale} ${key} is empty`);
        assert.doesNotMatch(text, /[–—]/, `${locale} ${key} has a dash`);
      }
    }
  }
});

test('no key is declared by two catalog files', () => {
  const areas = [
    enAccount,
    enAccounts,
    enCommon,
    enFiles,
    enHistory,
    enPublic,
    enSearch,
    enSharing,
    enShell,
  ];
  const declared = areas.reduce((total, area) => total + Object.keys(area).length, 0);
  assert.equal(declared, Object.keys(english).length);
});
