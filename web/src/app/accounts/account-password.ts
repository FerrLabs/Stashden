import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { generatePassword } from './managed';
import { t } from '../i18n/language';

@Component({
  selector: 'rx-account-password',
  templateUrl: './account-password.html',
  styleUrl: './account-password.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccountPassword {
  protected readonly t = t;
  readonly heading = input.required<string>();
  readonly action = input<string>();
  readonly secret = input<string | null>(null);
  readonly submitted = output<string>();
  readonly dismissed = output<void>();

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  protected readonly draft = signal(generatePassword());
  protected readonly copied = signal(false);
  protected readonly failure = signal<string | null>(null);

  constructor() {
    afterNextRender(() => this.dialog().nativeElement.showModal());
  }

  protected submit(): void {
    if (this.draft().length === 0) {
      return;
    }
    this.dialog().nativeElement.close();
    this.submitted.emit(this.draft());
  }

  protected async copy(): Promise<void> {
    const secret = this.secret() ?? this.draft();
    try {
      await navigator.clipboard.writeText(secret);
      this.copied.set(true);
    } catch {
      this.copied.set(false);
      this.failure.set(t('common.copyPasswordFailed'));
    }
  }

  protected dismiss(): void {
    this.dialog().nativeElement.close();
    this.dismissed.emit();
  }
}
