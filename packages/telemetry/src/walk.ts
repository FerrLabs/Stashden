import type { TransportItem } from '@grafana/faro-web-sdk';
import { MASK, isSecretField, isSecretName, isSecretValue, redactText } from './redact';

export type StringRedactor = (value: string) => string;

export interface SanitizeOptions {
  redactString?: StringRedactor;
  redactPageUrl?: StringRedactor;
}

function defineEntry(target: Record<string, unknown>, key: string, value: unknown): void {
  Object.defineProperty(target, key, { value, enumerable: true, writable: true, configurable: true });
}

const SESSION_FIELD = /^session$/i;

interface Walk {
  readonly secret: WeakSet<object>;
  readonly copies: WeakMap<object, unknown>;
}

function isPlainObject(value: object): boolean {
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function isTraversable(value: object): boolean {
  return Array.isArray(value) || isPlainObject(value);
}

function collectSecretContainers(meta: unknown, payload: unknown): WeakSet<object> {
  const secret = new WeakSet<object>();
  const seenPlain = new WeakSet<object>();
  const visit = (value: unknown, inSecret: boolean, isMeta = false): void => {
    if (value === null || typeof value !== 'object' || !isTraversable(value)) return;
    const seen = inSecret ? secret : seenPlain;
    if (seen.has(value)) return;
    seen.add(value);
    if (Array.isArray(value)) {
      for (const entry of value) visit(entry, inSecret);
      return;
    }
    for (const [key, entry] of Object.entries(value)) {
      const exempt = isMeta && SESSION_FIELD.test(key) && !Array.isArray(entry);
      visit(entry, inSecret || (isSecretName(key) && !exempt));
    }
  };
  visit(meta, false, true);
  visit(payload, false);
  return secret;
}

function newWalk(meta: unknown, payload: unknown): Walk {
  return { secret: collectSecretContainers(meta, payload), copies: new WeakMap() };
}

function redactWalk(value: unknown, redactString: StringRedactor, walk: Walk): unknown {
  if (typeof value === 'string') return redactString(value);
  if (value === null || typeof value !== 'object') return value;
  if (!isTraversable(value)) return redactString(String(value));
  const known = walk.copies.get(value);
  if (known !== undefined) return known;
  const inSecret = walk.secret.has(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    walk.copies.set(value, copy);
    for (const entry of value) copy.push(inSecret && isSecretValue(entry) ? MASK : redactWalk(entry, redactString, walk));
    return copy;
  }
  const copy: Record<string, unknown> = {};
  walk.copies.set(value, copy);
  const taken = new Set<string>();
  for (const [key, entry] of Object.entries(value)) {
    const base = inSecret ? MASK : redactText(key);
    let target = base;
    for (let n = 2; taken.has(target); n++) target = `${base}#${n}`;
    taken.add(target);
    const masked = inSecret ? isSecretValue(entry) : isSecretField(key, entry);
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
    const walk = newWalk(item.meta, item.payload);
    const meta = redactWalk(item.meta, redactString, walk) as TransportItem['meta'];
    const pageUrl = item.meta.page?.url;
    if (pageUrl !== undefined && redactPageUrl) meta.page = { ...meta.page, url: redactPageUrl(pageUrl) };
    return { ...item, meta, payload: redactWalk(item.payload, redactString, walk) as TransportItem['payload'] };
  } catch {
    return null;
  }
}
