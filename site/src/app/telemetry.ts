import { initTelemetry as startTelemetry, sanitizeItem } from '@stashden/telemetry';

export { TelemetryErrorHandler } from '@stashden/telemetry';

export function initTelemetry(): void {
  startTelemetry({ appName: 'stashden-site', url: STASHDEN_TELEMETRY_URL, beforeSend: sanitizeItem });
}
