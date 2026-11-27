import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { test } from 'node:test';
import { CATALOGS } from '../src/app/i18n/catalogs.ts';

const ROOT = new URL('../src/app/', import.meta.url).pathname;
const CATALOG_DIRECTORY = join(ROOT, 'i18n');

function sources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return path === CATALOG_DIRECTORY ? [] : sources(path);
    }
    return /\.(ts|html)$/.test(entry.name) ? [path] : [];
  });
}

const files = sources(ROOT).map((path) => ({
  name: relative(ROOT, path),
  text: readFileSync(path, 'utf8'),
}));
const templates = files.filter((file) => file.name.endsWith('.html'));
const catalogKeys = Object.keys(CATALOGS.en);

test('every key a source file asks for exists in the catalogs', () => {
  const unknown = files.flatMap((file) =>
    [...file.text.matchAll(/\bt\(\s*'([^']+)'/g)]
      .map((match) => match[1] ?? '')
      .filter((key) => !catalogKeys.includes(key))
      .map((key) => `${file.name}: ${key}`),
  );
  assert.deepEqual(unknown, []);
});

test('every catalog key is used by a source file', () => {
  const used = new Set(
    files.flatMap((file) => [...file.text.matchAll(/\bt\(\s*'([^']+)'/g)].map((m) => m[1])),
  );
  assert.deepEqual(
    catalogKeys.filter((key) => !used.has(key)),
    [],
  );
});

test('no translated message is glued to another string', () => {
  const glued = files.filter(({ text }) =>
    /\bt\([^)]*\)\s*\+|\+\s*t\(|\$\{\s*t\(/.test(text),
  );
  assert.deepEqual(
    glued.map((file) => file.name),
    [],
  );
});

test('templates build no text by concatenating string literals', () => {
  const concatenating = templates.filter(({ text }) => /'\s*\+|\+\s*'/.test(text));
  assert.deepEqual(
    concatenating.map((file) => file.name),
    [],
  );
});

const TAG = /<\/?[a-zA-Z][^>"']*(?:(?:"[^"]*"|'[^']*')[^>"']*)*>/g;
const CONTROL_FLOW = /@(?:if|for|switch|case|else(?: if)?|empty|default)\b[^{}]*\{/g;

function visibleText(template: string): string {
  return template
    .replace(/<svg[\s\S]*?<\/svg>/g, '')
    .replace(/\{\{[\s\S]*?\}\}/g, '')
    .replace(CONTROL_FLOW, '')
    .replace(TAG, '')
    .replace(/[{}]/g, '');
}

test('templates hold no visible text of their own', () => {
  const offenders = templates.flatMap(({ name, text }) =>
    visibleText(text)
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => /\p{L}{2,}/u.test(line) && line !== 'Stashden')
      .map((line) => `${name}: ${line}`),
  );
  assert.deepEqual(offenders, []);
});

test('templates hold no literal accessible name, title or placeholder', () => {
  const attribute = /(?<![\w.[(-])(aria-label|title|alt|placeholder|label|heading|hint|action|body)="([^"]*\p{L}[^"]*)"/gu;
  const offenders = templates.flatMap(({ name, text }) =>
    [...text.matchAll(attribute)].map((match) => `${name}: ${match[0]}`),
  );
  assert.deepEqual(offenders, []);
});

test('code raises and records no sentence of its own', () => {
  const literal = /(?:\.set\(|\.push\(|new \w*Error\(|super\()\s*['"`][A-Z][^'"`]* [^'"`]*['"`]/;
  const offenders = files
    .filter(({ name }) => name.endsWith('.ts') && literal.test(readFileSync(join(ROOT, name), 'utf8')))
    .map((file) => file.name);
  assert.deepEqual(offenders, []);
});
