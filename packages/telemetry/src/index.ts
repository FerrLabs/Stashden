export { redactText } from './redact';
export { redactDeep, sanitizeItem } from './walk';
export type { SanitizeOptions, StringRedactor } from './walk';
export { TelemetryErrorHandler, environmentFor, initTelemetry, isLocalHost, isOptedOut } from './runtime';
export type { TelemetryEnvironment, TelemetryOptions } from './runtime';
