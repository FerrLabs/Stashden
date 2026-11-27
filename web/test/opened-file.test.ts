import '@angular/compiler';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { closeFile } from '../src/app/files/opened-file.ts';

function leaving() {
  const calls: string[] = [];
  return {
    calls,
    back: () => calls.push('back'),
    replace: () => calls.push('replace'),
  };
}

test('closing a file opened from the list goes back in the history', () => {
  const target = leaving();
  closeFile(true, target);
  assert.deepEqual(target.calls, ['back']);
});

test('closing a file reached by a link replaces the address instead', () => {
  const target = leaving();
  closeFile(false, target);
  assert.deepEqual(target.calls, ['replace']);
});

test('going back is done through Angular Location, never the global history', () => {
  const source = readFileSync(
    new URL('../src/app/files/opened-file.ts', import.meta.url).pathname,
    'utf8',
  );
  assert.match(source, /this\.location\.back\(\)/);
  assert.doesNotMatch(source, /\bhistory\./);
});
