import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';

type Stubs = { stored?: string | null; languages: readonly string[]; storageThrows?: boolean };

const written = new Map<string, string>();
let documentElement = { lang: 'en' };
let fresh = 0;

function define(values: Record<string, unknown>): void {
  for (const [name, value] of Object.entries(values)) {
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  }
}

function install({ stored = null, languages, storageThrows = false }: Stubs): void {
  written.clear();
  documentElement = { lang: 'en' };
  define({
    document: { documentElement },
    navigator: { languages },
    localStorage: {
      getItem: (key: string) => {
        if (storageThrows) {
          throw new Error('blocked');
        }
        return key === 'stashden.language' ? stored : null;
      },
      setItem: (key: string, value: string) => {
        if (storageThrows) {
          throw new Error('blocked');
        }
        written.set(key, value);
      },
    },
  });
}

function load(): Promise<typeof import('../src/app/i18n/language.ts')> {
  fresh += 1;
  return import(`../src/app/i18n/language.ts?load=${fresh}`);
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'document');
  Reflect.deleteProperty(globalThis, 'navigator');
  Reflect.deleteProperty(globalThis, 'localStorage');
});

test('a first visit with a French browser starts in French and sets html lang', async () => {
  install({ languages: ['fr-FR', 'en'] });
  const language = await load();
  assert.equal(language.locale(), 'fr');
  assert.equal(documentElement.lang, 'fr');
  assert.equal(language.t('common.cancel'), 'Annuler');
});

test('a remembered choice beats the browser preference', async () => {
  install({ stored: 'en', languages: ['fr'] });
  const language = await load();
  assert.equal(language.locale(), 'en');
  assert.equal(documentElement.lang, 'en');
});

test('a stored value that is not a supported language is ignored', async () => {
  install({ stored: 'de', languages: ['fr'] });
  assert.equal((await load()).locale(), 'fr');
});

test('choosing a language updates the page, html lang and the stored choice', async () => {
  install({ languages: ['en'] });
  const language = await load();
  language.chooseLocale('fr');
  assert.equal(language.locale(), 'fr');
  assert.equal(documentElement.lang, 'fr');
  assert.equal(written.get('stashden.language'), 'fr');
  assert.equal(language.t('files.announce.uploaded', { count: 2 }), '2 fichiers envoyés');
  assert.equal(language.formatSize(4_200_000), '4,2 Mo');
});

test('blocked storage still lets the language change', async () => {
  install({ languages: ['en'], storageThrows: true });
  const language = await load();
  language.chooseLocale('fr');
  assert.equal(language.locale(), 'fr');
  assert.equal(documentElement.lang, 'fr');
});
