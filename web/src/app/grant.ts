import { normalise } from './folder';
import { t } from './i18n/language';
import type { NodeKind } from './node';

export type Access = 'read' | 'write';

export type Given = {
  id: string;
  node_id: string;
  name: string;
  kind: NodeKind;
  email: string;
  access: Access;
  in_trash: boolean;
  created_at: string;
};

export type Received = {
  id: string;
  name: string;
  kind: NodeKind;
  access: Access;
  owner_email: string;
  owner_name: string;
  created_at: string;
};

export type NewGrant = {
  path: string;
  email: string;
  access: Access;
};

export const SHARED_WITH_ME = 'Shared with me';

export type Where = { kind: 'own' } | { kind: 'shelf' } | { kind: 'shared'; mount: string };

export function whereIs(path: string): Where {
  const [first, second] = normalise(path).split('/');
  if (first !== SHARED_WITH_ME) {
    return { kind: 'own' };
  }
  return second === undefined ? { kind: 'shelf' } : { kind: 'shared', mount: second };
}

export function describeAccess(access: Access): string {
  return access === 'write' ? t('access.write') : t('access.read');
}

export function describeOrigin(received: Received): string {
  return t('files.origin', {
    owner: received.owner_name || received.owner_email,
    access: describeAccess(received.access),
  });
}
