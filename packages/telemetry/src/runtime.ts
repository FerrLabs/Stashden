import { ErrorHandler } from '@angular/core';
import { type TransportItem, faro, getWebInstrumentations, initializeFaro } from '@grafana/faro-web-sdk';

export type TelemetryEnvironment = 'dev' | 'interne' | 'home' | 'prod';

export interface TelemetryOptions {
  appName: string;
  url: string;
  beforeSend: (item: TransportItem) => TransportItem | null;
}

export function environmentFor(hostname: string): TelemetryEnvironment {
  if (hostname.endsWith('.dev.ferrlabs')) return 'dev';
  if (hostname.endsWith('.ferrlabs')) return 'interne';
  if (hostname.endsWith('.home')) return 'home';
  return 'prod';
}

export function isOptedOut(): boolean {
  const { globalPrivacyControl, doNotTrack } = navigator as Navigator & { globalPrivacyControl?: boolean };
  return globalPrivacyControl === true || doNotTrack === '1';
}

export function isLocalHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/\.$/, '').replace(/^\[|\]$/g, '');
  return host === 'localhost' || host === '::1' || /^127(?:\.\d{1,3}){3}$/.test(host);
}

export function initTelemetry({ appName, url, beforeSend }: TelemetryOptions): void {
  if (url.length === 0) return;
  if (typeof window === 'undefined' || isLocalHost(window.location.hostname) || isOptedOut()) return;
  try {
    initializeFaro({
      url,
      app: { name: appName, environment: environmentFor(window.location.hostname) },
      instrumentations: getWebInstrumentations({ captureConsole: false }),
      beforeSend,
      sessionTracking: { enabled: false },
    });
  } catch {
    return;
  }
}

function toError(value: unknown): Error {
  if (value instanceof Error) return value;
  if (typeof value === 'object' && value !== null && 'message' in value && typeof value.message === 'string') {
    const error = new Error(value.message);
    if ('stack' in value && typeof value.stack === 'string') error.stack = value.stack;
    return error;
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
