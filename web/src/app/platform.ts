import { InjectionToken, type Provider } from '@angular/core';
import type { Account, Role } from './account';
import type { AppPassword, MintedPassword } from './account/app-password';
import type { ManagedAccount, NewAccount } from './accounts/managed';
import type { Given, NewGrant, Received } from './grant';
import type { Hit, Node, Trashed } from './node';
import {
  encodePath,
  saveStreaming,
  upload as uploadChunked,
  type Link,
  type Progress,
} from './transfer';
import type { Minted, NewShare, Share } from './share';
import type { Version } from './version';
import {
  loopbackAddress,
  parseHandoff,
  serializeHandoff,
  type Handoff,
  type Methods,
  type Returned,
} from './provider';

export type Confirmation = { password: string } | { email: string };

export type ProviderSignIn =
  | { kind: 'redirected' }
  | { kind: 'signedIn'; warning: string | null };

export interface Platform {
  authenticated(): boolean;
  login(email: string, password: string): Promise<string | null>;
  methods(): Promise<Methods>;
  loginWithProvider(): Promise<ProviderSignIn>;
  rememberHandoff?(handoff: Handoff): void;
  completeProviderLogin?(returned: Returned): Promise<boolean>;
  signOut(): void | Promise<void>;
  changePassword?(current: string | null, password: string): Promise<void>;
  deleteOwnAccount?(confirmation: Confirmation, handOverTo: string | null): Promise<void>;
  listAppPasswords?(): Promise<AppPassword[]>;
  mintAppPassword?(name: string): Promise<MintedPassword>;
  revokeAppPassword?(id: string): Promise<void>;
  listAccounts?(): Promise<ManagedAccount[]>;
  createAccount?(account: NewAccount): Promise<void>;
  setRole?(id: string, role: Role): Promise<void>;
  setQuota?(id: string, bytes: number): Promise<void>;
  setDisabled?(id: string, disabled: boolean): Promise<void>;
  unlockAccount?(id: string): Promise<void>;
  resetPassword?(id: string, password: string): Promise<void>;
  deleteAccount?(id: string, handOverTo: string | null): Promise<void>;
  account(): Promise<Account>;
  updateProfile?(displayName: string): Promise<Account>;
  listFolder(path: string): Promise<Node[]>;
  read(path: string): Promise<Blob>;
  download(path: string, name: string): Promise<string | null>;
  downloadArchive?(path: string, name: string): Promise<string | null>;
  remove(path: string): Promise<void>;
  rename(from: string, to: string): Promise<Node>;
  createFolder?(path: string): Promise<Node>;
  search?(query: string): Promise<Hit[]>;
  upload?(path: string, file: File, progress?: Progress): Promise<void>;
  listShares?(): Promise<Share[]>;
  share?(request: NewShare): Promise<Minted>;
  revokeShare?(id: string): Promise<void>;
  listGrants?(): Promise<Given[]>;
  grant?(request: NewGrant): Promise<Given>;
  receivedGrants?(): Promise<Received[]>;
  withdrawGrant?(id: string): Promise<void>;
  listVersions?(path: string): Promise<Version[]>;
  downloadVersion?(path: string, id: string, name: string): Promise<string | null>;
  restoreVersion?(path: string, id: string): Promise<Node>;
  listTrash?(): Promise<Trashed[]>;
  restoreFromTrash?(id: string): Promise<Node>;
  purgeFromTrash?(id: string): Promise<void>;
  emptyTrash?(): Promise<void>;
}

export const PLATFORM = new InjectionToken<Platform>('Stashden platform');

export class RequestFailed extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

const TOKEN_KEY = 'stashden.token';
const HANDOFF_KEY = 'stashden.handoff';
const KEY_PREFIX = 'stashden.';
const LEGACY_KEY_PREFIX = 'roxycloud.';

function adoptLegacyKeys(): void {
  try {
    const legacyKeys = Array.from({ length: localStorage.length }, (_, index) =>
      localStorage.key(index),
    ).filter((key): key is string => key?.startsWith(LEGACY_KEY_PREFIX) ?? false);
    for (const legacy of legacyKeys) {
      const current = KEY_PREFIX + legacy.slice(LEGACY_KEY_PREFIX.length);
      const value = localStorage.getItem(legacy);
      if (value !== null && localStorage.getItem(current) === null) {
        localStorage.setItem(current, value);
      }
      localStorage.removeItem(legacy);
    }
  } catch {
    return;
  }
}

function browserPlatform(baseUrl: string): Platform {
  const bearer = (): Record<string, string> => {
    const token = localStorage.getItem(TOKEN_KEY);
    return token ? { Authorization: `Bearer ${token}` } : {};
  };

  const call = async (path: string, init?: RequestInit): Promise<Response> => {
    const response = await fetch(`${baseUrl}${path}`, {
      ...init,
      headers: {
        ...init?.headers,
        ...bearer(),
      },
    });
    if (!response.ok) {
      throw new RequestFailed(response.status, await messageFor(response));
    }
    return response;
  };

  const json = async <T>(path: string, init?: RequestInit): Promise<T> =>
    (await (await call(path, init)).json()) as T;

  const link: Link = {
    base: baseUrl,
    headers: bearer,
    fail: async (status, text) =>
      new RequestFailed(status, await messageFor(new Response(text, { status }))),
  };

  return {
    authenticated: () => localStorage.getItem(TOKEN_KEY) !== null,
    login: async (email, password) => {
      const session = await json<{ token: string }>('/v1/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password }),
      });
      localStorage.setItem(TOKEN_KEY, session.token);
      return null;
    },
    methods: () => json<Methods>('/v1/auth/methods'),
    loginWithProvider: async () => {
      const started = await json<{ authorize_url: string }>('/v1/auth/oidc/start', {
        method: 'POST',
      });
      window.location.assign(started.authorize_url);
      return { kind: 'redirected' };
    },
    rememberHandoff: (handoff) => {
      try {
        sessionStorage.setItem(HANDOFF_KEY, serializeHandoff(handoff));
      } catch {
        return;
      }
    },
    completeProviderLogin: async ({ code, state }) => {
      const session = await json<{ token: string; expires_in: number }>(
        '/v1/auth/oidc/callback',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ code, state }),
        },
      );
      let handoff: Handoff | null = null;
      try {
        handoff = parseHandoff(sessionStorage.getItem(HANDOFF_KEY));
        sessionStorage.removeItem(HANDOFF_KEY);
      } catch {
        handoff = null;
      }
      if (handoff !== null) {
        window.location.replace(loopbackAddress(handoff, session.token, session.expires_in));
        return false;
      }
      localStorage.setItem(TOKEN_KEY, session.token);
      return true;
    },
    signOut: async () => {
      try {
        await call('/v1/auth/logout', { method: 'POST' });
      } catch (cause) {
        if (!(cause instanceof RequestFailed && cause.status === 401)) {
          throw cause;
        }
      } finally {
        localStorage.removeItem(TOKEN_KEY);
      }
    },
    changePassword: async (current, password) => {
      const response = await fetch(`${baseUrl}/v1/auth/password`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          ...bearer(),
        },
        body: JSON.stringify({ current, password }),
      });
      if (response.status === 401) {
        throw new Error(await refusalFor(response, 'that is not your current password'));
      }
      if (!response.ok) {
        throw new Error(await messageFor(response));
      }
    },
    deleteOwnAccount: async (confirmation, handOverTo) => {
      const response = await fetch(`${baseUrl}/v1/auth/me/delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...bearer() },
        body: JSON.stringify({ ...confirmation, hand_over_to: handOverTo }),
      });
      if (response.status === 401) {
        throw new Error(
          await refusalFor(
            response,
            'password' in confirmation ? 'that is not your password' : 'that is not your address',
          ),
        );
      }
      if (!response.ok) {
        throw new Error(await messageFor(response));
      }
      localStorage.removeItem(TOKEN_KEY);
    },
    listAppPasswords: () => json<AppPassword[]>('/v1/app-passwords'),
    mintAppPassword: (name) =>
      json<MintedPassword>('/v1/app-passwords', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
      }),
    revokeAppPassword: async (id) => {
      await call(`/v1/app-passwords/${encodeURIComponent(id)}`, { method: 'DELETE' });
    },
    listAccounts: () => json<ManagedAccount[]>('/v1/users'),
    createAccount: async (account) => {
      await call('/v1/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(account),
      });
    },
    setRole: async (id, role) => {
      await call(`/v1/users/${encodeURIComponent(id)}/role`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role }),
      });
    },
    setQuota: async (id, bytes) => {
      await call(`/v1/users/${encodeURIComponent(id)}/quota`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ bytes_max: bytes }),
      });
    },
    setDisabled: async (id, disabled) => {
      await call(`/v1/users/${encodeURIComponent(id)}/${disabled ? 'disable' : 'enable'}`, {
        method: 'POST',
      });
    },
    unlockAccount: async (id) => {
      await call(`/v1/users/${encodeURIComponent(id)}/unlock`, { method: 'POST' });
    },
    deleteAccount: async (id, handOverTo) => {
      const query = handOverTo === null ? '' : `?hand_over_to=${encodeURIComponent(handOverTo)}`;
      await call(`/v1/users/${encodeURIComponent(id)}${query}`, { method: 'DELETE' });
    },
    resetPassword: async (id, password) => {
      await call(`/v1/users/${encodeURIComponent(id)}/password`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
      });
    },
    account: () => json<Account>('/v1/auth/me'),
    updateProfile: (displayName) =>
      json<Account>('/v1/auth/me', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ display_name: displayName }),
      }),
    listFolder: (path) => json<Node[]>(`/v1/folders${encodePath(path)}`),
    read: async (path) => (await call(`/v1/files${encodePath(path)}`)).blob(),
    upload: (path, file, progress) => uploadChunked(link, path, file, progress),
    download: async (path, name) => {
      await saveStreaming(() => call(`/v1/files${encodePath(path)}`), name);
      return null;
    },
    downloadArchive: async (path, name) => {
      await saveStreaming(() => call(`/v1/archive${encodePath(path)}`), `${name}.zip`);
      return null;
    },
    remove: async (path) => {
      await call(`/v1/files${encodePath(path)}`, { method: 'DELETE' });
    },
    createFolder: (path) => json<Node>(`/v1/folders${encodePath(path)}`, { method: 'POST' }),
    search: (query) => json<Hit[]>(`/v1/search?q=${encodeURIComponent(query)}`),
    rename: (from, to) =>
      json<Node>('/v1/move', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from, to }),
      }),
    listShares: () => json<Share[]>('/v1/shares'),
    share: (request) =>
      json<Minted>('/v1/shares', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }),
    revokeShare: async (id) => {
      await call(`/v1/shares/${encodeURIComponent(id)}`, { method: 'DELETE' });
    },
    listGrants: () => json<Given[]>('/v1/grants'),
    grant: (request) =>
      json<Given>('/v1/grants', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }),
    receivedGrants: () => json<Received[]>('/v1/grants/received'),
    withdrawGrant: async (id) => {
      await call(`/v1/grants/${encodeURIComponent(id)}`, { method: 'DELETE' });
    },
    listVersions: (path) => json<Version[]>(`/v1/versions${encodePath(path)}`),
    downloadVersion: async (path, id, name) => {
      const address = `/v1/version/${encodeURIComponent(id)}${encodePath(path)}`;
      await saveStreaming(() => call(address), name);
      return null;
    },
    restoreVersion: (path, id) =>
      json<Node>(`/v1/version/${encodeURIComponent(id)}${encodePath(path)}`, { method: 'POST' }),
    listTrash: () => json<Trashed[]>('/v1/trash'),
    restoreFromTrash: (id) =>
      json<Node>(`/v1/trash/${encodeURIComponent(id)}/restore`, { method: 'POST' }),
    purgeFromTrash: async (id) => {
      await call(`/v1/trash/${encodeURIComponent(id)}`, { method: 'DELETE' });
    },
    emptyTrash: async () => {
      await call('/v1/trash', { method: 'DELETE' });
    },
  };
}

async function refusalFor(response: Response, fallback: string): Promise<string> {
  const explanation = await explanationFrom(response);
  return explanation?.includes('identity provider') ? explanation : fallback;
}

async function messageFor(response: Response): Promise<string> {
  switch (response.status) {
    case 403:
      return (await explanationFrom(response)) ?? 'this account may only read';
    case 507:
      return 'there is no room left in this account';
    default:
      return (await explanationFrom(response)) ?? `${response.status} ${response.statusText}`;
  }
}

async function explanationFrom(response: Response): Promise<string | null> {
  const body: unknown = await response.json().catch(() => null);
  const explanation =
    typeof body === 'object' && body !== null ? (body as { error?: unknown }).error : null;
  return typeof explanation === 'string' && explanation.length > 0 ? explanation : null;
}

export { encodePath };

export function resolvePlatform(baseUrl: string): Platform {
  adoptLegacyKeys();
  return browserPlatform(baseUrl);
}

export function providePlatform(): Provider {
  return { provide: PLATFORM, useFactory: () => resolvePlatform(STASHDEN_API_URL) };
}
