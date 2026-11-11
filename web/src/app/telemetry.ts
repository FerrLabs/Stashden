import { type TransportItem } from '@grafana/faro-web-sdk';
import { initTelemetry as startTelemetry, redactText, sanitizeItem as sanitizeTransportItem } from '@stashden/telemetry';

export { TelemetryErrorHandler } from '@stashden/telemetry';

const capabilityPaths: readonly (readonly [RegExp, string])[] = [
  [/(\/v1\/public\/)[^/?#\s"'<>]+(\/(?:entries|content|archive)\/)[^?#\s"'<>]+/g, '$1:token$2:path'],
  [/(\/v1\/(?:files|folders|archive|versions|thumbnails)\/)[^?#\s"'<>]+/g, '$1:path'],
  [/(\/v1\/version\/)[^/?#\s"'<>]+(?:\/[^?#\s"'<>]*)?/g, '$1:id/:path'],
  [/(\/s\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/reset-password\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/v1\/public\/)[^/?#\s"'<>]+/g, '$1:token'],
];
const FILES_ROUTE = /^(\/files\/).+$/;

function redactUrl(text: string): string {
  const redacted = capabilityPaths.reduce((value, [pattern, replacement]) => value.replace(pattern, replacement), text);
  return redactText(redacted);
}

function redactPageUrl(url: string): string {
  const hashAt = url.indexOf('#');
  if (hashAt < 0) return redactUrl(url);
  const route = url.slice(hashAt + 1).split(/[?#]/, 1)[0] ?? '';
  return `${redactUrl(url.slice(0, hashAt))}#${redactUrl(route.replace(FILES_ROUTE, '$1:path'))}`;
}

function sanitizeItem(item: TransportItem): TransportItem | null {
  return sanitizeTransportItem(item, { redactString: redactUrl, redactPageUrl });
}

export function initTelemetry(): void {
  startTelemetry({ appName: 'stashden-web', url: STASHDEN_TELEMETRY_URL, beforeSend: sanitizeItem });
}
