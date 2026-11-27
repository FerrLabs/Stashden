import { locale } from './i18n/language';

export type NodeKind = 'directory' | 'file';

export type Node = {
  id: string;
  owner_id: string;
  parent_id: string | null;
  name: string;
  kind: NodeKind;
  size: number;
  etag: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string;
};

export type Trashed = Node & { expires_at?: string };

export type Hit = Node & { path: string };

export function byKindThenName(left: Node, right: Node): number {
  if (left.kind !== right.kind) {
    return left.kind === 'directory' ? -1 : 1;
  }
  return left.name.localeCompare(right.name, locale());
}
