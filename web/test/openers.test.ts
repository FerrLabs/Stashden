import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  PREVIEW_LIMIT,
  extensionOf,
  mimeOf,
  openerFor,
  waysToOpen,
} from '../src/app/files/openers.ts';

const writer = { canChange: true };
const reader = { canChange: false };

function open(name: string, size = 10, options = writer) {
  return openerFor({ name, size }, options);
}

test('known types open with their viewer whatever the case of the extension', () => {
  assert.equal(open('photo.png').viewer, 'image');
  assert.equal(open('PHOTO.PNG').viewer, 'image');
  assert.equal(open('Scan.JpEg').viewer, 'image');
  assert.equal(open('report.PDF').viewer, 'pdf');
  assert.equal(open('notes.TXT').viewer, 'text');
  assert.equal(open('README.Md').viewer, 'text');
});

test('md, markdown and csv stay plain text until their viewers exist', () => {
  for (const name of ['a.md', 'a.markdown', 'a.csv']) {
    assert.deepEqual(open(name), { viewer: 'text', editor: null, reason: 'ready' });
  }
});

test('xlsx is declared and has no viewer yet', () => {
  assert.deepEqual(open('budget.xlsx'), { viewer: null, editor: null, reason: 'unsupported' });
});

test('only the last extension counts', () => {
  assert.equal(extensionOf('archive.tar.gz'), 'gz');
  assert.equal(open('archive.tar.gz').viewer, null);
  assert.equal(open('photo.png.txt').viewer, 'text');
  assert.equal(open('notes.txt.png').viewer, 'image');
});

test('a name without extension or made of a dot only has no opener', () => {
  assert.equal(extensionOf('Makefile'), '');
  assert.equal(extensionOf('.png'), '');
  assert.equal(extensionOf('.gitignore'), '');
  assert.equal(extensionOf('trailing.'), '');
  for (const name of ['Makefile', '.png', '.gitignore', 'trailing.']) {
    assert.deepEqual(open(name), { viewer: null, editor: null, reason: 'unsupported' });
  }
});

test('an unknown type has no opener', () => {
  assert.deepEqual(open('data.bin'), { viewer: null, editor: null, reason: 'unsupported' });
  assert.deepEqual(open('movie.mkv'), { viewer: null, editor: null, reason: 'unsupported' });
});

test('the size limit is inclusive and wins over the type', () => {
  assert.equal(open('big.txt', PREVIEW_LIMIT).viewer, 'text');
  assert.deepEqual(open('big.txt', PREVIEW_LIMIT + 1), {
    viewer: null,
    editor: null,
    reason: 'tooLarge',
  });
  assert.equal(open('big.bin', PREVIEW_LIMIT + 1).reason, 'tooLarge');
});

test('a reader gets the same viewer as a writer and no editor', () => {
  assert.deepEqual(open('a.md', 10, reader), open('a.md', 10, writer));
  assert.equal(open('a.md', 10, reader).editor, null);
});

test('the Open with menu only exists when a file has more than one way to open', () => {
  assert.deepEqual(waysToOpen(open('a.md')), []);
  assert.deepEqual(waysToOpen(open('a.png')), []);
  assert.deepEqual(waysToOpen(open('a.bin')), []);
  assert.deepEqual(waysToOpen(open('big.txt', PREVIEW_LIMIT + 1)), []);
  assert.deepEqual(waysToOpen({ viewer: 'text', editor: 'markdown', reason: 'ready' }), [
    'preview',
    'download',
  ]);
  assert.deepEqual(waysToOpen({ viewer: 'text', editor: null, reason: 'ready' }), []);
});

test('the mime type follows the extension and falls back to octet-stream', () => {
  assert.equal(mimeOf('a.SVG'), 'image/svg+xml');
  assert.equal(mimeOf('a.jpg'), 'image/jpeg');
  assert.equal(mimeOf('a.jpeg'), 'image/jpeg');
  assert.equal(mimeOf('a.pdf'), 'application/pdf');
  assert.equal(mimeOf('a.txt'), 'application/octet-stream');
  assert.equal(mimeOf('Makefile'), 'application/octet-stream');
});
