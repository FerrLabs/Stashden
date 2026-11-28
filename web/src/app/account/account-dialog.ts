import {
  afterNextRender,
  computed,
  Directive,
  ElementRef,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { Session } from '../account';
import { t } from '../i18n/language';
import { PLATFORM } from '../platform';

@Directive()
export abstract class AccountDialog {
  protected readonly t = t;
  protected readonly platform = inject(PLATFORM);
  private readonly session = inject(Session);

  protected readonly hasPassword = computed(() => this.session.account()?.has_password ?? true);
  protected readonly needsFreshSignIn = computed(
    () => !this.hasPassword() && !(this.session.account()?.fresh_sign_in ?? false),
  );

  readonly dismissed = output<void>();

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  protected readonly busy = signal(false);
  protected readonly failure = signal<string | null>(null);

  constructor() {
    afterNextRender(() => this.dialog().nativeElement.showModal());
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    try {
      await this.session.load();
    } catch {
      return;
    }
  }

  protected async attempt(action: () => Promise<void>): Promise<boolean> {
    if (this.busy()) {
      return false;
    }

    this.failure.set(null);
    this.busy.set(true);
    try {
      await action();
      this.dialog().nativeElement.close();
      return true;
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
      await this.refresh();
      return false;
    } finally {
      this.busy.set(false);
    }
  }

  protected async signInAgain(): Promise<void> {
    this.failure.set(null);
    this.busy.set(true);
    try {
      await this.platform.loginWithProvider();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
      this.busy.set(false);
    }
  }

  protected dismiss(): void {
    this.dialog().nativeElement.close();
    this.dismissed.emit();
  }
}
