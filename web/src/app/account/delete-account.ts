import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { PLATFORM } from '../platform';

@Component({
  selector: 'rx-delete-account',
  templateUrl: './delete-account.html',
  styleUrl: './delete-account.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DeleteAccount {
  private readonly platform = inject(PLATFORM);

  readonly dismissed = output<void>();
  readonly deleted = output<void>();

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  protected readonly password = signal('');
  protected readonly heir = signal('');
  protected readonly busy = signal(false);
  protected readonly failure = signal<string | null>(null);

  constructor() {
    afterNextRender(() => this.dialog().nativeElement.showModal());
  }

  protected async submit(): Promise<void> {
    const leave = this.platform.deleteOwnAccount;
    if (leave === undefined || this.busy()) {
      return;
    }

    this.failure.set(null);
    this.busy.set(true);
    try {
      await leave(this.password(), this.heir().trim() || null);
      this.dialog().nativeElement.close();
      this.deleted.emit();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
    } finally {
      this.busy.set(false);
    }
  }

  protected dismiss(): void {
    this.dialog().nativeElement.close();
    this.dismissed.emit();
  }
}
