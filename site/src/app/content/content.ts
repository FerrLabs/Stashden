import en from './en.json' with { type: 'json' };
import fr from './fr.json' with { type: 'json' };
import type { Locale } from './locale';
import type { SiteContent } from './site-content';

export const CONTENT: Record<Locale, SiteContent> = { en, fr };
