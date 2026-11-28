import { ChangeDetectionStrategy, Component, output, signal } from '@angular/core';
import { AccountDialog } from './account-dialog';

@Component({
  selector: 'rx-change-password',
  templateUrl: './change-password.html',
  styleUrl: './change-password.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChangePassword extends AccountDialog {
  readonly changed = output<void>();

  protected readonly current = signal('');
  protected readonly replacement = signal('');

  protected async submit(): Promise<void> {
    const change = this.platform.changePassword;
    if (change === undefined) {
      return;
    }

    const done = await this.attempt(() =>
      change(this.hasPassword() ? this.current() : null, this.replacement()),
    );
    if (done) {
      this.changed.emit();
    }
  }
}
