import assert from 'node:assert/strict';
import { beforeEach, test } from 'node:test';
import { stubBrowser } from './browser.ts';
import { MemoryStorage, install } from './storage.ts';

stubBrowser('en');
const { resolvePlatform } = await import('../src/app/platform.ts');
const { RequestFailed } = await import('../src/app/request-failed.ts');

type Call = { url: string; init: RequestInit | undefined };

const BASE = 'https://api.example.test';

let calls: Call[];
let replies: Response[];
let local: MemoryStorage;
let session: MemoryStorage;
let navigated: { assigned: string[]; replaced: string[] };

function reply(status: number, body?: unknown): Response {
  return new Response(body === undefined ? null : JSON.stringify(body), { status });
}

beforeEach(() => {
  calls = [];
  replies = [];
  local = new MemoryStorage();
  session = new MemoryStorage();
  navigated = { assigned: [], replaced: [] };
  install('localStorage', local);
  install('sessionStorage', session);
  install('window', {
    location: {
      assign: (url: string) => navigated.assigned.push(url),
      replace: (url: string) => navigated.replaced.push(url),
    },
  });
  install('fetch', async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    const next = replies.shift();
    assert.ok(next, `unexpected request to ${url}`);
    return next;
  });
});

function platform() {
  return resolvePlatform(BASE);
}

function headersOf(call: Call): Record<string, string> {
  return call.init?.headers as Record<string, string>;
}

function bodyOf(call: Call): unknown {
  return JSON.parse(call.init?.body as string);
}

function summary(): string[] {
  return calls.map((call) => `${call.init?.method ?? 'GET'} ${call.url.slice(BASE.length)}`);
}

test('requests carry the stored token as a bearer header and none without one', async () => {
  replies.push(reply(200, []), reply(200, []));
  await platform().listShares!();
  assert.equal(headersOf(calls[0]!)['Authorization'], undefined);
  local.setItem('stashden.token', 'abc');
  await platform().listShares!();
  assert.equal(headersOf(calls[1]!)['Authorization'], 'Bearer abc');
});

test('authenticated follows the presence of a stored token', () => {
  assert.equal(platform().authenticated(), false);
  local.setItem('stashden.token', 'abc');
  assert.equal(platform().authenticated(), true);
});

test('login stores the token it is given and posts the credentials as JSON', async () => {
  replies.push(reply(200, { token: 'fresh' }));
  assert.equal(await platform().login('a@b.test', 'pw'), null);
  assert.equal(local.getItem('stashden.token'), 'fresh');
  assert.equal(calls[0]!.url, `${BASE}/v1/auth/login`);
  assert.equal(calls[0]!.init?.method, 'POST');
  assert.deepEqual(bodyOf(calls[0]!), { email: 'a@b.test', password: 'pw' });
});

test('a failed login keeps no token and surfaces the status and server message', async () => {
  replies.push(reply(401, { error: 'bad credentials' }));
  await assert.rejects(
    platform().login('a@b.test', 'pw'),
    (cause: unknown) =>
      cause instanceof RequestFailed && cause.status === 401 && cause.message === 'bad credentials',
  );
  assert.equal(local.getItem('stashden.token'), null);
});

test('403 uses the server explanation, 507 is translated, others fall back to the status line', async () => {
  replies.push(reply(403, { error: 'read only share' }), reply(507), reply(500));
  const remove = platform().remove;
  await assert.rejects(remove('/a'), /read only share/);
  await assert.rejects(
    remove('/a'),
    (cause: unknown) => cause instanceof RequestFailed && cause.status === 507,
  );
  await assert.rejects(remove('/a'), /^Error: 500/);
});

test('a 403 without an explanation still gets a message', async () => {
  replies.push(reply(403));
  await assert.rejects(
    platform().remove('/a'),
    (cause: unknown) =>
      cause instanceof RequestFailed && cause.status === 403 && cause.message.length > 0,
  );
});

test('an error body that is not a non-empty string explanation is ignored', async () => {
  replies.push(reply(500, { error: '' }), reply(500, { error: 7 }), reply(500, 'plain'));
  const remove = platform().remove;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await assert.rejects(remove('/a'), /^Error: 500/);
  }
});

test('loginWithProvider sends the browser to the authorize url', async () => {
  replies.push(reply(200, { authorize_url: 'https://idp.test/auth' }));
  assert.deepEqual(await platform().loginWithProvider(), { kind: 'redirected' });
  assert.deepEqual(navigated.assigned, ['https://idp.test/auth']);
  assert.equal(calls[0]!.url, `${BASE}/v1/auth/oidc/start`);
});

test('completeProviderLogin without a handoff stores the token and reports signed in', async () => {
  replies.push(reply(200, { token: 'tok', expires_in: 60 }));
  assert.equal(await platform().completeProviderLogin!({ code: 'c', state: 's' }), true);
  assert.equal(local.getItem('stashden.token'), 'tok');
  assert.deepEqual(bodyOf(calls[0]!), { code: 'c', state: 's' });
  assert.deepEqual(navigated.replaced, []);
});

test('completeProviderLogin with a remembered handoff hands the token to the loopback app instead', async () => {
  const nonce = 'a'.repeat(32);
  const p = platform();
  p.rememberHandoff!({ port: 4321, nonce });
  replies.push(reply(200, { token: 'tok', expires_in: 60 }));
  assert.equal(await p.completeProviderLogin!({ code: 'c', state: 's' }), false);
  assert.equal(local.getItem('stashden.token'), null);
  assert.equal(session.getItem('stashden.handoff'), null);
  assert.deepEqual(navigated.replaced, [
    `http://127.0.0.1:4321/?token=tok&expires_in=60&nonce=${nonce}`,
  ]);
});

test('rememberHandoff swallows a storage that refuses the write', () => {
  install('sessionStorage', {
    setItem: () => {
      throw new Error('quota');
    },
  });
  assert.doesNotThrow(() => platform().rememberHandoff!({ port: 4000, nonce: 'b'.repeat(32) }));
});

test('completeProviderLogin falls back to a plain sign-in when session storage throws', async () => {
  install('sessionStorage', {
    getItem: () => {
      throw new Error('blocked');
    },
  });
  replies.push(reply(200, { token: 'tok', expires_in: 60 }));
  assert.equal(await platform().completeProviderLogin!({ code: 'c', state: 's' }), true);
  assert.equal(local.getItem('stashden.token'), 'tok');
});

test('signOut clears the token even when the server call fails with something else', async () => {
  local.setItem('stashden.token', 'abc');
  replies.push(reply(500));
  await assert.rejects(async () => platform().signOut());
  assert.equal(local.getItem('stashden.token'), null);
});

test('signOut treats an expired session (401) as already signed out', async () => {
  local.setItem('stashden.token', 'abc');
  replies.push(reply(401));
  await platform().signOut();
  assert.equal(local.getItem('stashden.token'), null);
  assert.equal(calls[0]!.url, `${BASE}/v1/auth/logout`);
});

test('changePassword sends current and new password to the password endpoint', async () => {
  replies.push(reply(204));
  await platform().changePassword!('old', 'new');
  assert.equal(calls[0]!.init?.method, 'PUT');
  assert.equal(calls[0]!.url, `${BASE}/v1/auth/password`);
  assert.deepEqual(bodyOf(calls[0]!), { current: 'old', password: 'new' });
});

test('changePassword: a 401 reads as a wrong password unless the server names the identity provider', async () => {
  const p = platform();
  replies.push(reply(401, { error: 'wrong' }));
  await assert.rejects(p.changePassword!('bad', 'new'), (cause: unknown) => {
    return cause instanceof Error && !(cause instanceof RequestFailed) && cause.message !== 'wrong';
  });
  replies.push(reply(401, { error: 'managed by the identity provider' }));
  await assert.rejects(p.changePassword!('x', 'new'), /identity provider/);
});

test('changePassword surfaces other server errors', async () => {
  replies.push(reply(500, { error: 'boom' }));
  await assert.rejects(platform().changePassword!('x', 'new'), /boom/);
});

test('deleteOwnAccount sends the confirmation and hand over target, then forgets the token', async () => {
  local.setItem('stashden.token', 'abc');
  replies.push(reply(204));
  await platform().deleteOwnAccount!({ email: 'a@b.test' }, 'heir');
  assert.deepEqual(bodyOf(calls[0]!), { email: 'a@b.test', hand_over_to: 'heir' });
  assert.equal(local.getItem('stashden.token'), null);
});

test('deleteOwnAccount refusals keep the token and pick the wording by confirmation kind', async () => {
  local.setItem('stashden.token', 'abc');
  const p = platform();
  replies.push(reply(401), reply(401), reply(500, { error: 'nope' }));
  const byPassword = await p.deleteOwnAccount!({ password: 'x' }, null).catch((e: Error) => e);
  const byEmail = await p.deleteOwnAccount!({ email: 'x' }, null).catch((e: Error) => e);
  assert.ok(byPassword instanceof Error && byEmail instanceof Error);
  assert.notEqual(byPassword.message, byEmail.message);
  await assert.rejects(p.deleteOwnAccount!({ password: 'x' }, null), /nope/);
  assert.equal(local.getItem('stashden.token'), 'abc');
});

test('resource ids are percent-encoded in the request url', async () => {
  const p = platform();
  for (let i = 0; i < 6; i += 1) {
    replies.push(reply(204));
  }
  await p.revokeAppPassword!('a/b c');
  await p.setRole!('x y', 'admin');
  await p.setQuota!('x y', 5);
  await p.unlockAccount!('x/y');
  await p.withdrawGrant!('g?1');
  await p.purgeFromTrash!('t#1');
  assert.deepEqual(summary(), [
    'DELETE /v1/app-passwords/a%2Fb%20c',
    'PUT /v1/users/x%20y/role',
    'PUT /v1/users/x%20y/quota',
    'POST /v1/users/x%2Fy/unlock',
    'DELETE /v1/grants/g%3F1',
    'DELETE /v1/trash/t%231',
  ]);
  assert.deepEqual(bodyOf(calls[1]!), { role: 'admin' });
  assert.deepEqual(bodyOf(calls[2]!), { bytes_max: 5 });
});

test('setDisabled picks the enable or disable endpoint', async () => {
  const p = platform();
  replies.push(reply(204), reply(204));
  await p.setDisabled!('u', true);
  await p.setDisabled!('u', false);
  assert.equal(calls[0]!.url, `${BASE}/v1/users/u/disable`);
  assert.equal(calls[1]!.url, `${BASE}/v1/users/u/enable`);
});

test('deleteAccount only adds the hand over query when someone inherits', async () => {
  const p = platform();
  replies.push(reply(204), reply(204));
  await p.deleteAccount!('u', null);
  await p.deleteAccount!('u', 'a b');
  assert.equal(calls[0]!.url, `${BASE}/v1/users/u`);
  assert.equal(calls[1]!.url, `${BASE}/v1/users/u?hand_over_to=a%20b`);
});

test('file operations address encoded paths and the right verbs', async () => {
  const p = platform();
  replies.push(
    reply(200, []),
    reply(200, { id: 'n' }),
    reply(200, []),
    reply(204),
    reply(200, { id: 'n' }),
  );
  await p.listFolder('/dir one/sub');
  await p.createFolder!('/new folder');
  await p.search!('a&b');
  await p.remove('/dir one/file.txt');
  await p.rename('/a', '/b');
  assert.deepEqual(summary(), [
    'GET /v1/folders/dir%20one/sub',
    'POST /v1/folders/new%20folder',
    'GET /v1/search?q=a%26b',
    'DELETE /v1/files/dir%20one/file.txt',
    'POST /v1/move',
  ]);
  assert.deepEqual(bodyOf(calls[4]!), { from: '/a', to: '/b' });
});

test('read returns the file body', async () => {
  replies.push(new Response('hello'));
  const blob = await platform().read('/a.txt');
  assert.equal(await blob.text(), 'hello');
});

test('shares, grants, versions, trash and profile use their endpoints', async () => {
  const p = platform();
  for (let i = 0; i < 9; i += 1) {
    replies.push(reply(200, {}));
  }
  await p.share!({ path: '/a' } as never);
  await p.grant!({ path: '/a', email: 'x', access: 'read' });
  await p.listGrants!();
  await p.receivedGrants!();
  await p.listVersions!('/a b');
  await p.restoreVersion!('/a', 'v 1');
  await p.listTrash!();
  await p.restoreFromTrash!('t1');
  await p.updateProfile!('Name');
  assert.deepEqual(summary(), [
    'POST /v1/shares',
    'POST /v1/grants',
    'GET /v1/grants',
    'GET /v1/grants/received',
    'GET /v1/versions/a%20b',
    'POST /v1/version/v%201/a',
    'GET /v1/trash',
    'POST /v1/trash/t1/restore',
    'PUT /v1/auth/me',
  ]);
  assert.deepEqual(bodyOf(calls[8]!), { display_name: 'Name' });
});

test('empty trash, revoke share, create account and reset password', async () => {
  const p = platform();
  replies.push(reply(204), reply(204), reply(204), reply(204));
  await p.emptyTrash!();
  await p.revokeShare!('s 1');
  await p.createAccount!({ email: 'e', display_name: 'd', password: 'p', role: 'member' });
  await p.resetPassword!('u 1', 'pw');
  assert.deepEqual(summary(), [
    'DELETE /v1/trash',
    'DELETE /v1/shares/s%201',
    'POST /v1/users',
    'PUT /v1/users/u%201/password',
  ]);
});

test('app passwords and accounts are listed and minted through their endpoints', async () => {
  const p = platform();
  replies.push(reply(200, []), reply(200, {}), reply(200, []), reply(200, {}));
  await p.listAppPasswords!();
  await p.mintAppPassword!('laptop');
  await p.listAccounts!();
  await p.account();
  assert.deepEqual(summary(), [
    'GET /v1/app-passwords',
    'POST /v1/app-passwords',
    'GET /v1/users',
    'GET /v1/auth/me',
  ]);
  assert.deepEqual(bodyOf(calls[1]!), { name: 'laptop' });
});

test('legacy roxycloud keys move to the stashden prefix without overwriting newer values', () => {
  local.setItem('roxycloud.language', 'fr');
  local.setItem('roxycloud.token', 'old');
  local.setItem('stashden.token', 'new');
  platform();
  assert.equal(local.getItem('stashden.language'), 'fr');
  assert.equal(local.getItem('stashden.token'), 'new');
  assert.equal(local.getItem('roxycloud.language'), null);
  assert.equal(local.getItem('roxycloud.token'), null);
});

test('legacy key adoption survives an unusable storage', () => {
  install('localStorage', {
    get length(): number {
      throw new Error('blocked');
    },
  });
  assert.doesNotThrow(() => platform());
});
