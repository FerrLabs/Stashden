import assert from 'node:assert/strict';
import { test } from 'node:test';
import { stubBrowser } from './browser.ts';
import { childOf, linkTo, normalise, parentOf, segmentsOf } from '../src/app/folder.ts';

stubBrowser('en');
const { SHARED_WITH_ME, describeAccess, describeOrigin, whereIs } = await import(
  '../src/app/grant.ts'
);
const { byKindThenName } = await import('../src/app/node.ts');
const { removalBody } = await import('../src/app/files/removal-text.ts');
const { bytesFrom, gigabytesOf, describeUsage, generatePassword } = await import(
  '../src/app/accounts/managed.ts'
);

test('normalise drops empty segments and outer slashes', () => {
  assert.equal(normalise('//a///b/'), 'a/b');
  assert.equal(normalise('/'), '');
});

test('linkTo roots the route at files and splits the path', () => {
  assert.deepEqual(linkTo('/a/b'), ['/', 'files', 'a', 'b']);
  assert.deepEqual(linkTo(''), ['/', 'files']);
});

test('segmentsOf gives each crumb its cumulative path', () => {
  assert.deepEqual(segmentsOf('a/b/c'), [
    { name: 'a', path: 'a' },
    { name: 'b', path: 'a/b' },
    { name: 'c', path: 'a/b/c' },
  ]);
  assert.deepEqual(segmentsOf(''), []);
});

test('childOf joins below the root without a leading slash', () => {
  assert.equal(childOf('', 'x'), 'x');
  assert.equal(childOf('/a/', 'x'), 'a/x');
});

test('parentOf is null at the root, empty for top level entries, the parent path otherwise', () => {
  assert.equal(parentOf(''), null);
  assert.equal(parentOf('a'), '');
  assert.equal(parentOf('a/b/c'), 'a/b');
});

test('whereIs tells own files, the shelf and a mount apart', () => {
  assert.deepEqual(whereIs('docs/a'), { kind: 'own' });
  assert.deepEqual(whereIs(''), { kind: 'own' });
  assert.deepEqual(whereIs(SHARED_WITH_ME), { kind: 'shelf' });
  assert.deepEqual(whereIs(`/${SHARED_WITH_ME}/Project/x`), { kind: 'shared', mount: 'Project' });
  assert.deepEqual(whereIs(`docs/${SHARED_WITH_ME}`), { kind: 'own' });
});

test('access wording differs between read and write', () => {
  assert.notEqual(describeAccess('read'), describeAccess('write'));
});

test('an origin names the owner, falling back to their address when they have no name', () => {
  const base = {
    id: '1',
    name: 'n',
    kind: 'file' as const,
    access: 'read' as const,
    owner_email: 'o@x.test',
    created_at: '',
  };
  assert.match(describeOrigin({ ...base, owner_name: 'Olive' }), /Olive/);
  const unnamed = describeOrigin({ ...base, owner_name: '' });
  assert.match(unnamed, /o@x\.test/);
  assert.ok(unnamed.includes(describeAccess('read')));
});

test('folders sort before files, then by name', () => {
  const node = (name: string, kind: 'file' | 'directory') => ({ name, kind }) as never;
  const sorted = [node('b', 'file'), node('z', 'directory'), node('a', 'file'), node('c', 'directory')]
    .sort(byKindThenName)
    .map((item: { name: string }) => item.name);
  assert.deepEqual(sorted, ['c', 'z', 'a', 'b']);
});

test('removal text distinguishes folders from files and own from shared content', () => {
  const texts = new Set([
    removalBody('directory', true),
    removalBody('directory', false),
    removalBody('file', true),
    removalBody('file', false),
  ]);
  assert.equal(texts.size, 4);
});

test('quota gigabytes convert to bytes and back with decimal gigabytes', () => {
  assert.equal(bytesFrom('1'), 1_000_000_000);
  assert.equal(bytesFrom(' 1.5 '), 1_500_000_000);
  assert.equal(gigabytesOf(1_500_000_000), '1.5');
  assert.equal(gigabytesOf(1_234_567_890), '1.23');
  assert.equal(gigabytesOf(undefined), '');
});

test('a quota that is zero, negative or not a number is refused', () => {
  for (const bad of ['0', '-3', 'abc', '', 'Infinity', 'NaN']) {
    assert.equal(bytesFrom(bad), null, bad);
  }
});

test('usage says nothing stored without a quota and shows used over max with one', () => {
  const account = { id: '1', email: 'a', display_name: 'a', role: 'member' as const };
  assert.notEqual(describeUsage(account), describeUsage({ ...account, bytes_max: 1000 }));
  const used = describeUsage({ ...account, bytes_max: 2000, bytes_used: 1000 });
  const unused = describeUsage({ ...account, bytes_max: 2000 });
  assert.notEqual(used, unused);
});

test('generated passwords are long, drawn from the unambiguous alphabet and differ each time', () => {
  const first = generatePassword();
  assert.equal(first.length, 20);
  assert.match(first, /^[abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789]+$/);
  assert.notEqual(first, generatePassword());
});
