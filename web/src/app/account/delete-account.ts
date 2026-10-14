import {
  ChangeDetectionStrategy,
  Component,
  computed,
  ElementRef,
  afterNextRender,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { Session } from '../account';
import { PLATFORM } from '../platform';

@Component({
  selector: 'rx-delete-account',
  templateUrl: './delete-account.html',
  styleUrl: './delete-account.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DeleteAccount {
  private readonly platform = inject(PLATFORM);
  private readonly session = inject(Session);

  protected readonly hasPassword = computed(() => this.session.account()?.has_password ?? true);
  protected readonly needsFreshSignIn = computed(
    () => !this.hasPassword() && !(this.session.account()?.fresh_sign_in ?? false),
  );

  readonly dismissed = output<void>();
  readonly deleted = output<void>();

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  protected readonly confirmation = signal('');
  protected readonly heir = signal('');
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

  protected async submit(): Promise<void> {
    const leave = this.platform.deleteOwnAccount;
    if (leave === undefined || this.busy()) {
      return;
    }

    this.failure.set(null);
    this.busy.set(true);
    try {
      const proof = this.hasPassword()
        ? { password: this.confirmation() }
        : { email: this.confirmation().trim() };
      await leave(proof, this.heir().trim() || null);
      this.dialog().nativeElement.close();
      this.deleted.emit();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
      await this.refresh();
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
