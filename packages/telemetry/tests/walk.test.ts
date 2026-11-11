import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { redactDeep, sanitizeItem } from '../src/walk.ts';

describe('redactDeep', () => {
  it('masks a string field named like a secret', () => {
    assert.deepEqual(redactDeep({ token: 'abc123def' }), { token: '*' });
    assert.deepEqual(redactDeep({ password: 'hunter2' }), { password: '*' });
    assert.deepEqual(redactDeep({ apiKey: 'x' }), { apiKey: '*' });
    assert.deepEqual(redactDeep({ 'x-auth-token': 'x' }), { 'x-auth-token': '*' });
  });

  it('masks a nested field named like a secret', () => {
    assert.deepEqual(redactDeep({ a: { apiKey: 'x' } }), { a: { apiKey: '*' } });
    assert.deepEqual(redactDeep({ list: [{ session_id: 'x' }] }), { list: [{ session_id: '*' }] });
  });

  it('keeps a field that is not named like a secret', () => {
    assert.deepEqual(redactDeep({ name: 'ok' }), { name: 'ok' });
  });

  it('keeps walking objects under a sensitive key and masks numbers', () => {
    assert.deepEqual(redactDeep({ token: { value: 'x', note: 'password=abc12345' } }), {
      token: { value: 'x', note: 'password=*' },
    });
    assert.deepEqual(redactDeep({ key: 42 }), { key: '*' });
  });

  it('masks numbers, bigints and array elements under a sensitive key', () => {
    assert.deepEqual(redactDeep({ password: 123456 }), { password: '*' });
    assert.deepEqual(redactDeep({ session_id: 987654321 }), { session_id: '*' });
    assert.deepEqual(redactDeep({ sessionId: 123456789n }), { sessionId: '*' });
    assert.deepEqual(redactDeep({ token: ['abc123def', 5, 7n, ['nested']] }), { token: ['*', '*', '*', ['*']] });
    assert.deepEqual(redactDeep({ apiKey: ['k1', { note: 'x', url: 'https://a.test/x?code=abc' }] }), {
      apiKey: ['*', { note: 'x', url: 'https://a.test/x' }],
    });
    assert.deepEqual(redactDeep({ list: ['plain'] }), { list: ['plain'] });
  });

  it('masks field names with dots, dollars and spaces and keeps benign names', () => {
    assert.deepEqual(redactDeep({ 'user.password': 'hunter2', $password: 'h', 'api key': 'k', client$token: 'c', 'auth.token': 't' }), {
      'user.password': '*',
      $password: '*',
      'api key': '*',
      client$token: '*',
      'auth.token': '*',
    });
    assert.deepEqual(redactDeep({ cacheKey: 'home', authMethod: 'sso', author: 'jane' }), {
      cacheKey: 'home',
      authMethod: 'sso',
      author: 'jane',
    });
  });

  it('scrubs string values with the given redactor', () => {
    assert.deepEqual(redactDeep({ url: 'x' }, (value) => value.toUpperCase()), { url: 'X' });
  });

  it('survives cycles', () => {
    const node: Record<string, unknown> = { name: 'a' };
    node['self'] = node;
    const copy = redactDeep(node) as Record<string, unknown>;
    assert.equal(copy['self'], copy);
  });
});

describe('sanitizeItem', () => {
  const item = (meta: object, payload: object) => ({ type: 'exception', meta, payload }) as never;

  it('redacts meta and payload', () => {
    const out = sanitizeItem(item({ page: { url: 'https://h.example.com/a?token=abc' } }, { value: 'password=abc12345' })) as never as {
      meta: { page: { url: string } };
      payload: { value: string };
    };
    assert.ok(!out.meta.page.url.includes('abc'));
    assert.ok(!out.payload.value.includes('abc'));
  });

  it('applies the page url redactor', () => {
    const out = sanitizeItem(item({ page: { url: 'https://h.example.com/p' } }, {}), {
      redactPageUrl: () => 'rewritten',
    }) as never as { meta: { page: { url: string } } };
    assert.equal(out.meta.page.url, 'rewritten');
  });

  it('drops the item when the walk throws', () => {
    const hostile = {
      get meta(): never {
        throw new Error('boom');
      },
      payload: {},
    };
    assert.equal(sanitizeItem(hostile as never), null);
  });
});
