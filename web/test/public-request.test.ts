import assert from 'node:assert/strict';
import { beforeEach, test } from 'node:test';
import { stubBrowser } from './browser.ts';
import { install } from './storage.ts';

stubBrowser('en');
const { LinkIsBusy, LinkIsGone, LinkNeedsPassword, entriesAt, getPublic } = await import(
  '../src/app/public/public-request.ts'
);

let requested: { url: string; init: RequestInit | undefined }[];
let answer: Response;

beforeEach(() => {
  requested = [];
  install('fetch', async (url: string, init?: RequestInit) => {
    requested.push({ url, init });
    return answer;
  });
});

test('the root of a link has no entries segment, a path gets an encoded one', () => {
  assert.equal(entriesAt(''), '');
  assert.equal(entriesAt('/'), '');
  assert.equal(entriesAt('a b/c'), '/entries/a%20b/c');
});

test('the token is encoded and the password is sent only when there is one', async () => {
  answer = new Response('{}', { status: 200 });
  await getPublic('https://api.test', 'to/ken', '/entries/x', '');
  await getPublic('https://api.test', 't', '', 'secret');
  assert.equal(requested[0]!.url, 'https://api.test/v1/public/to%2Fken/entries/x');
  assert.deepEqual(requested[0]!.init?.headers, {});
  assert.deepEqual(requested[1]!.init?.headers, { 'X-Share-Password': 'secret' });
});

test('a 401 asks for the password', async () => {
  answer = new Response(null, { status: 401 });
  await assert.rejects(getPublic('b', 't', '', ''), LinkNeedsPassword);
});

test('a 429 reports how long to wait, one minute when the server does not say', async () => {
  answer = new Response(null, { status: 429, headers: { 'Retry-After': '17' } });
  await assert.rejects(
    getPublic('b', 't', '', ''),
    (cause: unknown) => cause instanceof LinkIsBusy && cause.seconds === 17,
  );
  answer = new Response(null, { status: 429 });
  await assert.rejects(
    getPublic('b', 't', '', ''),
    (cause: unknown) => cause instanceof LinkIsBusy && cause.seconds === 60,
  );
});

test('any other failure means the link is gone', async () => {
  for (const status of [403, 404, 410, 500]) {
    answer = new Response(null, { status });
    await assert.rejects(getPublic('b', 't', '', ''), LinkIsGone, String(status));
  }
});
