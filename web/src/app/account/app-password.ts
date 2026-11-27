import { formatDay, t } from '../i18n/language';

export type AppPassword = {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
};

export type MintedPassword = AppPassword & { secret: string };

export function davUrl(): string {
  const base = STASHDEN_API_URL.length > 0 ? STASHDEN_API_URL : location.origin;
  return `${base.replace(/\/$/, '')}/dav`;
}

export function describeUse(password: AppPassword, now = new Date()): string {
  const added = day(new Date(password.created_at), now);
  const used = password.last_used_at === null ? null : new Date(password.last_used_at);
  return used === null || Number.isNaN(used.getTime())
    ? t('appPasswords.neverUsed', { added })
    : t('appPasswords.lastUsed', { added, used: day(used, now) });
}

function day(at: Date, now: Date): string {
  return formatDay(at, at.getFullYear() !== now.getFullYear());
}
