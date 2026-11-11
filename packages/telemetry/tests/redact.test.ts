import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { redactText } from '../src/redact.ts';
import { keptInputs, leakInputs } from './cases.ts';

const LEAK_MARKER = 'abc';

describe('redactText query and host handling', () => {
  it('keeps the host and cuts the query when the query contains an @', () => {
    assert.equal(redactText('https://app.example.com/x?next=a@b.c'), 'https://app.example.com/x');
  });

  it('keeps the host when the query follows the host directly', () => {
    assert.equal(redactText('https://app.example.com?next=a@b.c'), 'https://app.example.com');
    assert.equal(redactText('https://app.example.com:8443?next=a@b.c'), 'https://app.example.com:8443');
  });

  it('keeps the port of a host whose name looks sensitive', () => {
    assert.equal(redactText('failed GET https://authserver:11020#f while (x)'), 'failed GET https://authserver:11020 while (x)');
    assert.equal(redactText('GET http://authelia:9091 -> 502'), 'GET http://authelia:9091 -> 502');
  });

  it('parses userinfo like a URL parser', () => {
    assert.equal(redactText('https://u:123?x@h.com/a'), 'https://u:123');
    assert.equal(redactText('http://db:5432?redirect=x@y'), 'http://db:5432');
    assert.equal(redactText('http://api-gw:3000#a@b'), 'http://api-gw:3000');
    assert.equal(redactText('http://authelia:9091: 504 Gateway Timeout'), 'http://authelia:9091: 504 Gateway Timeout');
    assert.equal(redactText('email=a@b.com'), '*');
    assert.equal(redactText('code == 200'), 'code == 200');
    assert.equal(redactText('{"a":{"password":"x"}}'), '{"a":{"password":*}}');
    assert.equal(redactText('http://u:+[)v~5-pc[g1A|@[::1]:39503/x'), 'http://[::1]:39503/x');
  });

  it('keeps the port of a host with a sensitive name when a quote follows', () => {
    assert.equal(redactText('https://authserver:11020"'), 'https://authserver:11020"');
  });

  it('keeps a port only right after a colon', () => {
    assert.ok(!redactText('x //pwd: 1234abc').includes('1234'));
    assert.ok(!redactText('x //secret:\n12345abc').includes('12345'));
  });

  it('keeps the host when the fragment contains an @', () => {
    const out = redactText('open https://app.example.com/x#next=a@b.c now');
    assert.ok(out.includes('https://app.example.com/x'), out);
    assert.ok(!out.includes('a@b.c'), out);
  });

  it('drops URL userinfo and keeps the host', () => {
    const out = redactText('connect https://user:pw@host.example.com/db failed');
    assert.ok(!out.includes('pw'), out);
    assert.ok(!out.includes('user'), out);
    assert.ok(out.includes('https://host.example.com/db'), out);
  });

  it('keeps host:port while masking a numeric password', () => {
    assert.equal(redactText('connect host.example.com:8443'), 'connect host.example.com:8443');
    assert.equal(redactText('https://host.example.com:8443/x'), 'https://host.example.com:8443/x');
    const masked = redactText('login password: 8443');
    assert.ok(!masked.includes('8443'), masked);
  });

  it('does not treat a numeric secret after a URL as a port', () => {
    const out = redactText('https://host.example.com/x password: 48291');
    assert.ok(!out.includes('48291'), out);
  });

  it('masks quoted query values', () => {
    for (const input of [
      'GET /a?x="secret value abc"&y=1',
      "GET /a?x='secret value abc'&y=1",
      'GET https://h.example.com/a?x="secret value abc" failed',
    ]) {
      const out = redactText(input);
      assert.ok(!out.includes(LEAK_MARKER), `${input} -> ${out}`);
    }
  });
});

describe('redactText credentials', () => {
  it('masks Basic credentials', () => {
    const out = redactText('Authorization Basic YQ==');
    assert.ok(!out.includes('YQ=='), out);
    assert.ok(!redactText('Basic YQ==').includes('YQ=='));
  });

  it('masks Basic credentials that follow a key', () => {
    for (const input of ['token: Basic abcdefghijk==', 'password=Basic Zm9vOmJhcabc==', 'token: Basic abc12345']) {
      const out = redactText(input);
      assert.ok(!out.includes('abc'), `${input} -> ${out}`);
    }
  });

  it('masks a Bearer token and a JWT', () => {
    assert.ok(!redactText('Authorization: Bearer abcOR5iMadOdPDXCB3').includes('abc'));
    assert.ok(!redactText('jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4abc.sigabc').includes('abc'));
  });

  it('masks every sensitive pair when several share a line', () => {
    const out = redactText('token=abc12345,password="hunter22abc" key=abcdefgh');
    assert.ok(!out.includes('abc'), out);
    assert.ok(!out.includes('hunter22'), out);
  });
});

describe('redactText tables', () => {
  for (const input of leakInputs) {
    it(`masks ${JSON.stringify(input.slice(0, 60))}`, () => {
      const out = redactText(input);
      assert.ok(!out.includes(LEAK_MARKER), `${JSON.stringify(input)} -> ${JSON.stringify(out)}`);
    });
  }

  for (const input of keptInputs) {
    it(`keeps ${JSON.stringify(input.slice(0, 60))}`, () => {
      assert.equal(redactText(input), input);
    });
  }
});

describe('redactText bounds', () => {
  it('never returns more than 2048 characters', () => {
    for (const input of ['x'.repeat(256000), 'a.'.repeat(32000), '?a='.repeat(21000), 'token=abc '.repeat(5000)]) {
      assert.ok(redactText(input).length <= 2048);
    }
  });

  it('masks a secret that sits across the input cut', () => {
    for (let n = 2000; n < 2060; n++) {
      const out = redactText(`${'x'.repeat(n)} token=SECRETSECRETSECRET`);
      assert.ok(!/CRET/.test(out), `n=${n}`);
    }
  });

  it('is idempotent', () => {
    for (const input of leakInputs) {
      const once = redactText(input);
      assert.equal(redactText(once), once, input);
    }
  });

  it('handles 16 KB of adversarial input in under 250 ms', () => {
    const inputs = [
      '?a= '.repeat(4096),
      'a@'.repeat(8192),
      'token='.repeat(2730),
      'Basic '.repeat(2730),
      'k=' + 'v'.repeat(16000),
      'https://u:p@'.repeat(1365),
      'password: '.repeat(1638),
      'bearer '.repeat(2340),
    ];
    for (const input of inputs) {
      const started = performance.now();
      redactText(input);
      const elapsed = performance.now() - started;
      assert.ok(elapsed < 250, `${input.slice(0, 20)} took ${elapsed.toFixed(0)} ms`);
    }
  });
});

describe('redactText bracketed keys, multi-chunk quotes and whitespace runs', () => {
  const leaks: ReadonlyArray<readonly [string, string]> = [
    ['user[password]=hunter2abc', 'hunter2abc'],
    ['{"user[password]":"hunter2abc"}', 'hunter2abc'],
    ['user[password]: hunter2abc', 'hunter2abc'],
    ['params[api_key]=x1abc', 'x1abc'],
    ['password=\'it\'\'s\'\'my\'\'pass\'\'hunter2abc\'', 'hunter2abc'],
  ];

  for (const [input, marker] of leaks) {
    it(`masks ${JSON.stringify(input)}`, () => {
      assert.ok(!redactText(input).includes(marker));
    });
  }

  it('is idempotent on auth=""\'\'""\'\'{', () => {
    const once = redactText('auth=""\'\'""\'\'{');
    assert.equal(redactText(once), once);
  });

  const runs: ReadonlyArray<readonly [string, string]> = [
    ['spaces after a token word', 'token: a' + ' '.repeat(16000) + 'x'],
    ['spaces after a bare colon', 'token:' + ' '.repeat(16000)],
    ['spaces inside a password value', 'password=ab' + ' '.repeat(16000) + 'x'],
    ['spaces after an auth scheme', 'Authorization: Token' + ' '.repeat(16000) + 'x'],
    ['alternating tabs and spaces', 'token: a' + '\t \t'.repeat(5400) + 'x'],
    ['a newline then spaces', 'token:\n' + ' '.repeat(16000) + 'x'],
  ];

  for (const [label, input] of runs) {
    it(`stays under 250 ms on 16 KB of ${label}`, () => {
      assert.ok(input.length >= 16000);
      const started = performance.now();
      redactText(input);
      assert.ok(performance.now() - started < 250);
    });
  }
});
