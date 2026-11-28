import { t } from '../i18n/language';
import { encodePath } from '../transfer';

export class LinkNeedsPassword extends Error {
  constructor() {
    super(t('public.error.needsPassword'));
  }
}

export class LinkIsGone extends Error {
  constructor() {
    super(t('public.gone.heading'));
  }
}

export class LinkIsBusy extends Error {
  readonly seconds: number;

  constructor(seconds: number) {
    super(t('public.error.busy'));
    this.seconds = seconds;
  }
}

const DEFAULT_RETRY_SECONDS = 60;

export function entriesAt(path: string): string {
  const below = encodePath(path);
  return below.length === 0 ? '' : `/entries${below}`;
}

export async function getPublic(
  base: string,
  token: string,
  at: string,
  password: string,
): Promise<Response> {
  const response = await fetch(`${base}/v1/public/${encodeURIComponent(token)}${at}`, {
    headers: password.length > 0 ? { 'X-Share-Password': password } : {},
  });

  if (response.status === 401) {
    throw new LinkNeedsPassword();
  }
  if (response.status === 429) {
    throw new LinkIsBusy(Number(response.headers.get('Retry-After') ?? DEFAULT_RETRY_SECONDS));
  }
  if (!response.ok) {
    throw new LinkIsGone();
  }
  return response;
}
