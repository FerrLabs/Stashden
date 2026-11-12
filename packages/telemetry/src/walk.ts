import type { TransportItem } from '@grafana/faro-web-sdk';
import { MASK, METRIC_FIELD, isSecretName, redactText } from './redact';

export type StringRedactor = (value: string) => string;

export interface SanitizeOptions {
  redactString?: StringRedactor;
  redactPageUrl?: StringRedactor;
}

function defineEntry(target: Record<string, unknown>, key: string, value: unknown): void {
  Object.defineProperty(target, key, { value, enumerable: true, writable: true, configurable: true });
}

const SESSION_FIELD = /^session$/i;

type Entry = readonly [string, unknown];

interface Snapshot {
  readonly plain: WeakMap<object, boolean>;
  readonly elements: WeakMap<readonly unknown[], readonly unknown[]>;
  readonly entries: WeakMap<object, readonly Entry[]>;
}

interface Walk extends Snapshot {
  readonly secret: WeakSet<object>;
  readonly copies: WeakMap<object, unknown>;
}

function elementsOf(snapshot: Snapshot, array: readonly unknown[]): readonly unknown[] {
  const known = snapshot.elements.get(array);
  if (known !== undefined) return known;
  const taken = Array.from(array);
  snapshot.elements.set(array, taken);
  return taken;
}

function entriesOf(snapshot: Snapshot, container: object): readonly Entry[] {
  const known = snapshot.entries.get(container);
  if (known !== undefined) return known;
  const taken = Object.entries(container);
  snapshot.entries.set(container, taken);
  return taken;
}

function isPlainObject(value: object): boolean {
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function isTraversable(snapshot: Snapshot, value: object): boolean {
  const known = snapshot.plain.get(value);
  if (known !== undefined) return known;
  const verdict = Array.isArray(value) || isPlainObject(value);
  snapshot.plain.set(value, verdict);
  return verdict;
}

function isPlainRecord(snapshot: Snapshot, value: unknown): boolean {
  return typeof value === 'object' && value !== null && !Array.isArray(value) && isTraversable(snapshot, value);
}

function isSecretValue(snapshot: Snapshot, value: unknown, metricKey?: string): boolean {
  if (value === null || value === undefined || typeof value === 'boolean') return false;
  if (typeof value === 'number' || typeof value === 'bigint') return metricKey === undefined || !METRIC_FIELD.test(metricKey);
  if (typeof value !== 'object') return true;
  return !isTraversable(snapshot, value);
}

function isSecretField(snapshot: Snapshot, key: string, value: unknown): boolean {
  return isSecretName(key) && isSecretValue(snapshot, value, key);
}

function collectSecretContainers(
  meta: unknown,
  payload: unknown,
  snapshot: Snapshot,
): WeakSet<object> {
  const secret = new WeakSet<object>();
  const seenPlain = new WeakSet<object>();
  const seenMetaRoot = new WeakSet<object>();
  const visit = (value: unknown, inSecret: boolean, isMetaRoot = false): void => {
    if (value === null || typeof value !== 'object' || !isTraversable(snapshot, value)) return;
    const seen = inSecret ? secret : isMetaRoot ? seenMetaRoot : seenPlain;
    if (seen.has(value)) return;
    seen.add(value);
    if (Array.isArray(value)) {
      for (const entry of elementsOf(snapshot, value)) visit(entry, inSecret);
      return;
    }
    for (const [key, entry] of entriesOf(snapshot, value)) {
      const exempt = isMetaRoot && SESSION_FIELD.test(key) && isPlainRecord(snapshot, entry);
      visit(entry, inSecret || (isSecretName(key) && !exempt));
    }
  };
  visit(meta, false, true);
  visit(payload, false);
  return secret;
}

function newWalk(meta: unknown, payload: unknown): Walk {
  const snapshot: Snapshot = { plain: new WeakMap(), elements: new WeakMap(), entries: new WeakMap() };
  return {
    ...snapshot,
    secret: collectSecretContainers(meta, payload, snapshot),
    copies: new WeakMap(),
  };
}

function redactWalk(value: unknown, redactString: StringRedactor, walk: Walk): unknown {
  if (typeof value === 'string') return redactString(value);
  if (value === null || typeof value !== 'object') return value;
  if (!isTraversable(walk, value)) return value instanceof Date ? value : redactString(String(value));
  const known = walk.copies.get(value);
  if (known !== undefined) return known;
  const inSecret = walk.secret.has(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    walk.copies.set(value, copy);
    for (const entry of elementsOf(walk, value)) copy.push(inSecret && isSecretValue(walk, entry) ? MASK : redactWalk(entry, redactString, walk));
    return copy;
  }
  const copy: Record<string, unknown> = {};
  walk.copies.set(value, copy);
  const taken = new Set<string>();
  for (const [key, entry] of entriesOf(walk, value)) {
    const base = inSecret ? MASK : redactText(key);
    let target = base;
    for (let n = 2; taken.has(target); n++) target = `${base}#${n}`;
    taken.add(target);
    const masked = inSecret ? isSecretValue(walk, entry) : isSecretField(walk, key, entry);
    defineEntry(copy, target, masked ? MASK : redactWalk(entry, redactString, walk));
  }
  return copy;
}

export function redactDeep(value: unknown, redactString: StringRedactor = redactText): unknown {
  return redactWalk(value, redactString, newWalk(undefined, value));
}

export function sanitizeItem(item: TransportItem, options: SanitizeOptions = {}): TransportItem | null {
  const { redactString = redactText, redactPageUrl } = options;
  try {
    const { meta: itemMeta, payload: itemPayload } = item;
    const walk = newWalk(itemMeta, itemPayload);
    const meta = redactWalk(itemMeta, redactString, walk) as TransportItem['meta'];
    const pageUrl = itemMeta.page?.url;
    if (pageUrl !== undefined && redactPageUrl) meta.page = { ...meta.page, url: redactPageUrl(pageUrl) };
    return { ...item, meta, payload: redactWalk(itemPayload, redactString, walk) as TransportItem['payload'] };
  } catch {
    return null;
  }
}
