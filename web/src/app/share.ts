import { formatDay, t } from './i18n/language';
import type { NodeKind } from './node';

export type Share = {
  id: string;
  node_id: string;
  name: string;
  has_password: boolean;
  expires_at: string | null;
  created_at: string;
  last_used_at: string | null;
};

export type Minted = Share & { token: string };

export type NewShare = {
  path: string;
  expires_at?: string;
  password?: string;
};

export type PublicEntry = {
  name: string;
  kind: NodeKind;
  size: number;
  updated_at: string;
};

export type Linked = {
  entry: PublicEntry;
  children: PublicEntry[];
};

export const SHARE_PREFIX = '/s';

export function linkFor(token: string, origin: string): string {
  return `${origin}/#${SHARE_PREFIX}/${token}`;
}

export function describeExpiry(share: Share, now = new Date()): string {
  if (share.expires_at === null) {
    return t('share.describe.none');
  }
  const at = new Date(share.expires_at);
  if (Number.isNaN(at.getTime())) {
    return t('share.describe.unreadable');
  }
  const date = formatDay(at);
  return at.getTime() <= now.getTime()
    ? t('share.describe.expired', { date })
    : t('share.describe.expires', { date });
}

export function describeUse(share: Share): string {
  const at = share.last_used_at === null ? null : new Date(share.last_used_at);
  return at === null || Number.isNaN(at.getTime())
    ? t('share.describe.never')
    : t('share.describe.lastOpened', { date: formatDay(at, false) });
}

export function endOfDay(day: string): string | null {
  const at = new Date(`${day}T23:59:59`);
  return Number.isNaN(at.getTime()) ? null : at.toISOString();
}

export function today(): string {
  const now = new Date();
  return new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
}

export function describeLink(share: Share): string {
  const parts = [describeExpiry(share), describeUse(share)];
  if (share.has_password) {
    parts.push(t('share.describe.password'));
  }
  return parts.join(', ');
}
