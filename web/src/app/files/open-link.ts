import type { Node } from '../node';

export const OPEN_PARAM = 'open';

export type Target =
  | { readonly kind: 'none' }
  | { readonly kind: 'file'; readonly node: Node }
  | { readonly kind: 'directory'; readonly node: Node }
  | { readonly kind: 'missing' };

export function targetOf(name: string | null, nodes: readonly Node[]): Target {
  if (name === null || name === '') {
    return { kind: 'none' };
  }
  const node = nodes.find((candidate) => candidate.name === name);
  if (node === undefined) {
    return { kind: 'missing' };
  }
  return node.kind === 'directory' ? { kind: 'directory', node } : { kind: 'file', node };
}

export type Listing = {
  hasValue(): boolean;
  value(): readonly Node[] | undefined;
};

export function loadedNodes(listing: Listing): readonly Node[] {
  return listing.hasValue() ? (listing.value() ?? []) : [];
}
