import assert from 'node:assert/strict';
import { test } from 'node:test';
import { contentSecurityPolicy, definesFrom, withContentSecurityPolicy } from './csp.mjs';

const directive = (policy, name) =>
  policy.split('; ').find((part) => part.startsWith(`${name} `));

test('a same-origin build with telemetry off connects to itself only', () => {
  const policy = contentSecurityPolicy({ apiUrl: '', telemetryUrl: '' });
  assert.equal(directive(policy, 'connect-src'), "connect-src 'self'");
});

test('the API and the collector are allowed by origin, whatever path they carry', () => {
  const policy = contentSecurityPolicy({
    apiUrl: 'https://api.example.com/',
    telemetryUrl: 'https://rum.example.com/collect?key=1',
  });
  assert.equal(
    directive(policy, 'connect-src'),
    "connect-src 'self' https://api.example.com https://rum.example.com",
  );
});

test('the same origin twice is listed once', () => {
  const policy = contentSecurityPolicy({
    apiUrl: 'https://x.example.com',
    telemetryUrl: 'https://x.example.com/collect',
  });
  assert.equal(directive(policy, 'connect-src'), "connect-src 'self' https://x.example.com");
});

test('scripts, forms and the base element stay locked down', () => {
  const policy = contentSecurityPolicy({ apiUrl: '', telemetryUrl: '' });
  assert.equal(directive(policy, 'script-src'), "script-src 'self'");
  assert.equal(directive(policy, 'worker-src'), undefined);
  assert.equal(directive(policy, 'form-action'), "form-action 'none'");
  assert.equal(directive(policy, 'base-uri'), "base-uri 'self'");
});

test('the PDF preview is allowed to load its blob in the viewer plugin and nothing else', () => {
  const policy = contentSecurityPolicy({ apiUrl: '', telemetryUrl: '' });
  assert.equal(directive(policy, 'object-src'), 'object-src blob:');
  assert.equal(directive(policy, 'frame-src'), 'frame-src blob:');
  assert.equal(directive(policy, 'img-src'), "img-src 'self' blob:");
});

test('an empty value or a same-origin path adds no origin', () => {
  for (const value of ['', '/api', '/']) {
    const policy = contentSecurityPolicy({ apiUrl: value, telemetryUrl: '' });
    assert.equal(directive(policy, 'connect-src'), "connect-src 'self'", value);
  }
});

test('a value that is not an absolute http(s) URL fails the build', () => {
  for (const value of ['api.example.com', '//api.example.com', 'javascript:x', 'http://', 'ftp://x.example.com']) {
    assert.throws(
      () => contentSecurityPolicy({ apiUrl: value, telemetryUrl: '' }),
      /http|absolute/,
      value,
    );
    assert.throws(() => contentSecurityPolicy({ apiUrl: '', telemetryUrl: value }), Error, value);
  }
});

test('the policy lands right after the charset, before any resource is requested', () => {
  const html = '<head><meta charset="UTF-8" /><base href="/" /></head>';
  const out = withContentSecurityPolicy(html, "default-src 'self'");
  assert.equal(
    out,
    '<head><meta charset="UTF-8" /><meta http-equiv="Content-Security-Policy" content="default-src \'self\'" /><base href="/" /></head>',
  );
});

test('the minified self-closing form of the charset meta is found too', () => {
  const out = withContentSecurityPolicy('<head><meta charset="UTF-8"/><base href="/"/></head>', 'p');
  assert.match(out, /^<head><meta charset="UTF-8"\/><meta http-equiv="Content-Security-Policy" content="p" \/><base/);
});

test('a page without the anchor is refused', () => {
  assert.throws(() => withContentSecurityPolicy('<head></head>', "default-src 'self'"));
});

test('defines come from both argument forms and override the defaults', () => {
  const defines = definesFrom(
    ['--define', "STASHDEN_API_URL='https://a.example.com'", "--define=STASHDEN_TELEMETRY_URL='https://t.example.com/c'"],
    { STASHDEN_API_URL: 'http://localhost:3001', STASHDEN_TELEMETRY_URL: '', STASHDEN_SOURCE_URL: 'src' },
  );
  assert.deepEqual(defines, {
    STASHDEN_API_URL: 'https://a.example.com',
    STASHDEN_TELEMETRY_URL: 'https://t.example.com/c',
    STASHDEN_SOURCE_URL: 'src',
  });
});

test('double quotes around a define are stripped like single ones', () => {
  const defines = definesFrom(['--define', 'STASHDEN_API_URL="https://a.example.com"'], {});
  assert.equal(defines.STASHDEN_API_URL, 'https://a.example.com');
});

test('an empty define overrides a default', () => {
  const defines = definesFrom(['--define', "STASHDEN_API_URL=''"], { STASHDEN_API_URL: 'http://localhost:3001' });
  assert.equal(defines.STASHDEN_API_URL, '');
});
