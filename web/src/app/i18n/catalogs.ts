import { en } from './en';
import { fr } from './fr';
import type { Locale } from './locale';
import type { Mirror, Names, Value } from './message';

export type Key = keyof typeof en;

type English = typeof en;

export type Args<K extends Key> = [Names<English[K]>] extends [never]
  ? []
  : [values: { readonly [Name in Names<English[K]>]: Value }];

export const CATALOGS: Readonly<Record<Locale, Mirror<English>>> = { en, fr };
