import { CATALOGS, type Args, type Key } from './catalogs';
import type { Locale } from './locale';
import { render } from './render';

export function translate<K extends Key>(locale: Locale, key: K, ...args: Args<K>): string {
  return render(locale, CATALOGS[locale][key], args[0]);
}
