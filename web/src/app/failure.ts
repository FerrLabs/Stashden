import { t } from './i18n/language';
import { RequestFailed } from './request-failed';

export function failureText(cause: unknown): string {
  if (cause instanceof TypeError) {
    return t('error.unreachable');
  }
  return cause instanceof Error ? cause.message : String(cause);
}

export function signInFailure(cause: unknown): string {
  if (cause instanceof RequestFailed && cause.status === 401) {
    return t('signIn.invalid');
  }
  return failureText(cause);
}
