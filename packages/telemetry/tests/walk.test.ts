import assert from 'node:assert/strict';
import { isSecretName, redactText } from '../src/redact.ts';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import type { TransportItem } from '@grafana/faro-web-sdk';
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
      token: { '*': '*', '*#2': '*' },
    });
    assert.deepEqual(redactDeep({ key: 42 }), { key: '*' });
  });

  it('masks numbers, bigints and array elements under a sensitive key', () => {
    assert.deepEqual(redactDeep({ password: 123456 }), { password: '*' });
    assert.deepEqual(redactDeep({ session_id: 987654321 }), { session_id: '*' });
    assert.deepEqual(redactDeep({ sessionId: 123456789n }), { sessionId: '*' });
    assert.deepEqual(redactDeep({ token: ['abc123def', 5, 7n, ['nested']] }), { token: ['*', '*', '*', ['*']] });
    assert.deepEqual(redactDeep({ apiKey: ['k1', { note: 'x', url: 'https://a.test/x?code=abc' }] }), {
      apiKey: ['*', { '*': '*', '*#2': '*' }],
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
    assert.deepEqual(Object.values(payload['token'] as Record<string, unknown>).slice(0, 2), ['*', '*']);
    assert.ok(!Object.keys(payload['token'] as object).includes('password'));
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
      password: { '*': true, '*#2': null, '*#3': { '*': false, '*#2': '*' } },
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
describe('redactText v6.37 code and state keys', () => {
  const masked: ReadonlyArray<readonly [string, string]> = [
    ['otp_code: 834201 sent', 'otp_code: *'],
    ['{"mfa_code": "834201"}', '{"mfa_code": *}'],
    ['user_code: WDJB-MJHT', 'user_code: *'],
    ['relay_state: abc123xyz', 'relay_state: *'],
  ];
  const kept: readonly string[] = [
    'Error code: 404',
    'status code: 404',
    'status_code: 200',
    'code: ERR_NETWORK',
    'state: open',
    'readyState: complete',
    'game_state: lobby',
  ];

  for (const [input, expected] of masked) {
    it(`masks ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), expected);
    });
  }

  for (const input of kept) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }

  it('stays under 250 ms on 16 KB of prefixed code keys', () => {
    const input = 'x_code:'.repeat(2400);
    assert.ok(input.length >= 16000);
    const started = performance.now();
    redactText(input);
    assert.ok(performance.now() - started < 250);
  });
});

describe('spaced field names v6.37', () => {
  it('masks benign-looking prefixes before a secret word', () => {
    assert.deepEqual(
      redactDeep({
        'Monkey api_key': 'v0',
        'hotkey password': 'v1',
        'turkey session_id': 'v2',
        'donkey key': 'v3',
        'Hotkey pwd': 'v4',
      }),
      {
        'Monkey api_key': '*',
        'hotkey password': '*',
        'turkey session_id': '*',
        'donkey key': '*',
        'Hotkey pwd': '*',
      },
    );
  });

  it('keeps spaced names that are not secret', () => {
    const payload = { 'cache key': 'ok', monkey: 'ok', hotkey: 'ok' };
    assert.deepEqual(redactDeep(payload), { 'cache key': 'ok', monkey: 'ok', hotkey: 'ok' });
  });
});

describe('field names with punctuation and long prefixes', () => {
  const valueOf = (name: string): unknown => {
    const out = sanitizeItem({ type: 'log', meta: {}, payload: { [name]: 'ZqVal1' } } as never);
    assert.ok(out);
    return Object.values((out as unknown as { payload: Record<string, unknown> }).payload)[0];
  };

  for (const name of ['login(form)/password', 'user@host/password', 'form#login/token', 'a+b/secret', '100%/token', 'a'.repeat(80) + '_password']) {
    it(`masks ${JSON.stringify(name.length > 40 ? `${name.slice(0, 4)}...${name.slice(-9)}` : name)}`, () => {
      assert.equal(valueOf(name), '*');
    });
  }

  for (const name of ['cache key', 'i18n:key', 'homeCacheKey', 'a'.repeat(80) + '_monkey']) {
    it(`keeps ${JSON.stringify(name.length > 40 ? `${name.slice(0, 4)}...${name.slice(-7)}` : name)}`, () => {
      assert.equal(valueOf(name), 'ZqVal1');
    });
  }
});

describe('sanitizeItem copies and the Faro session meta', () => {
  type Sanitized = { meta: Record<string, any>; payload: Record<string, any> };
  const run = (meta: object, payload: object): Sanitized => {
    const out = sanitizeItem({ type: 'log', meta, payload } as never);
    assert.ok(out);
    return out as unknown as Sanitized;
  };

  it('does not mutate the item or anything reachable from it', () => {
    const session = { id: 'sess-123', attributes: { plan: 'pro' } };
    const meta = { session, user: { email: 'jane@corp.io' } };
    const payload = { context: { password: 'hunter2SECRETzq', note: 'a@b.com' }, list: ['a@b.com'] };
    const before = structuredClone({ meta, payload });
    const out = run(meta, payload);
    assert.deepEqual({ meta, payload }, before);
    assert.notEqual(out.meta['session'], session);
    assert.equal(out.payload['context']['password'], '*');
  });

  it('keeps the session id and attributes of the Faro meta', () => {
    const { meta } = run({ session: { id: 'sess-123', attributes: { plan: 'pro' } } }, {});
    assert.deepEqual(meta['session'], { id: 'sess-123', attributes: { plan: 'pro' } });
  });

  it('masks a string, number or bigint directly under session', () => {
    const { payload } = run({}, { session: 'abc123SECRET' });
    assert.equal(payload['session'], '*');
    assert.equal(run({}, { Session: 48291234 }).payload['Session'], '*');
    assert.equal(run({}, { session: 48291234n }).payload['session'], '*');
  });

  it('still masks the other session fields', () => {
    const { payload } = run({}, { session_id: 'abc123SECRET', sessionId: 'abc123SECRET', session_token: 'abc123SECRET', sessionKey: 'abc123SECRET' });
    assert.deepEqual(Object.values(payload), ['*', '*', '*', '*']);
  });

  it('walks a non-extensible object with a secret and colliding keys without touching it', () => {
    const data = Object.preventExtensions({ password: 'hunter2SECRETzq', 'a@b.com': 1, 'c@d.com': 2, keep: 'ok' });
    const before = structuredClone(data);
    const { payload } = run({}, { data });
    assert.deepEqual(data, before);
    assert.equal(payload['data']['password'], '*');
    assert.equal(payload['data']['keep'], 'ok');
    assert.equal(Object.keys(payload['data']).length, 4);
  });

  it('masks every leaf of an object that holds itself under a secret field', () => {
    const loop: Record<string, unknown> = {};
    loop['password'] = loop;
    loop['note'] = 'hunter2SECRETzq';
    const { payload } = run({}, { loop });
    assert.ok(!Object.keys(payload['loop']).includes('note'));
    assert.ok(!Object.values(payload['loop']).includes('hunter2SECRETzq'));
    const reversed: Record<string, unknown> = { note: 'hunter2SECRETzq' };
    reversed['password'] = reversed;
    const reversedOut = run({}, { reversed }).payload['reversed'];
    assert.ok(!Object.keys(reversedOut).includes('note'));
    assert.ok(!Object.values(reversedOut).includes('hunter2SECRETzq'));
  });

  it('keeps token counts and masks bare tokens', () => {
    const { payload } = run({}, { usage: { inputTokens: 12, max_tokens: 100, total_tokens: 7, OUTPUT_TOKENS: 3, 'prompt-tokens': 5 }, tokens: ['abc123SECRET'], token: 42 });
    assert.deepEqual(payload['usage'], { inputTokens: 12, max_tokens: 100, total_tokens: 7, OUTPUT_TOKENS: 3, 'prompt-tokens': 5 });
    assert.deepEqual(payload['tokens'], ['*']);
    assert.equal(payload['token'], '*');
  });
});

describe('keys under secret fields v6.41', () => {
  const marker = 'ghp_Zq9realTokenValue';
  const payloadOf = (payload: object): Record<string, any> => (sanitizeItem({ type: 'log', meta: {}, payload } as never) as unknown as { payload: Record<string, any> }).payload;

  it('masks keys and leaves under a secret field', () => {
    const out = payloadOf({ token: { [marker]: { a: 1 } } });
    assert.deepEqual(out, { token: { '*': { '*': '*' } } });
    assert.ok(!JSON.stringify(out).includes(marker));
  });

  it('keeps colliding masked keys apart', () => {
    assert.deepEqual(payloadOf({ secret: { aaa: 'x', bbb: 'y' } }), { secret: { '*': '*', '*#2': '*' } });
  });

  it('keeps null and booleans under a secret subtree', () => {
    assert.deepEqual(payloadOf({ password: { enabled: true, off: false, value: null } }), { password: { '*': true, '*#2': false, '*#3': null } });
  });

  it('leaves keys outside secret subtrees unchanged', () => {
    assert.deepEqual(payloadOf({ settings: { theme: 'dark', nested: { size: 'big' } } }), { settings: { theme: 'dark', nested: { size: 'big' } } });
  });

  it('masks keys and leaves of a cyclic secret subtree and does not mutate it', () => {
    const loop: Record<string, unknown> = {};
    loop['password'] = loop;
    loop[marker] = marker;
    const keysBefore = Object.keys(loop);
    const out = payloadOf({ loop });
    const seen = new Set<unknown>();
    const strings: string[] = [];
    const collect = (value: unknown): void => {
      if (typeof value === 'string') strings.push(value);
      if (typeof value !== 'object' || value === null || seen.has(value)) return;
      seen.add(value);
      for (const [key, inner] of Object.entries(value)) {
        strings.push(key);
        collect(inner);
      }
    };
    collect(out);
    assert.ok(!strings.some((text) => text.includes(marker)));
    assert.deepEqual(Object.keys(loop), keysBefore);
    assert.equal(loop[marker], marker);
  });

  it('does not mutate the input', () => {
    const payload = { token: { [marker]: { a: 1 } }, keep: { x: 'y' } };
    const before = structuredClone(payload);
    payloadOf(payload);
    assert.deepEqual(payload, before);
  });
});

describe('credential hosts, cookies, bearer chains and code words', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['//password:12345/next', '//password:*'],
    ['see //token:8342/x', 'see //token:*'],
    ['//auth:8080/x', '//auth:8080/x'],
    ['//db.internal:5432/app', '//db.internal:5432/app'],
    ['Cookie: theme=dark; sid=abc123', 'Cookie: *'],
    ['Set-Cookie: sid=abc123; Path=/', 'Set-Cookie: *'],
    ['cookie=abc123', 'cookie=*'],
    ['cookie_consent=accepted', 'cookie_consent=accepted'],
    ['cookieConsent: yes', 'cookieConsent: yes'],
    ['a'.repeat(60) + '_monkey=3', 'a'.repeat(60) + '_monkey=3'],
    ['a'.repeat(317) + '_monkey=3', 'a'.repeat(317) + '_monkey=3'],
    ['a'.repeat(300) + '_password=hunter2abc', 'a'.repeat(300) + '_password=*'],
    ['a'.repeat(300) + '_otp_code: 834201', 'a'.repeat(300) + '_otp_code: *'],
    ['Authorization: Bearer abcdefgh&otp_code: 834201', 'Authorization: Bearer *&otp_code: *'],
    [
      'Authorization: Bearer abcdefgh&id=1&password=Basic ZqSECRET9',
      'Authorization: Bearer *&password=*',
    ],
    ['password: correct\vhorse', 'password: *'],
    ['password: correct\fhorse', 'password: *'],
    ['versionCode=42', 'versionCode=42'],
    ['roomCode=AB12', 'roomCode=AB12'],
    ['combatState=idle', 'combatState=idle'],
    ['boardState=x', 'boardState=x'],
    ['team_code=x', 'team_code=*'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input.slice(0, 48))} to ${JSON.stringify(output.slice(-24))}`, () => {
      assert.equal(redactText(input), output);
    });
  }
});

describe('secret reachability does not depend on key order', () => {
  const MARKER = 'hunter2SECRETzq';

  interface Walked {
    meta: { session?: unknown };
    payload: {
      plain?: unknown;
      secret?: unknown;
      password?: object;
      reversed?: object;
      context?: { session?: unknown };
    };
  }

  function walked(payload: object, meta: object = {}): Walked {
    const out = sanitizeItem({ type: 'log', meta, payload } as unknown as TransportItem);
    assert.ok(out);
    return out as unknown as Walked;
  }

  function containsMarker(value: unknown, seen = new WeakSet<object>()): boolean {
    if (typeof value === 'string') return value.includes(MARKER);
    if (value === null || typeof value !== 'object') return false;
    if (seen.has(value)) return false;
    seen.add(value);
    return Object.entries(value).some(
      ([key, entry]) => key.includes(MARKER) || containsMarker(entry, seen),
    );
  }

  for (const keys of [
    ['password', 'reversed'],
    ['reversed', 'password'],
  ]) {
    it(`masks a cycle through a secret field with keys in the order ${keys.join(', ')}`, () => {
      const ring: { note: string; password?: unknown } = { note: MARKER };
      ring.password = ring;
      const payload: Record<string, unknown> = {};
      for (const key of keys) payload[key] = ring;
      const out = walked(payload).payload;
      assert.equal(containsMarker(out), false);
      assert.deepEqual(Object.keys(out.reversed ?? {}), Object.keys(out.password ?? {}));
    });
  }

  for (const keys of [
    ['token', 'loop'],
    ['loop', 'token'],
  ]) {
    it(`masks a loop shared with a token field with keys in the order ${keys.join(', ')}`, () => {
      const loop: { note: string; password?: unknown } = { note: MARKER };
      loop.password = loop;
      const payload: Record<string, unknown> = {};
      for (const key of keys) payload[key] = loop;
      assert.equal(containsMarker(walked(payload).payload), false);
    });
  }

  for (const keys of [
    ['plain', 'secret'],
    ['secret', 'plain'],
  ]) {
    it(`masks a container shared between a plain and a secret field, order ${keys.join(', ')}`, () => {
      const shared = { v: MARKER };
      const payload: Record<string, unknown> = {};
      for (const key of keys) payload[key] = shared;
      const out = walked(payload).payload;
      assert.deepEqual(out.plain, { '*': '*' });
      assert.deepEqual(out.secret, { '*': '*' });
    });
  }

  it('masks everything under a session key outside meta', () => {
    const out = walked({ context: { session: { value: MARKER, cookie: MARKER } } }).payload;
    assert.deepEqual(out.context?.session, { '*': '*', '*#2': '*' });
  });

  it('masks a session key nested in meta.session', () => {
    const out = walked({}, { session: { id: 's1', nested: { session: { value: MARKER } } } }).meta;
    assert.equal(containsMarker(out), false);
  });

  it('masks cookie fields', () => {
    const out = walked({ cookie: MARKER, headers: { Cookie: 'a=' + MARKER } }).payload;
    assert.deepEqual(out, { cookie: '*', headers: { Cookie: '*' } });
  });

  it('keeps meta.session and masks secrets inside it', () => {
    const session = { id: 'sess-1', attributes: { a: 'b' }, token: MARKER };
    const out = walked({}, { session }).meta;
    assert.deepEqual(out.session, { id: 'sess-1', attributes: { a: 'b' }, token: '*' });
  });

  it('masks meta.session when a secret field also reaches it', () => {
    const session = { id: 'sess-1' };
    const out = walked({ token: session }, { session }).meta;
    assert.deepEqual(out.session, { '*': '*' });
  });
});

describe('secret reachability across shared containers', () => {
  const MARKER = 'hunter2SECRETzq';

  function containsMarker(value: unknown, seen = new WeakSet<object>()): boolean {
    if (typeof value === 'string') return value.includes(MARKER);
    if (value === null || typeof value !== 'object') return false;
    if (seen.has(value)) return false;
    seen.add(value);
    return Object.entries(value).some(
      ([key, entry]) => key.includes(MARKER) || containsMarker(entry, seen),
    );
  }

  function sanitized(payload: object, meta: object = {}): unknown {
    const out = sanitizeItem({ type: 'log', meta, payload } as unknown as TransportItem);
    assert.ok(out);
    return out;
  }

  for (const order of ['secret first', 'plain first']) {
    it(`masks a cycle that reaches itself through a plain key and a secret subtree, ${order}`, () => {
      const node: { note: string; password?: unknown } = { note: MARKER };
      const hub: { b: unknown } = { b: node };
      node.password = hub;
      const payload = order === 'secret first' ? { token: hub, node } : { node, token: hub };
      assert.equal(containsMarker(sanitized(payload)), false);
    });
  }
});

describe('field names: primary and query keys, narrowed state words', () => {
  const payloadOf = (payload: object): Record<string, unknown> => {
    const out = sanitizeItem({ type: 'log', meta: {}, payload } as unknown as TransportItem);
    return (out as unknown as { payload: Record<string, unknown> }).payload;
  };

  for (const name of ['auth_request_state', 'oauth_client_state', 'api_key_code', 'orderState', 'x_keycode', 'cosmosPrimaryKey', 'storageAccountPrimaryKey', 'searchAdminQueryKey', 'cosmos_primary_key']) {
    it(`masks the field ${name}`, () => {
      assert.deepEqual(payloadOf({ [name]: 'SECRET99' }), { [name]: '*' });
    });
  }

  for (const name of ['primaryKey', 'primary_key', 'db.primaryKey', 'queryKey', 'keyCode', 'key_code', 'readyState', 'game_state']) {
    it(`keeps the field ${name}`, () => {
      assert.deepEqual(payloadOf({ [name]: 'ok' }), { [name]: 'ok' });
    });
  }

  it('keeps a primaryKey nested under a plain parent', () => {
    assert.deepEqual(payloadOf({ db: { primaryKey: 'ok', queryKey: 'ok' } }), {
      db: { primaryKey: 'ok', queryKey: 'ok' },
    });
  });
});
