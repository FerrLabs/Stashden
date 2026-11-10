import { type ApplicationConfig, ErrorHandler, provideZonelessChangeDetection } from '@angular/core';
import { provideRouter, withHashLocation } from '@angular/router';
import { routes } from './app.routes';
import { providePlatform } from './platform';
import { TelemetryErrorHandler } from './telemetry';

export const appConfig: ApplicationConfig = {
  providers: [
    { provide: ErrorHandler, useClass: TelemetryErrorHandler },
    provideZonelessChangeDetection(),
    provideRouter(routes, withHashLocation()),
    providePlatform(),
  ],
};
