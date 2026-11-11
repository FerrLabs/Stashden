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
}

function newCopies(): Copies {
  return { plain: new WeakMap(), secret: new WeakMap() };
}

export function redactDeep(value: unknown, redactString: StringRedactor = redactText, seen = newCopies(), field = ''): unknown {
  if (typeof value === 'string') return redactString(value);
  if (value === null || typeof value !== 'object') return value;
  const scope = isSecretName(field) ? seen.secret : seen.plain;
  if (scope.has(value)) return scope.get(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    scope.set(value, copy);
    for (const entry of value) copy.push(isSecretField(field, entry, true) ? MASK : redactDeep(entry, redactString, seen, field));
    return copy;
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return redactString(String(value));
  const copy: Record<string, unknown> = {};
  scope.set(value, copy);
  const taken = new Set<string>();
  for (const [key, entry] of Object.entries(value)) {
    const base = redactText(key);
    let target = base;
    for (let n = 2; taken.has(target); n++) target = `${base}#${n}`;
    taken.add(target);
    defineEntry(copy, target, isSecretField(key, entry) ? MASK : redactDeep(entry, redactString, seen, key));
  }
  return copy;
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
