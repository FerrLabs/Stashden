import { Injectable, computed, inject, signal } from '@angular/core';
import { t } from './i18n/language';
import { PLATFORM } from './platform';

export type Role = 'admin' | 'member' | 'reader';

export function describeRole(role: Role): string {
  switch (role) {
    case 'admin':
      return t('role.admin');
    case 'member':
      return t('role.member');
    case 'reader':
      return t('role.reader');
  }
}

export type Account = {
  id: string;
  email: string;
  display_name: string;
  role: Role;
  has_password: boolean;
  fresh_sign_in: boolean;
};

@Injectable({ providedIn: 'root' })
export class Session {
  private readonly platform = inject(PLATFORM);

  readonly account = signal<Account | null>(null);

  readonly canWrite = computed(() => {
    const role = this.account()?.role;
    return role === 'admin' || role === 'member';
  });

  readonly isReader = computed(() => this.account()?.role === 'reader');

  readonly isAdmin = computed(() => this.account()?.role === 'admin');

  async load(): Promise<void> {
    this.account.set(await this.platform.account());
  }

  forget(): void {
    this.account.set(null);
  }
}
