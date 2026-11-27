import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const css = readFileSync(
  new URL('../src/app/files/file-row.css', import.meta.url).pathname,
  'utf8',
);

test('the Open button of the selected row is never hidden, so a synthesized activation can open', () => {
  const hiding = [...css.matchAll(/([^{}]+)\{([^{}]*display:\s*none[^{}]*)\}/g)].map((match) =>
    (match[1] ?? '').trim(),
  );
  for (const selector of hiding) {
    for (const part of selector.split(',').map((piece) => piece.trim())) {
      if (part.includes('.open')) {
        assert.match(part, /:not\(\.selected\)/, part);
      }
    }
  }
});
