import { type Signal, signal } from '@angular/core';
import type { Args, Key } from './catalogs';
import { dateIn, dayIn, momentIn, sizeIn } from './format';
import { type Locale, isLocale, negotiate } from './locale';
import { translate } from './translate';

const STORAGE_KEY = 'stashden.language';

function remembered(): Locale | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isLocale(stored) ? stored : null;
  } catch {
    return null;
  }
}

function initial(): Locale {
  return remembered() ?? negotiate(navigator.languages);
}

const active = signal<Locale>(initial());

document.documentElement.lang = active();

export const locale: Signal<Locale> = active.asReadonly();

export function chooseLocale(next: Locale): void {
  active.set(next);
  document.documentElement.lang = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    return;
  }
}

export function t<K extends Key>(key: K, ...args: Args<K>): string {
  return translate(active(), key, ...args);
}

export const formatSize = (bytes: number): string => sizeIn(active(), bytes);

export const formatDate = (iso: string): string => dateIn(active(), iso);

export const formatMoment = (iso: string): string => momentIn(active(), iso);

export const formatDay = (at: Date, withYear = true): string => dayIn(active(), at, withYear);
