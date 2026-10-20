import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { PLATFORM } from '../platform';
import type { Methods } from '../provider';
import type { Credentials } from './credentials';

@Component({
  selector: 'rx-login-form',
  imports: [ReactiveFormsModule, RouterLink],
  templateUrl: './login-form.html',
  styleUrl: './login-form.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LoginForm {
  private readonly platform = inject(PLATFORM);

  readonly busy = input(false);
  readonly methods = input<Methods | null>(null);
  readonly submitted = output<Credentials>();
  readonly providerChosen = output<string | undefined>();
  readonly serverChanged = output<string>();

  protected readonly asks = this.platform.server !== undefined;
  protected readonly opensResetElsewhere = this.platform.forgotPassword !== undefined;

  protected readonly form = inject(FormBuilder).nonNullable.group({
    server: [this.platform.server?.() ?? '', this.asks ? [Validators.required] : []],
    email: ['', [Validators.required, Validators.email]],
    password: ['', Validators.required],
  });

  protected readonly serverValid = () => !this.asks || this.form.controls.server.valid;

  protected submit(): void {
    if (this.form.invalid) {
      return;
    }
    const { server, ...credentials } = this.form.getRawValue();
    this.submitted.emit(this.asks ? { ...credentials, server } : credentials);
  }

  protected chooseProvider(): void {
    if (!this.serverValid()) {
      this.form.controls.server.markAsTouched();
      return;
    }
    this.providerChosen.emit(this.asks ? this.form.controls.server.value : undefined);
  }

  protected forgotPassword(): void {
    if (!this.serverValid()) {
      this.form.controls.server.markAsTouched();
      return;
    }
    void this.platform.forgotPassword?.(this.form.controls.server.value);
  }

  protected serverLeft(): void {
    if (this.serverValid()) {
      this.serverChanged.emit(this.form.controls.server.value);
    }
  }
}
