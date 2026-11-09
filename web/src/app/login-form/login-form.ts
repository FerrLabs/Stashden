import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { RouterLink } from '@angular/router';
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
  readonly busy = input(false);
  readonly methods = input<Methods | null>(null);
  readonly submitted = output<Credentials>();
  readonly providerChosen = output<void>();

  protected readonly form = inject(FormBuilder).nonNullable.group({
    email: ['', [Validators.required, Validators.email]],
    password: ['', Validators.required],
  });

  protected submit(): void {
    if (this.form.invalid) {
      return;
    }
    this.submitted.emit(this.form.getRawValue());
  }

  protected chooseProvider(): void {
    this.providerChosen.emit();
  }
}
