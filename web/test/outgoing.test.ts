import assert from 'node:assert/strict';
import { test } from 'node:test';
import { stubBrowser } from './browser.ts';

stubBrowser('en');
const { chosenFromDrop, chosenFromFiles, fromChosen, fromFiles } = await import(
  '../src/app/outgoing.ts'
);

function file(name: string, relative = ''): File {
  const made = new File(['x'], name);
  Object.defineProperty(made, 'webkitRelativePath', { value: relative });
  return made;
}

type Fake =
  | { isFile: true; isDirectory: false; name: string; file: (ok: (f: File) => void) => void }
  | {
      isFile: false;
      isDirectory: true;
      name: string;
      createReader: () => { readEntries: (ok: (batch: Fake[]) => void) => void };
    };

function fakeFile(name: string): Fake {
  return { isFile: true, isDirectory: false, name, file: (ok) => ok(file(name)) };
}

function fakeDirectory(name: string, children: Fake[]): Fake {
  return {
    isFile: false,
    isDirectory: true,
    name,
    createReader: () => {
      let served = false;
      return {
        readEntries: (ok) => {
          ok(served ? [] : children);
          served = true;
        },
      };
    },
  };
}

function dropOf(entries: (Fake | null)[], files: File[] = []): DataTransfer {
  return {
    items: entries.map((entry) => ({ kind: 'file', webkitGetAsEntry: () => entry })),
    files,
  } as unknown as DataTransfer;
}

test('a chosen file is named by its relative folder path when it has one', () => {
  const chosen = chosenFromFiles([file('a.txt'), file('b.txt', 'photos/b.txt')]);
  assert.deepEqual(
    chosen.map((item) => item.path),
    ['a.txt', 'photos/b.txt'],
  );
});

test('sending a chosen file uploads it to the destination with progress passed through', async () => {
  const seen: unknown[] = [];
  const outgoing = fromFiles(
    async (path, sent, progress) => {
      seen.push([path, sent.name]);
      progress?.(1, 2);
    },
    [file('a.txt', 'dir/a.txt')],
  );
  assert.equal(outgoing[0]!.name, 'dir/a.txt');
  const ticks: number[][] = [];
  await outgoing[0]!.send('/dest/dir/a.txt', (sent, total) => ticks.push([sent, total]));
  assert.deepEqual(seen, [['/dest/dir/a.txt', 'a.txt']]);
  assert.deepEqual(ticks, [[1, 2]]);
});

test('fromChosen keeps the order and one outgoing per file', () => {
  const outgoing = fromChosen(async () => undefined, [
    { file: file('1'), path: 'one' },
    { file: file('2'), path: 'two' },
  ]);
  assert.deepEqual(
    outgoing.map((item) => item.name),
    ['one', 'two'],
  );
});

test('dropping folders walks them recursively and prefixes names with the folder path', async () => {
  const tree = fakeDirectory('top', [
    fakeFile('a.txt'),
    fakeDirectory('inner', [fakeFile('b.txt')]),
  ]);
  const chosen = await chosenFromDrop(dropOf([tree, fakeFile('loose.txt')]));
  assert.deepEqual(chosen.map((item) => item.path).sort(), [
    'loose.txt',
    'top/a.txt',
    'top/inner/b.txt',
  ]);
});

test('when the browser cannot describe an entry the plain file list is used', async () => {
  const fallback = file('plain.txt');
  const chosen = await chosenFromDrop(dropOf([fakeFile('x'), null], [fallback]));
  assert.deepEqual(
    chosen.map((item) => item.path),
    ['plain.txt'],
  );
});

test('a drop with no file items uses the plain file list', async () => {
  const transfer = { items: [{ kind: 'string', webkitGetAsEntry: () => null }], files: [file('z')] };
  const chosen = await chosenFromDrop(transfer as unknown as DataTransfer);
  assert.deepEqual(
    chosen.map((item) => item.path),
    ['z'],
  );
});

test('an empty folder yields nothing and an unreadable file entry rejects the drop', async () => {
  assert.deepEqual(await chosenFromDrop(dropOf([fakeDirectory('empty', [])])), []);
  const broken: Fake = {
    isFile: true,
    isDirectory: false,
    name: 'bad',
    file: () => {
      throw new Error('unreadable');
    },
  };
  await assert.rejects(chosenFromDrop(dropOf([broken])), /unreadable/);
});
