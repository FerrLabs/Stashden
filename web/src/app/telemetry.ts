import { ErrorHandler } from '@angular/core';
import { type TransportItem, faro, getWebInstrumentations, initializeFaro } from '@grafana/faro-web-sdk';

const collectorUrl = 'https://rum.ferrlabs.com/collect';
const appName = 'roxycloud-web';
const localHostnames = ['localhost', '127.0.0.1'];

export type TelemetryEnvironment = 'dev' | 'interne' | 'home' | 'prod';

export function environmentFor(hostname: string): TelemetryEnvironment {
  if (hostname.endsWith('.dev.ferrlabs')) return 'dev';
  if (hostname.endsWith('.ferrlabs')) return 'interne';
  if (hostname.endsWith('.home')) return 'home';
  return 'prod';
}

const capabilityPaths: readonly (readonly [RegExp, string])[] = [
  [/(\/s\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/reset-password\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/v1\/public\/)[^/?#\s"'<>]+/g, '$1:token'],
];
const FILES_ROUTE = /^(\/files\/).+$/;

const MAX_LENGTH = 2048;
const MAX_INPUT_LENGTH = 16384;
const MAX_WORD_LENGTH = 512;
const MAX_PASSES = 4;
const MASK = '*';
const MARKERS = /[?#=@]/;
const VALUE = `(?:"[^"]*"?|'[^']*'?|[^\\s"'<>]+)`;
const PARAM_KEYS = `[\\w-]{0,64}(?:token|secret|passw(?:or)?d|pwd|key|session(?:id)?|auth\\w{0,32}|sig(?:nature)?|code|state)`;
const COLON_KEYS = `[\\w-]{0,64}(?:token|secret|passw(?:or)?d|pwd|key|session(?:id)?|auth\\w{0,32}|sig(?:nature)?)`;
const JWT = /\beyJ[\w-]*\.[\w-]+\.[\w-]*/g;
const BEARER = /\b(bearer)(?:\s+bearer)*\s+(?:"[^"]*"?|'[^']*'?|[^\s"',;]{8,})/gi;
const BASIC = /\b([Bb][Aa][Ss][Ii][Cc])\s+(?=[A-Za-z0-9+/]*[0-9A-Z+/])[A-Za-z0-9+/]{8,}={0,2}(?![\w+/])/g;
const EMAIL = /[\w.+-]{1,64}@[\w-]{1,63}(?:\.[\w-]{1,63})+/g;
const SENSITIVE_COLON = new RegExp(`(("?)${COLON_KEYS}\\2\\s*:\\s*)(?!(?:bearer|basic)\\s)(?:"[^"]*"?|'[^']*'?|[^\\s,;"'<>{}\\]]+)`, 'gi');
const SENSITIVE_PARAM = new RegExp(`(${PARAM_KEYS}=)${VALUE}`, 'gi');
const USERINFO = /(\/\/)[^\s/?#]*@/g;
const HOST_TAIL = /^((?:[a-z][\w+.-]*:)?(?:\/\/)?(?:[\w-]+(?:\.[\w-]+)+|localhost|[\w-]+(?=:\d))(?::\d+)?)[?#].*$/i;
const PATH_TAIL = /[?#].*$/;
const PARAM_VALUE = /([?#&;][^\s=&#?;"'<>]+=)[^\s&#;"'<>]+/g;

function redactWord(word: string): string {
  if (!MARKERS.test(word)) return word;
  if (word.length > MAX_WORD_LENGTH) return MASK;
  const scrubbed = word.replace(USERINFO, '$1').replace(HOST_TAIL, '$1');
  const cut = scrubbed.includes('/') ? scrubbed.replace(PATH_TAIL, '') : scrubbed;
  return cut.replace(PARAM_VALUE, `$1${MASK}`);
}

function redactPass(text: string): string {
  return text
    .replace(JWT, MASK)
    .replace(BEARER, `$1 ${MASK}`)
    .replace(BASIC, `$1 ${MASK}`)
    .replace(EMAIL, MASK)
    .replace(SENSITIVE_COLON, `$1${MASK}`)
    .replace(SENSITIVE_PARAM, `$1${MASK}`)
    .replace(/\S+/g, redactWord);
}

function dropPartialWord(text: string): string {
  const cut = text.search(/\s\S*$/);
  return cut === -1 ? MASK : text.slice(0, cut);
}

export function redactText(value: string): string {
  let text = value.length > MAX_INPUT_LENGTH ? dropPartialWord(value.slice(0, MAX_INPUT_LENGTH)) : value;
  for (let pass = 0; pass < MAX_PASSES; pass++) {
    const next = redactPass(text);
    if (next === text) break;
    text = next;
  }
  return text.length > MAX_LENGTH ? text.slice(0, MAX_LENGTH) : text;
}

function redactUrl(text: string): string {
  const redacted = capabilityPaths.reduce((value, [pattern, replacement]) => value.replace(pattern, replacement), text);
  return redactText(redacted);
}

export function redactPageUrl(url: string): string {
  const hashAt = url.indexOf('#');
  if (hashAt < 0) return redactUrl(url);
  const route = url.slice(hashAt + 1).split(/[?#]/, 1)[0] ?? '';
  return `${redactUrl(url.slice(0, hashAt))}#${redactUrl(route.replace(FILES_ROUTE, '$1:path'))}`;
}

function redactDeep(value: unknown, seen = new WeakMap<object, unknown>()): unknown {
  if (typeof value === 'string') return redactUrl(value);
  if (value === null || typeof value !== 'object') return value;
  if (seen.has(value)) return seen.get(value);
  if (Array.isArray(value)) {
    const copy: unknown[] = [];
    seen.set(value, copy);
    for (const entry of value) copy.push(redactDeep(entry, seen));
    return copy;
  }
  if (Object.getPrototypeOf(value) !== Object.prototype) return value;
  const copy: Record<string, unknown> = {};
  seen.set(value, copy);
  for (const [key, entry] of Object.entries(value)) copy[key] = redactDeep(entry, seen);
  return copy;
}

export function sanitizeItem(item: TransportItem): TransportItem | null {
  try {
    const meta = redactDeep(item.meta) as TransportItem['meta'];
    const pageUrl = item.meta.page?.url;
    if (pageUrl !== undefined) meta.page = { ...meta.page, url: redactPageUrl(pageUrl) };
    return { ...item, meta, payload: redactDeep(item.payload) as TransportItem['payload'] };
  } catch {
    return null;
  }
}

export function isOptedOut(): boolean {
  const { globalPrivacyControl, doNotTrack } = navigator as Navigator & { globalPrivacyControl?: boolean };
  return globalPrivacyControl === true || doNotTrack === '1';
}

export function initTelemetry(): void {
  if (typeof window === 'undefined' || localHostnames.includes(window.location.hostname) || isOptedOut()) return;
  initializeFaro({
    url: collectorUrl,
    app: { name: appName, environment: environmentFor(window.location.hostname) },
    instrumentations: getWebInstrumentations({ captureConsole: false }),
    beforeSend: sanitizeItem,
    sessionTracking: { enabled: false },
  });
}

function toError(value: unknown): Error {
  if (value instanceof Error) return value;
  if (typeof value === 'object' && value !== null && 'message' in value && typeof value.message === 'string') {
    return new Error(value.message);
  }
  try {
    return new Error(String(value));
  } catch {
    return new Error('Non-Error value');
  }
}

function reportError(value: unknown): void {
  try {
    faro.api?.pushError(toError(value));
  } catch {
    return;
  }
}

export class TelemetryErrorHandler extends ErrorHandler {
  override handleError(error: unknown): void {
    reportError(error);
    super.handleError(error);
  }
}
