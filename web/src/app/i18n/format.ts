import type { Locale } from './locale';

const UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const;

const NO_BREAK_SPACE = ' ';

function unitLabel(locale: Locale, unit: (typeof UNITS)[number]): string {
  const parts = new Intl.NumberFormat(locale, {
    style: 'unit',
    unit,
    unitDisplay: 'narrow',
  }).formatToParts(1);
  return parts.find((part) => part.type === 'unit')?.value ?? '';
}

export function sizeIn(locale: Locale, bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 10 ? 0 : 1;
  const number = new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value);
  return `${number}${NO_BREAK_SPACE}${unitLabel(locale, UNITS[unit] ?? 'byte')}`;
}

export function dayIn(locale: Locale, at: Date, withYear = true): string {
  return at.toLocaleDateString(locale, {
    day: 'numeric',
    month: 'short',
    ...(withYear ? { year: 'numeric' } : {}),
  });
}

export function dateIn(locale: Locale, iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? '' : dayIn(locale, at);
}

export function momentIn(locale: Locale, iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime())
    ? ''
    : at.toLocaleString(locale, {
        day: 'numeric',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      });
}
