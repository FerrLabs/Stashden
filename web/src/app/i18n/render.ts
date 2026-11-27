import type { Locale } from './locale';
import type { Message, Value } from './message';

const PLACEHOLDER = /\{(\w+)\}/g;

const numbers = new Map<Locale, Intl.NumberFormat>();
const plurals = new Map<Locale, Intl.PluralRules>();

function numberFormat(locale: Locale): Intl.NumberFormat {
  const known = numbers.get(locale);
  if (known !== undefined) {
    return known;
  }
  const created = new Intl.NumberFormat(locale);
  numbers.set(locale, created);
  return created;
}

function pluralRules(locale: Locale): Intl.PluralRules {
  const known = plurals.get(locale);
  if (known !== undefined) {
    return known;
  }
  const created = new Intl.PluralRules(locale);
  plurals.set(locale, created);
  return created;
}

function pick(locale: Locale, message: Message, values: Readonly<Record<string, Value>>): string {
  if (typeof message === 'string') {
    return message;
  }
  const count = values['count'];
  if (typeof count !== 'number') {
    throw new TypeError('a plural message needs a numeric count');
  }
  return pluralRules(locale).select(count) === 'one' ? message.one : message.other;
}

export function render(
  locale: Locale,
  message: Message,
  values: Readonly<Record<string, Value>> = {},
): string {
  return pick(locale, message, values).replace(PLACEHOLDER, (_, name: string) => {
    const value = values[name];
    if (value === undefined) {
      throw new TypeError(`the message needs a value for {${name}}`);
    }
    return typeof value === 'number' ? numberFormat(locale).format(value) : value;
  });
}
