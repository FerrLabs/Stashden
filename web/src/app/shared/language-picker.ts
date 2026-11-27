import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { chooseLocale, locale, t } from '../i18n/language';
import { LANGUAGE_NAMES, LOCALES } from '../i18n/locale';

@Component({
  selector: 'rx-language-picker',
  templateUrl: './language-picker.html',
  styleUrl: './language-picker.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LanguagePicker {
  readonly inMenu = input(false);

  protected readonly t = t;
  protected readonly current = locale;
  protected readonly choose = chooseLocale;
  protected readonly options = LOCALES.map((code) => ({ code, name: LANGUAGE_NAMES[code] }));
}
