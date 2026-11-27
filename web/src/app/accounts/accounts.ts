import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  resource,
  signal,
} from '@angular/core';
import { Router } from '@angular/router';
import { Session, describeRole, type Role } from '../account';
import { linkTo } from '../folder';
import { PLATFORM } from '../platform';
import { Confirm } from '../shared/confirm';
import { AccountPassword } from './account-password';
import { DeleteAccountDialog } from './delete-account';
import { NewAccountDialog } from './new-account';
import { QuotaDialog } from './quota';
import { ROLES, describeUsage, type ManagedAccount } from './managed';
import { t } from '../i18n/language';

@Component({
  selector: 'rx-accounts',
  imports: [AccountPassword, Confirm, DeleteAccountDialog, NewAccountDialog, QuotaDialog],
  templateUrl: './accounts.html',
  styleUrl: './accounts.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Accounts {
  protected readonly t = t;
  private readonly platform = inject(PLATFORM);
  private readonly router = inject(Router);
  private readonly session = inject(Session);

  protected readonly roles = ROLES;
  protected readonly describe = describeRole;
  protected readonly usage = describeUsage;
  protected readonly me = computed(() => this.session.account()?.id ?? null);

  private readonly version = signal(0);
  protected readonly accounts = resource({
    params: () => (this.session.isAdmin() ? { version: this.version() } : undefined),
    loader: () => this.platform.listAccounts?.() ?? Promise.resolve([]),
  });

  protected readonly failure = signal<string | null>(null);
  protected readonly announcement = signal<string | null>(null);
  protected readonly adding = signal(false);
  protected readonly disabling = signal<ManagedAccount | null>(null);
  protected readonly quota = signal<ManagedAccount | null>(null);
  protected readonly deleting = signal<ManagedAccount | null>(null);
  protected readonly canDelete = this.platform.deleteAccount !== undefined;
  protected readonly resetting = signal<ManagedAccount | null>(null);
  protected readonly password = signal<{ heading: string; secret: string } | null>(null);

  constructor() {
    effect(() => {
      const account = this.session.account();
      if (account !== null && account.role !== 'admin') {
        void this.router.navigate(linkTo(''));
      }
    });
  }

  protected async setRole(account: ManagedAccount, event: Event): Promise<void> {
    const select = event.target as HTMLSelectElement;
    const role = select.value as Role;
    const changed = await this.attempt(
      (reason) => t('accounts.failed.change', { email: account.email, reason }),
      async () => {
        await this.platform.setRole?.(account.id, role);
        this.announcement.set(
          t('accounts.announce.role', {
            email: account.email,
            role: describeRole(role).toLowerCase(),
          }),
        );
      },
    );
    if (!changed) {
      select.value = account.role;
    }
  }

  protected async setDisabled(account: ManagedAccount, disabled: boolean): Promise<void> {
    this.disabling.set(null);
    await this.attempt(
      (reason) => t('accounts.failed.change', { email: account.email, reason }),
      async () => {
        await this.platform.setDisabled?.(account.id, disabled);
        this.announcement.set(
          disabled
            ? t('accounts.announce.disabled', { email: account.email })
            : t('accounts.announce.enabled', { email: account.email }),
        );
      },
    );
  }

  protected async remove(account: ManagedAccount, handOverTo: string | null): Promise<void> {
    this.deleting.set(null);
    await this.attempt(
      (reason) => t('accounts.failed.delete', { email: account.email, reason }),
      async () => {
        await this.platform.deleteAccount?.(account.id, handOverTo);
        this.announcement.set(t('accounts.announce.deleted', { email: account.email }));
      },
    );
  }

  protected async unlock(account: ManagedAccount): Promise<void> {
    await this.attempt(
      (reason) => t('accounts.failed.unlock', { email: account.email, reason }),
      async () => {
        await this.platform.unlockAccount?.(account.id);
        this.announcement.set(t('accounts.announce.unlocked', { email: account.email }));
      },
    );
  }

  protected async setQuota(account: ManagedAccount, bytes: number): Promise<void> {
    this.quota.set(null);
    await this.attempt(
      (reason) => t('accounts.failed.quota', { email: account.email, reason }),
      async () => {
        await this.platform.setQuota?.(account.id, bytes);
        this.announcement.set(t('accounts.announce.quota', { email: account.email }));
      },
    );
  }

  protected async resetPassword(account: ManagedAccount, secret: string): Promise<void> {
    this.resetting.set(null);
    await this.attempt(
      (reason) => t('accounts.failed.reset', { email: account.email, reason }),
      async () => {
        await this.platform.resetPassword?.(account.id, secret);
        this.password.set({
          heading: t('accounts.passwordFor.reset', { email: account.email }),
          secret,
        });
      },
    );
  }

  protected async create(account: {
    email: string;
    display_name: string;
    password: string;
    role: Role;
  }): Promise<void> {
    this.adding.set(false);
    await this.attempt(
      (reason) => t('accounts.failed.create', { email: account.email, reason }),
      async () => {
        await this.platform.createAccount?.(account);
        this.password.set({
          heading: t('accounts.passwordFor.created', { email: account.email }),
          secret: account.password,
        });
      },
    );
  }

  private async attempt(
    failed: (reason: string) => string,
    run: () => Promise<void>,
  ): Promise<boolean> {
    this.failure.set(null);
    this.announcement.set(null);
    try {
      await run();
      this.version.update((count) => count + 1);
      return true;
    } catch (cause: unknown) {
      this.failure.set(failed(cause instanceof Error ? cause.message : String(cause)));
      return false;
    }
  }
}
