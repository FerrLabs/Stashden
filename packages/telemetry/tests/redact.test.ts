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

describe('redactText', () => {
  const leaks: ReadonlyArray<readonly [string, string]> = [
    ['email_verification_code=834201', '834201'],
    ['user_mfa_code=834201', '834201'],
    ['user_code=WDJB-MJHT', 'WDJB'],
    ['sms_code=abc123', 'abc123'],
    ['backup_code=abc123', 'abc123'],
    ['params[user_code]=abc123', 'abc123'],
    ['daemonKey=abc123', 'abc123'],
    ['commonKey=abc123', 'abc123'],
    ['codeSig=abc123', 'abc123'],
    ['nodeSignature=abc123', 'abc123'],
    ['passkey=abc123', 'abc123'],
    ['see //password:aJ@+QvZ7 end', 'QvZ7'],
  ];

  for (const [input, marker] of leaks) {
    it(`masks ${JSON.stringify(input)}`, () => {
      assert.ok(!redactText(input).includes(marker), redactText(input));
    });
  }

  it('keeps the host-less userinfo lookalike text around the masked value', () => {
    assert.equal(redactText('see //password:aJ@+QvZ7 end'), 'see //password:*');
  });

  for (const input of ['error_code=500', 'status_code=200', 'errorCode=500', 'zip_code=59000', 'barcode=123', 'keyCode=13', 'readyState=complete']) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }

  const benignKeyWords = [
    'dickey', 'donkey', 'doohickey', 'flukey', 'flunkey', 'hickey', 'hockey', 'hokey', 'hookey', 'hotkey',
    'jockey', 'lackey', 'malarkey', 'mickey', 'monkey', 'pokey', 'rickey', 'smokey', 'turkey', 'turnkey', 'whiskey',
  ];
  for (const word of benignKeyWords) {
    it(`keeps ${word}=3`, () => {
      assert.equal(redactText(`${word}=3`), `${word}=3`);
    });
  }
});

describe('redactText', () => {
  const leaks: ReadonlyArray<readonly [string, string]> = [
    ['session_state=abc123', 'abc123'],
    ['relay_state=abc123', 'abc123'],
    ['token_code=abc123', 'abc123'],
    ['storage_key=abc123', 'abc123'],
  ];

  for (const [input, marker] of leaks) {
    it(`masks ${JSON.stringify(input)}`, () => {
      assert.ok(!redactText(input).includes(marker), redactText(input));
    });
  }

  for (const input of ['readyState=complete', 'game_state=lobby', 'httpStatusCode=200', '//auth:8080/x']) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }

  it('does not treat a bracketed host as userinfo', () => {
    assert.equal(redactText('https://[::1]:8080/users/jane@corp.io'), 'https://[::1]:8080/users/*');
  });
});

describe('redactText bare //host:port', () => {
  it('masks a port that is not followed by a path', () => {
    for (const input of ['//password:12345', 'see //password:12345', '//password:8080']) {
      const out = redactText(input);
      assert.ok(!out.includes('12345') && !out.includes('8080'), out);
    }
  });

  it('keeps a port followed by a path', () => {
    assert.equal(redactText('//auth:8080/x'), '//auth:8080/x');
  });
});

describe('redactText doubled equals and long or punctuated prefixes', () => {
  const exact: ReadonlyArray<readonly [string, string]> = [
    ['password==""\'\'""\'\'ZqSecret123', 'password==*'],
    ["code==''\"\"''\"\";", 'code==*;'],
    ['pwd==abcRET1', 'pwd==*'],
    ['https://admin:hunter2@bücher.example/x', 'https://bücher.example/x'],
    ['see //password:aJ@+QvZ7 end', 'see //password:*'],
  ];

  for (const [input, expected] of exact) {
    it(`turns ${JSON.stringify(input)} into ${JSON.stringify(expected)}`, () => {
      assert.equal(redactText(input), expected);
      assert.equal(redactText(expected), expected);
    });
  }

  for (const input of ['a == b', 'x==y', 'password===x1', 'password== x1', 'readyState=complete', 'game_state=lobby', 'iceGatheringState=complete', 'dial_code=33']) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }

  for (const [input, marker] of [
    ['device_state=ZqMark91', 'ZqMark91'],
    ['authRequestState=ZqMark91', 'ZqMark91'],
    ['phone_code=ZqMark91', 'ZqMark91'],
    ['phoneCode=ZqMark91', 'ZqMark91'],
    ['a'.repeat(300) + '_password=hunter2abc', 'hunter2abc'],
    ['a'.repeat(300) + '_otp_code: 834201', '834201'],
  ] as const) {
    it(`masks ${JSON.stringify(input.length > 60 ? `${input.slice(0, 12)}...${input.slice(-24)}` : input)}`, () => {
      assert.ok(!redactText(input).includes(marker), redactText(input));
    });
  }

  it('keeps a long benign prefix before a non-secret key', () => {
    const input = 'a'.repeat(300) + '_monkey=3';
    assert.equal(redactText(input), input);
  });

  it('stays under 250 ms on 16 KB behind a long prefix and a run of equals', () => {
    for (const input of ['a'.repeat(16000) + '_monkey=3', 'a'.repeat(16000) + '_password=hunter2abc', 'password' + '='.repeat(16000) + 'x']) {
      const started = performance.now();
      redactText(input);
      assert.ok(performance.now() - started < 250);
    }
  });
});

describe('redactText chained values, slashed passwords and non-ASCII spaces', () => {
  const exact: ReadonlyArray<readonly [string, string]> = [
    ['client_secret: monkeySecret=\nfoo=1', 'client_secret: *'],
    ['Error key: key: \n"Token X" end', 'Error key: * end'],
    ['DATABASE_URL=postgres://app:pa/ss#1@db.internal:5432/app', 'DATABASE_URL=postgres://db.internal:5432/app'],
    ['redis://default:ab/cd?ef@cache:6379/0', 'redis://cache:6379/0'],
    ['https://u:123?x@h.com/a', 'https://u:123'],
    ['password: correct horse', 'password: *'],
    ['password=correct horse', 'password=*'],
    ['password=a"b"c"d"e"f"g"h"hunter2abc', 'password=*'],
    ['pwd=^""""\'\'\'\'k', 'pwd=*'],
    ['auth=&""\'\'\'\'""\'', 'auth=*'],
  ];

  for (const [input, expected] of exact) {
    it(`turns ${JSON.stringify(input)} into ${JSON.stringify(expected)}`, () => {
      assert.equal(redactText(input), expected);
    });
  }

  for (const input of ['password=a"b"c"d"e"f"g"h"hunter2abc', 'pwd=^""""\'\'\'\'k', 'auth=&""\'\'\'\'""\'']) {
    it(`is idempotent on ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(redactText(input)), redactText(input));
    });
  }

  for (let code = 0x2000; code <= 0x200a; code++) {
    const space = String.fromCharCode(code);
    it(`masks a value split by U+${code.toString(16).toUpperCase()}`, () => {
      assert.equal(redactText(`password: correct${space}horse`), 'password: *');
      assert.equal(redactText(`password=correct${space}horse`), 'password=*');
    });
  }

  it('stays under 250 ms on unbalanced brackets', () => {
    for (const input of ['x['.repeat(8000), 'a]'.repeat(8000)]) {
      const started = performance.now();
      redactText(input);
      const elapsed = performance.now() - started;
      assert.ok(elapsed < 250, `${input.slice(0, 6)} took ${elapsed.toFixed(0)} ms`);
    }
  });
});

describe('redactText primary and query keys, narrowed state words', () => {
  const masked: ReadonlyArray<readonly [string, string]> = [
  ['auth_request_state=SECRET99', 'auth_request_state=*'],
  ['oauth_client_state=SECRET99', 'oauth_client_state=*'],
  ['api_key_code=SECRET99', 'api_key_code=*'],
  ['orderState=paid', 'orderState=*'],
  ['x_keycode=1', 'x_keycode=*'],
  ['cosmosPrimaryKey=SECRET99', 'cosmosPrimaryKey=*'],
  ['storageAccountPrimaryKey: SECRET99', 'storageAccountPrimaryKey: *'],
  ['searchAdminQueryKey=SECRET99', 'searchAdminQueryKey=*'],
  ['cosmos_primary_key=SECRET99', 'cosmos_primary_key=*'],
  ['email=a@b.com', '*'],
  ];
  const kept: readonly string[] = [
  'keyCode=13',
  'key_code=13',
  'readyState=complete',
  'game_state=lobby',
  ];

  for (const [input, expected] of masked) {
    it(`masks ${JSON.stringify(input)} as ${JSON.stringify(expected)}`, () => {
      assert.equal(redactText(input), expected);
    });
  }

  for (const input of kept) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }

  it('drops the password of a host-relative userinfo with a query', () => {
    assert.ok(!redactText('//host:abc/path?email=a@b.com').includes('abc'));
  });

  for (const unit of ['x_request_state=', 'cosmosPrimaryKey=', 'x_client_state=', 'x_keycode=']) {
    it(`stays under 250 ms on 16 KB of ${unit}`, () => {
      const input = unit.repeat(Math.ceil(16384 / unit.length));
      const started = performance.now();
      redactText(input);
      assert.ok(performance.now() - started < 250);
    });
  }
});

describe('bearer tail stops before a following secret key', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    [
      'Authorization: Bearer abcdefgh&user=passwd: ZqSECRET9',
      'Authorization: Bearer *=passwd: *',
    ],
    [
      'Authorization: Bearer abcdefgh&id=password: ZqSECRET9',
      'Authorization: Bearer *=password: *',
    ],
    [
      'Authorization: Bearer abcdefgh:token: ZqSECRET9',
      'Authorization: Bearer *:token: *',
    ],
    [
      'Authorization: Bearer abcdefgh=otp_code: ZqSECRET9',
      'Authorization: Bearer *=otp_code: *',
    ],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), output);
    });
  }
});

describe('secret words deep inside long keys', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['b'.repeat(100) + 'Password=hunter2', 'b'.repeat(100) + 'Password=*'],
    ['x_' + 'c'.repeat(100) + 'Token: abc', 'x_' + 'c'.repeat(100) + 'Token: *'],
    ['a'.repeat(317) + '_monkey=3', 'a'.repeat(317) + '_monkey=3'],
    ['b'.repeat(100) + '_error_code=500', 'b'.repeat(100) + '_error_code=500'],
    ['b'.repeat(100) + 'ErrorCode=500', 'b'.repeat(100) + 'ErrorCode=500'],
  ];

  rows.forEach(([input, output], index) => {
    it(`redacts row ${index}`, () => {
      assert.equal(redactText(input), output);
    });
  });

  const adversarial: ReadonlyArray<string> = [
    '[a'.repeat(8000) + ':',
    'a-'.repeat(8000) + ':',
  ];

  adversarial.forEach((input, index) => {
    it(`stays under 250 ms on 16 KB of adversarial keys ${index}`, () => {
      const started = performance.now();
      redactText(input);
      assert.ok(performance.now() - started < 250);
    });
  });
});

describe('a secret value stops before a following query key', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['token=abc?pwd= hunter2secret', 'token=*?pwd= *'],
    ['token=abc#pwd= hunter2secret', 'token=*#pwd= *'],
    ['token = abc?pwd = hunter2secret', 'token = *?pwd = *'],
    ['password=x?api_key = hunter2secret', 'password=*?api_key = *'],
    ['password=a?b', 'password=*'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  it('does not leak a secret behind a fragment key in a url', () => {
    assert.ok(!redactText('https://x.com/cb?token=abc#pwd= hunter2secret').includes('hunter2secret'));
  });

  it('stays under 250 ms on 16 KB of long words', () => {
    const started = performance.now();
    redactText(('a'.repeat(100) + ' ').repeat(160));
    assert.ok(performance.now() - started < 250);
  });

  it('stays under 250 ms and masks the value on repeated long-word keys', () => {
    const started = performance.now();
    const out = redactText(('a'.repeat(100) + 'Password=x ').repeat(140));
    assert.ok(performance.now() - started < 250);
    assert.ok(!out.includes('=x'));
  });
});

describe('bearer and colon values stop before a separated key', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['Bearer abcdefgh#otp_code: 834201', 'Bearer *#otp_code: *'],
    ['Bearer abcdefgh|otp_code: 834201', 'Bearer *|otp_code: *'],
    ['Bearer {otp_code: ZqSecret9x}', 'Bearer {otp_code: *}'],
    ['Bearer x|otp_code = ZqSecret9x', 'Bearer *|otp_code = *'],
    ['password: x|token:\nZqSecret9x', 'password: *|token:\n*'],
    ['password: |\n  ZqSecret9x\n  line2\nnext: ok', 'password: *\nnext: ok'],
    ['password: >-\n  ZqSecret9x', 'password: *'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input.slice(0, 48))} to ${JSON.stringify(output.slice(-24))}`, () => {
      assert.equal(redactText(input), output);
    });
  }
});

describe('nested bracket keys', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['filter[user[token]]=Zq1234567', 'filter[user[token]]=*'],
    ['[params[api_key]]=Zq1234567', '[params[api_key]]=*'],
    ['a[b[password]]=Zq1234567', 'a[b[password]]=*'],
    ['[params[api_key]]: Zq1234567', '[params[api_key]]: *'],
    ['a[b]=c', 'a[b]=c'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  it('stays under 250 ms on repeated closing brackets', () => {
    const start = performance.now();
    redactText('a]'.repeat(8000) + '=');
    assert.ok(performance.now() - start < 250);
  });
});

describe('backslashes inside bearer tokens', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['Bearer abcd\\/efgh1234ZZ', 'Bearer *'],
    ['Bearer abc\\\\def12345', 'Bearer *'],
    ['{"a":"Bearer abcdefgh123\\""}', '{"a":"Bearer *\\""}'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  it('stays under 250 ms on a run of backslashes', () => {
    const start = performance.now();
    redactText('Bearer ' + '\\'.repeat(16000));
    assert.ok(performance.now() - start < 250);
  });
});

describe('quoted values before a query key and keys behind long prefixes', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['token="abc"?pwd= hunter2secret', 'token=*?pwd= *'],
    ['token="abc"#pwd= hunter2secret', 'token=*#pwd= *'],
    ['a'.repeat(600) + '_password: hunter2', 'a'.repeat(600) + '_password: *'],
    ['a'.repeat(317) + '_monkey=3', 'a'.repeat(317) + '_monkey=3'],
    ['a'.repeat(61) + '_monkey=3', 'a'.repeat(61) + '_monkey=3'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input.slice(0, 48))} to ${JSON.stringify(output.slice(-24))}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  for (const [name, input] of [
    ['long words around a password key', ('a'.repeat(2100) + ' ').repeat(7) + 'password: x'],
    ['words at the longest skip length', ('a'.repeat(2047) + ' ').repeat(8)],
  ] as const) {
    it(`redacts ${name} within 250 ms`, () => {
      const start = performance.now();
      redactText(input);
      assert.ok(performance.now() - start < 250);
    });
  }
});

describe('cookie headers anywhere and narrowed auth bypass', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['  Cookie: theme=dark; sid=abc123', '  Cookie: *'],
    ['> Cookie: theme=dark; sid=abc123', '> Cookie: *'],
    ['headers: Cookie: theme=dark; sid=abc123', 'headers: Cookie: *'],
    ['req Cookie: theme=dark; connect.sid=s%3Aabc', 'req Cookie: *'],
    ['GET /x failed Cookie: theme=dark; sid=abc123', 'GET /x failed Cookie: *'],
    ['auth_hotkey=Zq9SECRET', 'auth_hotkey=*'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  for (const input of ['author_status_code=200']) {
    it(`keeps ${JSON.stringify(input)}`, () => {
      assert.equal(redactText(input), input);
    });
  }
});

describe('glued commas and semicolons inside secret values', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['password: p@ss;word123', 'password: *'],
    ['password: hunter,2abcSecret', 'password: *'],
    ['password: ;hunter2abc', 'password: *'],
    ['Basic dXNlcjpw;ass12', 'Basic *'],
    ['Bearer abc,defgh12345', 'Bearer *'],
    ['password: x, user: bob', 'password: *, user: bob'],
    ['password: x,user=bob', 'password: *,user=bob'],
    ['password: x; y', 'password: *; y'],
    ['{"password":"x","user":"bob"}', '{"password":*,"user":"bob"}'],
    ['password=correct horse', 'password=*'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  const timed: ReadonlyArray<readonly [string, string]> = [
    ['a run of glued commas', 'password: ' + 'a,'.repeat(8000)],
    ['a run of glued semicolons', 'Basic dXNlcjpw' + ';a'.repeat(8000)],
    ['long dotted words', ('a'.repeat(70) + '.').repeat(230)],
    ['repeated key prefixes', ('x_'.repeat(40) + 'token=').repeat(150)],
    ['a dotted run', 'a.'.repeat(8192)],
  ];

  for (const [name, input] of timed) {
    it(`stays under 250 ms on ${name}`, () => {
      const start = performance.now();
      redactText(input);
      assert.ok(performance.now() - start < 250);
    });
  }
});

describe('equals values spanning spaces', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['password=correct horse battery staple', 'password=*'],
    ['login failed password=abc for user bob', 'login failed password=*'],
    ['password=x user=bob', 'password=* user=bob'],
    ['password=correct horse; user=bob', 'password=*; user=bob'],
    ['password=x user: bob', 'password=* user: bob'],
    ['token=abc, next', 'token=*, next'],
    ['token=abc def ghi=1', 'token=* ghi=1'],
    ['password=hunter2\nnext line', 'password=*\nnext line'],
    ['https://h.com/a?token=x&y=1', 'https://h.com/a'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  const timed: ReadonlyArray<readonly [string, string]> = [
    ['a long run of spaced words after an equals sign', 'password=' + 'a '.repeat(8000)],
    ['repeated spaced assignments', 'token=a b '.repeat(1600)],
  ];

  for (const [name, input] of timed) {
    it(`stays under 250 ms on ${name}`, () => {
      const start = performance.now();
      redactText(input);
      assert.ok(performance.now() - start < 250);
    });
  }
});

describe('userinfo, yaml blocks, glued separators and query values', () => {
  const rows: ReadonlyArray<readonly [string, string]> = [
    ['postgres://u:2/XKGNxq6c9JxDvt@host/db', 'postgres://host/db'],
    ['redis://:12/ab34cd56@cache:6379', 'redis://cache:6379'],
    ['https://admin:2024/Summer!x@db.internal/x', 'https://db.internal/x'],
    ['https://[::1]:8080/users/jane@corp.io', 'https://[::1]:8080/users/*'],
    ['https://h.com:8080/a/b', 'https://h.com:8080/a/b'],
    ['password: |\n  ZqLine1\n\n  ZqLine2\nnext: ok', 'password: *\nnext: ok'],
    ['password: |\n\n  ZqLine1', 'password: *'],
    ['password: |2\n  ZqLine1\nnext: ok', 'password: *\nnext: ok'],
    ['password: | # pw\n  ZqLine1', 'password: *'],
    ['  password: |\n    s3cret\n  user: bob', '  password: *\n  user: bob'],
    ['password: Zq7x,K9m:P2wLr8', 'password: *'],
    ['password: x,user=bob', 'password: *,user=bob'],
    ['password: x, user: bob', 'password: *, user: bob'],
    ['Basic dXNlcjpwYXNz,status=ok', 'Basic *,status=ok'],
    ['Basic dXNlcjpw;ass12', 'Basic *'],
    ['zip-code=57', 'zip-code=57'],
    ['x-api-key=abc123', 'x-api-key=*'],
    ['GET https://api.example/x?token=abc failed', 'GET https://api.example/x failed'],
    ['see https://a.com/p?api_key=x now', 'see https://a.com/p now'],
    ['token=abc?pwd= hunter2secret', 'token=*?pwd= *'],
    ["code==''\"\"''\"\";", 'code==*;'],
    ['password=correct horse:staple', 'password=*'],
    ['password=correct horse:battery staple', 'password=*'],
    ['password=x user: bob', 'password=* user: bob'],
    ['error.code=3', 'error.code=3'],
    ['error-code=3', 'error-code=3'],
    ['loading-state=1', 'loading-state=1'],
    ['zip-code=57', 'zip-code=57'],
    ['user.password=x1abc', 'user.password=*'],
    ['a.b.token=x1abc', 'a.b.token=*'],
    ['x.code=1', 'x.code=*'],
    ['password: abc?user[token] = ZqSECRET9x', 'password: *?user[token] = *'],
    ['password : abc?[token] =ZqSECRET9x', 'password : *?[token] =*'],
    ['Bearer abcdefgh?token=ZqSECRET9x', 'Bearer *?token=*'],
    ['password: what?', 'password: *'],
    ['cmd --password=hunter2', 'cmd --password=*'],
    ['-Dtoken=abc', '-Dtoken=*'],
    ['--api-key=abc', '--api-key=*'],
    ['- password: |\n  x\n  y', '- password: *'],
    ['- password: |\n  x\nnext: 1', '- password: *\nnext: 1'],
    ['  - token: >-\n      a\n      b\n  - name: z', '  - token: *\n  - name: z'],
    ['- password: |\n  ZqLine1\n  ZqLine2\nnext: ok', '- password: *\nnext: ok'],
    ['x - password: |\n  ZqLine1', 'x - password: *'],
    ['a: b password: |\n  ZqLine1', 'a: b password: *'],
    ['  - password: |\n    ZqLine1', '  - password: *'],
    ['- password: >\n  ZqLine1', '- password: *'],
    ['x\n  password: |\n    s\n  next: 1', 'x\n  password: *\n  next: 1'],
    ['note: a | b', 'note: a | b'],
    ['Server=db;User Id=sa;Password=correct horse battery staple;', 'Server=db;User Id=sa;Password=*;'],
    ['user=bob;password=correct horse battery staple', 'user=bob;password=*'],
    ['a=1&password=correct horse battery staple', 'a=1&password=*'],
    ['user=bob&password=correct horse', 'user=bob&password=*'],
    ['GET https://api.example/x?token=abc failed', 'GET https://api.example/x failed'],
    ['see https://a.com/p?a=1&api_key=x now', 'see https://a.com/p now'],
    ['app.integrations.payments.providers.stripe.webhooks.endpoint.signing.secret=whsec_abc', 'app.integrations.payments.providers.stripe.webhooks.endpoint.signing.secret=*'],
    ['.password=hunter2', '.password=*'],
    ['see .token=abc123', 'see .token=*'],
    ['java -Dpassword=hunter2 -jar app.jar', 'java -Dpassword=* -jar app.jar'],
    ['Basic YWRtaW46cGFzcw==,Basic Ym9iOnNlY3JldA==', 'Basic *,Basic *'],
    ['Bearer tok_abcdefghij,Bearer tok_klmnopqrst', 'Bearer *,Bearer *'],
    ['Authorization: Bearer abcdefghijkl,Basic YWRtaW46cGFzcw==', 'Authorization: Bearer *,Basic *'],
    ['password=hunter2 Basic YWRtaW46cGFzcw==', 'password=* Basic *'],
    ['password=hunter2,Basic YWRtaW46cGFzcw==', 'password=*,Basic *'],
    ['http://localhost:4873/@babel/core', 'http://localhost:4873/@babel/core'],
    ['https://medium.com:443/@alice/post', 'https://medium.com:443/@alice/post'],
    ['postgres://u:2/XKGNxq6c9JxDvt@host/db', 'postgres://host/db'],
    ['- password: |\n    x\n  user: bob', '- password: *\n  user: bob'],
    ['{"password":"@home.Town!2024"}', '{"password":*}'],
    ['{"password": "@a.b!hunter2abc"}', '{"password": *}'],
    ['password: ab);hunter2abc', 'password: *'],
    ['token: x],hunter2abc', 'token: *'],
    ['password: );T4NZq7Mk]D', 'password: *'],
    ['contact "jane doe"@corp.io now', 'contact * now'],
    ['{"a":{"token":"x"},"b":1}', '{"a":{"token":*},"b":1}'],
    ['password: x}, user: bob', 'password: *}, user: bob'],
    ['traceState=abc', 'traceState=abc'],
    ['trace_state=x', 'trace_state=x'],
    ['https://h.com:8080/jane@corp.io', 'https://h.com:8080/*'],
    ['https://cdn.example.com:8443/lib@1.2.3/x.js failed', 'https://cdn.example.com:8443/*/x.js failed'],
    ['(password)=Zq123456', '(password)=*'],
    ['{password}=Zq123456', '{password}=*'],
    ['<client_secret>=ZqKM1234', '<client_secret>=*'],
    ['<db_password>: ZqmLQ!-%spg14', '<db_password>: *'],
    ['pwd==*', 'pwd==*'],
    ['Authorization: Token Zq4WZwg5xZKWfyyh user=bob', 'Authorization: * user=bob'],
    ['password: Zq4WZwg5xZKW user=bob', 'password: * user=bob'],
    ['password: correct horse', 'password: *'],
    ['GET https://h.com/a?' + 'x'.repeat(200) + '&token=abc failed now', 'GET https://h.com/a failed now'],
    ['  password: |\n  s3cret', '  password: *\n  s3cret'],
  ];

  for (const [input, output] of rows) {
    it(`redacts ${JSON.stringify(input)} to ${JSON.stringify(output)}`, () => {
      assert.equal(redactText(input), output);
    });
  }

  const timed: ReadonlyArray<readonly [string, string]> = [
    ['a yaml block of 4000 blank-separated lines', 'password: |\n' + '  x\n\n'.repeat(4000)],
    ['4000 digit-led path segments before an at sign', '//' + 'a:1/'.repeat(4000) + '@'],
    ['16000 spaces before a yaml block key', ' '.repeat(16000) + 'password: |\n  x'],
    ['8000 question marks before an ampersand', '?'.repeat(8000) + '&'],
    ['5000 query fragments of question mark, letter, ampersand', '?a&'.repeat(5000)],
    ['8000 dotted segments before an equals sign', 'a.'.repeat(8000) + '='],
  ];

  for (const [name, input] of timed) {
    it(`stays under 250 ms on ${name}`, () => {
      const start = performance.now();
      redactText(input);
      assert.ok(performance.now() - start < 250);
    });
  }
});
