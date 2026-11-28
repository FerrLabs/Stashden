import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  arrivalAt,
  loopbackAddress,
  parseHandoff,
  serializeHandoff,
} from '../src/app/provider.ts';

const nonce = '0123456789abcdef0123456789abcdef';

test('a code and state in the query is the identity provider coming back', () => {
  assert.deepEqual(arrivalAt('?code=abc&state=xyz'), {
    kind: 'returned',
    returned: { code: 'abc', state: 'xyz' },
  });
});

test('a code without a state, or with an empty one, is not an arrival', () => {
  assert.equal(arrivalAt('?code=abc'), null);
  assert.equal(arrivalAt('?state=xyz'), null);
  assert.equal(arrivalAt('?code=&state=xyz'), null);
  assert.equal(arrivalAt('?code=abc&state='), null);
});

test('a loopback port and nonce in the query is a desktop handoff', () => {
  assert.deepEqual(arrivalAt(`?loopback_port=4321&loopback_nonce=${nonce}`), {
    kind: 'handoff',
    handoff: { port: 4321, nonce },
  });
});

test('the identity provider return wins over a handoff in the same query', () => {
  const arrival = arrivalAt(`?code=a&state=b&loopback_port=4321&loopback_nonce=${nonce}`);
  assert.equal(arrival?.kind, 'returned');
});

test('handoff ports outside the unprivileged range are refused', () => {
  for (const port of ['1023', '0', '65536', '-1', '99999']) {
    assert.equal(arrivalAt(`?loopback_port=${port}&loopback_nonce=${nonce}`), null, port);
  }
  assert.notEqual(arrivalAt(`?loopback_port=1024&loopback_nonce=${nonce}`), null);
  assert.notEqual(arrivalAt(`?loopback_port=65535&loopback_nonce=${nonce}`), null);
});

test('handoff ports that are not whole numbers are refused', () => {
  for (const port of ['12.5', 'abc', '']) {
    assert.equal(arrivalAt(`?loopback_port=${port}&loopback_nonce=${nonce}`), null, port);
  }
  assert.equal(arrivalAt(`?loopback_nonce=${nonce}`), null);
});

test('handoff nonces must be 32 lowercase hex characters', () => {
  for (const bad of ['short', 'A'.repeat(32), 'g'.repeat(32), `${nonce}0`, '']) {
    assert.equal(arrivalAt(`?loopback_port=4321&loopback_nonce=${bad}`), null, bad);
  }
  assert.equal(arrivalAt('?loopback_port=4321'), null);
});

test('an unrelated query is no arrival', () => {
  assert.equal(arrivalAt(''), null);
  assert.equal(arrivalAt('?q=1'), null);
});

test('the loopback address carries the token, its lifetime and the nonce on 127.0.0.1', () => {
  const address = loopbackAddress({ port: 4321, nonce }, 'tok&en', 3600);
  const url = new URL(address);
  assert.equal(url.origin, 'http://127.0.0.1:4321');
  assert.equal(url.searchParams.get('token'), 'tok&en');
  assert.equal(url.searchParams.get('expires_in'), '3600');
  assert.equal(url.searchParams.get('nonce'), nonce);
});

test('a handoff survives being serialised and parsed back', () => {
  const handoff = { port: 5000, nonce };
  assert.deepEqual(parseHandoff(serializeHandoff(handoff)), handoff);
});

test('parseHandoff rejects nothing stored, a returned arrival and garbage', () => {
  assert.equal(parseHandoff(null), null);
  assert.equal(parseHandoff('code=a&state=b'), null);
  assert.equal(parseHandoff('garbage'), null);
  assert.equal(parseHandoff('loopback_port=80&loopback_nonce=zz'), null);
});
