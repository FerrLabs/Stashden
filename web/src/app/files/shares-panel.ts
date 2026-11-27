import {
  ChangeDetectionStrategy,
  Component,
  inject,
  input,
  output,
  resource,
  signal,
} from '@angular/core';
import { SHARED_WITH_ME, describeAccess, type Given } from '../grant';
import { PLATFORM } from '../platform';
import { describeLink, type Share } from '../share';
import { Confirm } from '../shared/confirm';
import { t } from '../i18n/language';

@Component({
  selector: 'rx-shares-panel',
  imports: [Confirm],
  templateUrl: './shares-panel.html',
  styleUrl: './shares-panel.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SharesPanel {
  protected readonly t = t;
  private readonly platform = inject(PLATFORM);

  readonly version = input(0);
  readonly closed = output<void>();

  protected readonly links = resource({
    params: () => ({ version: this.version() }),
    loader: () => this.platform.listShares?.() ?? Promise.resolve([]),
  });

  protected readonly canGrant = this.platform.listGrants !== undefined;
  protected readonly grants = resource({
    params: () => ({ version: this.version() }),
    loader: () => this.platform.listGrants?.() ?? Promise.resolve([]),
  });

  protected readonly doomed = signal<Share | null>(null);
  protected readonly withdrawing = signal<Given | null>(null);
  protected readonly failure = signal<string | null>(null);

  protected readonly shelf = SHARED_WITH_ME;
  protected readonly describe = describeLink;
  protected describeGiven(given: Given): string {
    const parts = [given.email, describeAccess(given.access)];
    if (given.in_trash) {
      parts.push(t('share.panel.inTrash'));
    }
    return parts.join(', ');
  }

  protected async revoke(share: Share): Promise<void> {
    this.doomed.set(null);
    const drop = this.platform.revokeShare;
    if (drop === undefined) {
      return;
    }

    this.failure.set(null);
    try {
      await drop(share.id);
      this.links.reload();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
    }
  }

  protected async withdraw(given: Given): Promise<void> {
    this.withdrawing.set(null);
    const drop = this.platform.withdrawGrant;
    if (drop === undefined) {
      return;
    }

    this.failure.set(null);
    try {
      await drop(given.id);
      this.grants.reload();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
    }
  }
}
