import type { TransportItem } from '@grafana/faro-web-sdk';
import { MASK, isSecretField, isSecretName, redactText } from './redact';

export type StringRedactor = (value: string) => string;

export interface SanitizeOptions {
  redactString?: StringRedactor;
  redactPageUrl?: StringRedactor;
}

function defineEntry(target: Record<string, unknown>, key: string, value: unknown): void {
  Object.defineProperty(target, key, { value, enumerable: true, writable: true, configurable: true });
}

interface Copies {
  readonly plain: WeakMap<object, unknown>;
  readonly secret: WeakMap<object, unknown>;
  readonly active: WeakSet<object>;
  readonly cyclic: WeakSet<object>;
}

function newCopies(): Copies {
  return { plain: new WeakMap(), secret: new WeakMap(), active: new WeakSet(), cyclic: new WeakSet() };
}

function opensSecretScope(field: string): boolean {
  return isSecretName(field) && field.toLowerCase() !== 'session';
}

function maskedCopy(copy: unknown[] | Record<string, unknown>, masked: unknown): unknown {
  if (Array.isArray(copy)) copy.splice(0, copy.length, ...(masked as unknown[]));
  else for (const key of Object.keys(copy)) defineEntry(copy, key, (masked as Record<string, unknown>)[key]);
  return copy;
}

export function redactDeep(value: unknown, redactString: StringRedactor = redactText, seen = newCopies(), field = ''): unknown {
  if (typeof value === 'string') return redactString(value);
  if (value === null || typeof value !== 'object') return value;
  const inSecret = opensSecretScope(field);
  const scope = inSecret ? seen.secret : seen.plain;
  if (scope.has(value)) return scope.get(value);
  if (inSecret && seen.active.has(value)) seen.cyclic.add(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    scope.set(value, copy);
    if (!inSecret) seen.active.add(value);
    for (const entry of value) copy.push(isSecretField(field, entry, true) ? MASK : redactDeep(entry, redactString, seen, field));
    seen.active.delete(value);
    return seen.cyclic.has(value) && !inSecret ? maskedCopy(copy, seen.secret.get(value)) : copy;
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return redactString(String(value));
  const copy: Record<string, unknown> = {};
  scope.set(value, copy);
  if (!inSecret) seen.active.add(value);
  const taken = new Set<string>();
  for (const [key, entry] of Object.entries(value)) {
    const entryKey = inSecret ? field : key;
    const base = redactText(key);
    let target = base;
    for (let n = 2; taken.has(target); n++) target = `${base}#${n}`;
    taken.add(target);
    defineEntry(copy, target, isSecretField(entryKey, entry, inSecret) ? MASK : redactDeep(entry, redactString, seen, entryKey));
  }
  seen.active.delete(value);
  return seen.cyclic.has(value) && !inSecret ? maskedCopy(copy, seen.secret.get(value)) : copy;
}

export function sanitizeItem(item: TransportItem, options: SanitizeOptions = {}): TransportItem | null {
  const { redactString = redactText, redactPageUrl } = options;
  try {
    const meta = redactDeep(item.meta, redactString) as TransportItem['meta'];
    const pageUrl = item.meta.page?.url;
    if (pageUrl !== undefined && redactPageUrl) meta.page = { ...meta.page, url: redactPageUrl(pageUrl) };
    return { ...item, meta, payload: redactDeep(item.payload, redactString) as TransportItem['payload'] };
  } catch {
    return null;
  }
}
