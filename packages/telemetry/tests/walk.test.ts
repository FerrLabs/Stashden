import assert from 'node:assert/strict';
import { isSecretName, redactText } from '../src/redact.ts';
import { readFileSync } from 'node:fs';
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

  it('masks every leaf under a sensitive key including numbers', () => {
    assert.deepEqual(redactDeep({ token: { value: 'x', note: 'password=abc12345' } }), {
      token: { value: '*', note: '*' },
    });
    assert.deepEqual(redactDeep({ key: 42 }), { key: '*' });
  });

  it('masks numbers, bigints and array elements under a sensitive key', () => {
    assert.deepEqual(redactDeep({ password: 123456 }), { password: '*' });
    assert.deepEqual(redactDeep({ session_id: 987654321 }), { session_id: '*' });
    assert.deepEqual(redactDeep({ sessionId: 123456789n }), { sessionId: '*' });
    assert.deepEqual(redactDeep({ token: ['abc123def', 5, 7n, ['nested']] }), { token: ['*', '*', '*', ['*']] });
    assert.deepEqual(redactDeep({ apiKey: ['k1', { note: 'x', url: 'https://a.test/x?code=abc' }] }), {
      apiKey: ['*', { note: '*', url: '*' }],
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

  it('matches benign field exclusions on the key suffix only', () => {
    assert.deepEqual(redactDeep({ homeCacheKey: 'a', x: { idempotencyKey: 'b' }, user: { authType: 'sso' } }), {
      homeCacheKey: 'a',
      x: { idempotencyKey: 'b' },
      user: { authType: 'sso' },
    });
    assert.deepEqual(redactDeep({ daemonKey: 'a', commonKey: 'b', codeSig: 'c', escrowKey: 'd' }), {
      daemonKey: '*',
      commonKey: '*',
      codeSig: '*',
      escrowKey: '*',
    });
  });

  it('masks plural and prefixed code and state field names', () => {
    assert.deepEqual(
      redactDeep({ storageKey: 'abc', tokens: ['x'], apiKeys: 'k', otp_code: 123456, mfaCode: 'm', session_state: 's' }),
      { storageKey: '*', tokens: ['*'], apiKeys: '*', otp_code: '*', mfaCode: '*', session_state: '*' },
    );
  });

  it('keeps bare and benign code and state field names', () => {
    const input = { code: 'a', state: 'b', errorCode: 'c', httpStatusCode: 'd', readyState: 'e', homeCacheKeys: 'f' };
    assert.deepEqual(redactDeep(input), input);
  });

  it('keeps every key that redacts to the same string', () => {
    assert.deepEqual(redactDeep({ 'a@b.com': 1, 'c@d.com': 2 }), { '*': 1, '*#2': 2 });
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

describe('aliased containers and metric fields', () => {
  const run = (meta: object, payload: object) => {
    const out = sanitizeItem({ type: 'log', meta, payload } as never);
    assert.ok(out);
    return out as unknown as { meta: Record<string, unknown>; payload: Record<string, unknown> };
  };

  it('masks a container reached under a secret field after a benign field', () => {
    const shared = ['hunter2'];
    const { payload } = run({}, { list: shared, password: shared });
    assert.deepEqual(payload['password'], ['*']);
  });

  it('masks a container reached under a secret field before a benign field', () => {
    const shared = ['hunter2'];
    const { payload } = run({}, { password: shared, list: shared });
    assert.deepEqual(payload['password'], ['*']);
  });

  it('masks a container shared between meta and payload', () => {
    const shared = ['hunter2'];
    const { payload } = run({ list: shared }, { password: shared });
    assert.deepEqual(payload['password'], ['*']);
  });

  it('masks a container shared between meta and payload with the secret field in meta', () => {
    const shared = ['hunter2'];
    const { meta } = run({ token: shared }, { list: shared });
    assert.deepEqual(meta['token'], ['*']);
  });

  it('terminates on a self-referencing object under a secret field', () => {
    const loop: Record<string, unknown> = { note: 'x', password: 'hunter2' };
    loop['self'] = loop;
    const { payload } = run({}, { token: loop });
    assert.deepEqual((payload['token'] as Record<string, unknown>)['password'], '*');
  });

  it('terminates on a self-referencing array under a secret field', () => {
    const loop: unknown[] = ['hunter2'];
    loop.push(loop);
    const { payload } = run({}, { token: loop });
    assert.deepEqual((payload['token'] as unknown[])[0], '*');
  });

  it('keeps numbers and bigints under measurement-named secret fields', () => {
    const fields = { authentication_duration: 120, authLatencyMs: 8, tokenTtl: 3600, authExpiresAt: 1700000000, authSessionAgeSecs: 5n };
    const { payload } = run({}, { ...fields });
    assert.deepEqual(payload, fields);
  });

  it('masks numbers under other secret fields', () => {
    const { payload } = run({}, { password: 4829, pin_key: 1234, sessionId: 77n, authCode: 5 });
    assert.deepEqual(payload, { password: '*', pin_key: '*', sessionId: '*', authCode: '*' });
  });

  it('masks strings under measurement-named secret fields', () => {
    const { payload } = run({}, { authentication_duration: 'abc123', authLatencyMs: '8' });
    assert.deepEqual(payload, { authentication_duration: '*', authLatencyMs: '*' });
  });
});

describe('walk keys, non-plain values and metric names', () => {
  const run = (meta: object, payload: object) => {
    const out = sanitizeItem({ type: 'log', meta, payload } as never);
    assert.ok(out);
    return out as unknown as { meta: Record<string, unknown>; payload: Record<string, unknown> };
  };
  const payloadOf = (payload: object): Record<string, unknown> => run({}, payload).payload;

  it('masks numbers in an array reached under a metric-named secret field, in either order', () => {
    const pins = [4829];
    assert.deepEqual(payloadOf({ authCount: pins, password: pins }), { authCount: ['*'], password: ['*'] });
    const reversed = [4829];
    assert.deepEqual(payloadOf({ password: reversed, authCount: reversed }), { password: ['*'], authCount: ['*'] });
  });

  it('masks an array shared between meta and payload under a secret name on either side', () => {
    const pins = [4829];
    const first = run({ authCount: pins }, { password: pins });
    assert.deepEqual(first.meta['authCount'], ['*']);
    assert.deepEqual(first.payload['password'], ['*']);
    const others = [4829];
    const second = run({ password: others }, { authCount: others });
    assert.deepEqual(second.meta['password'], ['*']);
    assert.deepEqual(second.payload['authCount'], ['*']);
  });

  it('keeps a number stored directly under a metric-named secret field', () => {
    assert.deepEqual(payloadOf({ authCount: 3, authLatencyMs: 8, authentication_duration: 120, tokenTtl: 3600 }), { authCount: 3, authLatencyMs: 8, authentication_duration: 120, tokenTtl: 3600 });
  });

  it('masks a number under a name that only ends in a metric suffix by accident', () => {
    assert.deepEqual(payloadOf({ authSms: 482913 }), { authSms: '*' });
  });

  it('masks values under secret names written with brackets, colons and slashes', () => {
    const out = payloadOf({ 'user[password]': 'hunter2', 'db:password': 'hunter2', 'credentials[0].token': 'hunter2', 'a/b/secret': 'hunter2', name: 'ok' });
    assert.deepEqual(out, { 'user[password]': '*', 'db:password': '*', 'credentials[0].token': '*', 'a/b/secret': '*', name: 'ok' });
  });

  const nonPlain: ReadonlyArray<readonly [string, () => unknown]> = [
    ['a String object', () => new String('hunter2')],
    ['a class instance', () => new (class Credentials { readonly value = 'hunter2'; })()],
    ['a typed array', () => new Uint8Array([1, 2, 3])],
    ['a Date', () => new Date(0)],
    ['a Map', () => new Map([['k', 'hunter2']])],
    ['a Set', () => new Set(['hunter2'])],
  ];
  for (const [label, make] of nonPlain) {
    it(`masks ${label} under a secret name`, () => {
      const out = payloadOf({ password: make(), authCount: make(), token: [make()] });
      assert.deepEqual(out, { password: '*', authCount: '*', token: ['*'] });
    });
  }

  it('keeps null, undefined and booleans under a secret name', () => {
    assert.deepEqual(payloadOf({ password: null, token: undefined, secret: true, apiKey: false }), { password: null, token: undefined, secret: true, apiKey: false });
  });

  it('redacts object keys that carry a secret', () => {
    const out = payloadOf({ context: { 'https://h.com/cb?token=ZqSECRET9': 'clicked', 'ZqSECRET9@example.com': 1 } });
    assert.ok(!JSON.stringify(out).includes('ZqSECRET9'));
  });

  it('keeps both values when two keys collide after redaction', () => {
    const out = payloadOf({ context: { 'a?token=ZqSECRETa': 'first', 'a?token=ZqSECRETb': 'second' } });
    const context = out['context'] as Record<string, unknown>;
    assert.deepEqual(Object.values(context), ['first', 'second']);
    assert.ok(!JSON.stringify(out).includes('ZqSECRET'));
  });

  it('keeps ordinary keys untouched', () => {
    assert.deepEqual(payloadOf({ context: { userId: 'u1', 'x-request-id': 'r1' } }), { context: { userId: 'u1', 'x-request-id': 'r1' } });
  });

  it('keeps hotkey and error codes and masks apikey in text', () => {
    assert.deepEqual(redactText('hotkey: Ctrl'), 'hotkey: Ctrl');
    assert.deepEqual(redactText('errorCode=E42'), 'errorCode=E42');
    assert.ok(!redactText('apikey=S3cret').includes('S3cret'));
  });

  it('has no regex lookbehind in the module source', () => {
    const source = readFileSync(new URL('../src/redact.ts', import.meta.url), 'utf8');
    assert.ok(!source.includes('(?<'));
  });
});

describe('secret subtrees, metric names and key normalization', () => {
  const SECRET = 'hunter2abc';
  const subtreeLeaks: ReadonlyArray<readonly [object, string]> = [
    [{ password: { value: SECRET } }, SECRET],
    [{ token: [{ value: SECRET }] }, SECRET],
    [{ apiKey: { primary: SECRET } }, SECRET],
    [{ client_secret: { current: { raw: SECRET } } }, SECRET],
    [{ password: [[SECRET]] }, SECRET],
    [{ secret: { n: 48291234 } }, '48291234'],
    [{ token: { x: 3 } }, '3'],
    [{ auth_count: { x: 3 } }, '3'],
  ];

  for (const [payload, marker] of subtreeLeaks) {
    it(`masks every leaf of ${JSON.stringify(payload)}`, () => {
      const out = JSON.stringify(redactDeep(structuredClone(payload)));
      assert.ok(!out.includes(marker), out);
    });
  }

  it('keeps shape, booleans and null under a secret field and leaves other fields alone', () => {
    assert.deepEqual(redactDeep({ password: { enabled: true, value: null, deep: { ok: false, v: 'x' } } }), {
      password: { enabled: true, value: null, deep: { ok: false, v: '*' } },
    });
    assert.deepEqual(redactDeep({ settings: { theme: 'dark' } }), { settings: { theme: 'dark' } });
  });

  it('keeps numbers under metric names in any case and masks other secret numbers', () => {
    assert.deepEqual(redactDeep({ AUTH_COUNT: 3, auth_count: 3, authCount: 3, SESSION_DURATION_MS: 7, TOKEN_TTL: 9 }), {
      AUTH_COUNT: 3,
      auth_count: 3,
      authCount: 3,
      SESSION_DURATION_MS: 7,
      TOKEN_TTL: 9,
    });
    assert.deepEqual(redactDeep({ AUTH_TOKEN: 3 }), { AUTH_TOKEN: '*' });
  });

  it('keeps colliding redacted keys under a secret field', () => {
    assert.deepEqual(redactDeep({ password: { 'a@b.com': 'x', 'c@d.com': 'y' } }), { password: { '*': '*', '*#2': '*' } });
  });

  it('does not treat separator-joined benign key names as secret', () => {
    for (const key of ['i18n:key', 'cache:key', 'cache/key']) assert.equal(isSecretName(key), false, key);
  });

  it('treats bracketed, slashed, colon-joined and dotted secret names as secret', () => {
    for (const key of ['user[password]', 'headers/authorization', 'a:b:token', 'x.password'])
      assert.equal(isSecretName(key), true, key);
  });
});
