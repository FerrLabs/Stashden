import { ChangeDetectionStrategy, Component, output, signal } from '@angular/core';
import { AccountDialog } from './account-dialog';

@Component({
  selector: 'rx-delete-account',
  templateUrl: './delete-account.html',
  styleUrl: './delete-account.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DeleteAccount extends AccountDialog {
  readonly deleted = output<void>();

  protected readonly confirmation = signal('');
  protected readonly heir = signal('');

  protected async submit(): Promise<void> {
    const leave = this.platform.deleteOwnAccount;
    if (leave === undefined) {
      return;
    }

    const done = await this.attempt(() => {
      const proof = this.hasPassword()
        ? { password: this.confirmation() }
        : { email: this.confirmation().trim() };
      return leave(proof, this.heir().trim() || null);
    });
    if (done) {
      this.deleted.emit();
    }
  }
}
