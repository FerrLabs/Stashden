import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Node } from '../src/app/node.ts';
import { OPEN_PARAM, loadedNodes, targetOf } from '../src/app/files/open-link.ts';

function entry(name: string, kind: Node['kind']): Node {
  return {
    id: `id-${name}`,
    owner_id: 'u',
    parent_id: null,
    name,
    kind,
    size: 0,
    etag: 'e',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  };
}

const nodes = [entry('docs', 'directory'), entry('a.md', 'file'), entry('Notes (v2).txt', 'file')];

test('the link parameter is named open', () => {
  assert.equal(OPEN_PARAM, 'open');
});

test('no parameter and an empty one open nothing', () => {
  assert.deepEqual(targetOf(null, nodes), { kind: 'none' });
  assert.deepEqual(targetOf('', nodes), { kind: 'none' });
});

test('a file name opens that file, spaces and punctuation included', () => {
  assert.deepEqual(targetOf('a.md', nodes), { kind: 'file', node: nodes[1] });
  assert.deepEqual(targetOf('Notes (v2).txt', nodes), { kind: 'file', node: nodes[2] });
});

test('a folder name leads into that folder', () => {
  assert.deepEqual(targetOf('docs', nodes), { kind: 'directory', node: nodes[0] });
});

test('a missing name is reported, and names are matched exactly', () => {
  assert.deepEqual(targetOf('gone.md', nodes), { kind: 'missing' });
  assert.deepEqual(targetOf('A.MD', nodes), { kind: 'missing' });
  assert.deepEqual(targetOf('a.md', []), { kind: 'missing' });
});

test('a listing that failed to load yields no nodes and is never read', () => {
  const failed = {
    hasValue: () => false,
    value: (): never => {
      throw new Error('nope');
    },
  };
  assert.deepEqual(loadedNodes(failed), []);
  assert.deepEqual(targetOf('a.md', loadedNodes(failed)), { kind: 'missing' });
});

test('a loaded listing yields its nodes, and one still loading yields none', () => {
  assert.deepEqual(loadedNodes({ hasValue: () => true, value: () => nodes }), nodes);
  assert.deepEqual(loadedNodes({ hasValue: () => false, value: () => undefined }), []);
});
