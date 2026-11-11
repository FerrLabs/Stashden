import type { TransportItem } from '@grafana/faro-web-sdk';
import { MASK, isSensitiveField, redactText } from './redact';

export type StringRedactor = (value: string) => string;

export interface SanitizeOptions {
  redactString?: StringRedactor;
  redactPageUrl?: StringRedactor;
}

export function redactDeep(value: unknown, redactString: StringRedactor = redactText, seen = new WeakMap<object, unknown>()): unknown {
  if (typeof value === 'string') return redactString(value);
  if (value === null || typeof value !== 'object') return value;
  if (seen.has(value)) return seen.get(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    seen.set(value, copy);
    for (const entry of value) copy.push(redactDeep(entry, redactString, seen));
    return copy;
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) return redactString(String(value));
  const copy: Record<string, unknown> = {};
  seen.set(value, copy);
  for (const [key, entry] of Object.entries(value)) {
    copy[key] = typeof entry === 'string' && isSensitiveField(key) ? MASK : redactDeep(entry, redactString, seen);
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
