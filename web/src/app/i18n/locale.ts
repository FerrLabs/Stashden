export const LOCALES = ['en', 'fr'] as const;

export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = 'en';

export const LANGUAGE_NAMES: Readonly<Record<Locale, string>> = {
  en: 'English',
  fr: 'Français',
};

export function isLocale(value: unknown): value is Locale {
  return LOCALES.some((locale) => locale === value);
}

export function negotiate(languages: readonly string[]): Locale {
  for (const language of languages) {
    const primary = language.split('-')[0]?.toLowerCase();
    if (isLocale(primary)) {
      return primary;
    }
  }
  return DEFAULT_LOCALE;
}
