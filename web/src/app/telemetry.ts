import { initTelemetry as startTelemetry } from '@ferrlabs/telemetry';

export { TelemetryErrorHandler } from '@ferrlabs/telemetry/angular';

const CAPABILITY_PATHS: readonly (readonly [RegExp, string])[] = [
  [/(\/v1\/public\/)[^/?#\s"'<>]+(\/(?:entries|content|archive)\/)[^?#\s"'<>]+/g, '$1:token$2:path'],
  [/(\/v1\/(?:files|folders|archive|versions|thumbnails)\/)[^?#\s"'<>]+/g, '$1:path'],
  [/(\/v1\/version\/)[^/?#\s"'<>]+(?:\/[^?#\s"'<>]*)?/g, '$1:id/:path'],
  [/(\/s\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/reset-password\/)[^/?#\s"'<>]+/g, '$1:token'],
  [/(\/v1\/public\/)[^/?#\s"'<>]+/g, '$1:token'],
];

export function initTelemetry(): void {
  startTelemetry({ appName: 'stashden-web', collectorUrl: STASHDEN_TELEMETRY_URL, enabled: STASHDEN_TELEMETRY_URL.length > 0, capabilityPaths: CAPABILITY_PATHS });
}
