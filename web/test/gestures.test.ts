import assert from 'node:assert/strict';
import { test } from 'node:test';
import { commandFor, opensOnTap, type Keystroke } from '../src/app/files/gestures.ts';

function press(key: string, modifiers: Partial<Keystroke> = {}): Keystroke {
  return { key, shiftKey: false, altKey: false, ctrlKey: false, metaKey: false, ...modifiers };
}

test('arrows, Home and End move the selection', () => {
  assert.deepEqual(commandFor(press('ArrowDown')), { kind: 'move', step: 'next' });
  assert.deepEqual(commandFor(press('ArrowUp')), { kind: 'move', step: 'previous' });
  assert.deepEqual(commandFor(press('Home')), { kind: 'move', step: 'first' });
  assert.deepEqual(commandFor(press('End')), { kind: 'move', step: 'last' });
});

test('Enter opens, Space selects, Escape clears', () => {
  assert.deepEqual(commandFor(press('Enter')), { kind: 'open' });
  assert.deepEqual(commandFor(press(' ')), { kind: 'select' });
  assert.deepEqual(commandFor(press('Escape')), { kind: 'clear' });
});

test('Right reaches the actions, F2 renames and Delete removes', () => {
  assert.deepEqual(commandFor(press('ArrowRight')), { kind: 'actions' });
  assert.deepEqual(commandFor(press('F2')), { kind: 'rename' });
  assert.deepEqual(commandFor(press('Delete')), { kind: 'remove' });
});

test('Shift+F10 and the menu key open the Open with menu, F10 alone does not', () => {
  assert.deepEqual(commandFor(press('F10', { shiftKey: true })), { kind: 'menu' });
  assert.deepEqual(commandFor(press('ContextMenu')), { kind: 'menu' });
  assert.equal(commandFor(press('F10')), null);
});

test('a modifier keeps browser and assistive technology shortcuts out of the list', () => {
  for (const key of ['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', ' ', 'Delete', 'F2']) {
    assert.equal(commandFor(press(key, { ctrlKey: true })), null, `ctrl ${key}`);
    assert.equal(commandFor(press(key, { altKey: true })), null, `alt ${key}`);
    assert.equal(commandFor(press(key, { metaKey: true })), null, `meta ${key}`);
    assert.equal(commandFor(press(key, { shiftKey: true })), null, `shift ${key}`);
  }
});

test('other keys are left alone', () => {
  for (const key of ['a', 'Tab', 'ArrowLeft', 'PageDown']) {
    assert.equal(commandFor(press(key)), null, key);
  }
});

test('a touch or pen tap opens, a mouse click or a synthesized one does not', () => {
  assert.equal(opensOnTap('touch'), true);
  assert.equal(opensOnTap('pen'), true);
  assert.equal(opensOnTap('mouse'), false);
  assert.equal(opensOnTap(''), false);
});
