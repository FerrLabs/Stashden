import assert from 'node:assert/strict';
import { test } from 'node:test';
import { signal } from '@angular/core';
import { RowSelection, neighbour, successor } from '../src/app/files/selection.ts';

const ids = ['a', 'b', 'c', 'd'];

test('neighbour steps and stops at both ends', () => {
  assert.equal(neighbour(ids, 'b', 'next'), 'c');
  assert.equal(neighbour(ids, 'd', 'next'), 'd');
  assert.equal(neighbour(ids, 'b', 'previous'), 'a');
  assert.equal(neighbour(ids, 'a', 'previous'), 'a');
  assert.equal(neighbour(ids, 'c', 'first'), 'a');
  assert.equal(neighbour(ids, 'a', 'last'), 'd');
});

test('neighbour from nowhere starts at the top going down and stays at the top going up', () => {
  assert.equal(neighbour(ids, null, 'next'), 'a');
  assert.equal(neighbour(ids, null, 'previous'), 'a');
  assert.equal(neighbour(ids, 'gone', 'next'), 'a');
  assert.equal(neighbour([], null, 'next'), null);
  assert.equal(neighbour([], 'a', 'last'), null);
});

test('successor is the next row, or the previous one for the last row', () => {
  assert.equal(successor(ids, 'b'), 'c');
  assert.equal(successor(ids, 'd'), 'c');
  assert.equal(successor(['a'], 'a'), null);
  assert.equal(successor(ids, 'gone'), null);
});

function selectionOver(initial: string[]) {
  const current = signal<readonly string[]>(initial);
  return { current, selection: new RowSelection(current) };
}

test('nothing is selected at first and the first row is the tab stop', () => {
  const { selection } = selectionOver(ids);
  assert.equal(selection.selected(), null);
  assert.equal(selection.tabStop(), 'a');
});

test('selecting a row makes it the selection and the tab stop', () => {
  const { selection } = selectionOver(ids);
  selection.select('c');
  assert.equal(selection.selected(), 'c');
  assert.equal(selection.tabStop(), 'c');
});

test('moving moves focus and selection together', () => {
  const { selection } = selectionOver(ids);
  assert.equal(selection.move('next'), 'b');
  assert.equal(selection.move('next'), 'c');
  assert.equal(selection.selected(), 'c');
  assert.equal(selection.move('previous'), 'b');
  assert.equal(selection.move('last'), 'd');
  assert.equal(selection.move('next'), 'd');
  assert.equal(selection.move('first'), 'a');
});

test('clearing keeps the tab stop where it was and Space can select it again', () => {
  const { selection } = selectionOver(ids);
  selection.select('c');
  selection.clear();
  assert.equal(selection.selected(), null);
  assert.equal(selection.tabStop(), 'c');
  assert.equal(selection.move('next'), 'd');
});

test('focusing a row moves the tab stop without selecting', () => {
  const { selection } = selectionOver(ids);
  selection.select('a');
  selection.focus('c');
  assert.equal(selection.selected(), 'a');
  assert.equal(selection.tabStop(), 'c');
});

test('the selection follows its file when the listing reorders, as after a rename', () => {
  const { current, selection } = selectionOver(ids);
  selection.select('b');
  current.set(['a', 'c', 'd', 'b']);
  assert.equal(selection.selected(), 'b');
  assert.equal(selection.tabStop(), 'b');
});

test('the selection survives a reload that returns the same rows', () => {
  const { current, selection } = selectionOver(ids);
  selection.select('c');
  current.set([...ids]);
  assert.equal(selection.selected(), 'c');
});

test('a row that leaves the listing is no longer selected and the tab stop falls back', () => {
  const { current, selection } = selectionOver(ids);
  selection.select('c');
  current.set(['a', 'b', 'd']);
  assert.equal(selection.selected(), null);
  assert.equal(selection.tabStop(), 'a');
  current.set(ids);
  assert.equal(selection.selected(), 'c');
});

test('after a deletion the selection passes to the next row, or the previous for the last', () => {
  const { current, selection } = selectionOver(ids);
  selection.select('b');
  selection.passFrom('b');
  current.set(['a', 'c', 'd']);
  assert.equal(selection.selected(), 'c');

  selection.select('d');
  selection.passFrom('d');
  current.set(['a', 'c']);
  assert.equal(selection.selected(), 'c');
});

test('deleting the only row leaves nothing selected', () => {
  const { current, selection } = selectionOver(['a']);
  selection.select('a');
  selection.passFrom('a');
  current.set([]);
  assert.equal(selection.selected(), null);
  assert.equal(selection.tabStop(), null);
});

test('deleting a row that is not the selection leaves the selection alone', () => {
  const { selection } = selectionOver(ids);
  selection.select('a');
  selection.passFrom('c');
  assert.equal(selection.selected(), 'a');
});

test('changing folder clears the selection and the tab stop', () => {
  const { current, selection } = selectionOver(ids);
  selection.select('c');
  selection.reset();
  current.set(ids);
  assert.equal(selection.selected(), null);
  assert.equal(selection.tabStop(), 'a');
});
