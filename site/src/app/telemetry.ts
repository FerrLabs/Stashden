import { initTelemetry as startTelemetry } from '@ferrlabs/telemetry';

export { TelemetryErrorHandler } from '@ferrlabs/telemetry/angular';

export function initTelemetry(): void {
  if (STASHDEN_TELEMETRY_URL.length === 0) return;
  startTelemetry({ appName: 'stashden-site', collectorUrl: STASHDEN_TELEMETRY_URL });
}
