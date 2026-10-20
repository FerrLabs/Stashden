import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  signal,
} from "@angular/core";
import { toSignal } from "@angular/core/rxjs-interop";
import { ActivatedRoute, RouterLink } from "@angular/router";
import { ResetRefused, Resets } from "./resets";

type Stage = "asking" | "mailed" | "choosing" | "changed";

@Component({
  selector: "rx-password-reset",
  imports: [RouterLink],
  templateUrl: "./password-reset.html",
  styleUrl: "./password-reset.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PasswordReset {
  private readonly resets = inject(Resets);

  private readonly params = toSignal(inject(ActivatedRoute).params, {
    initialValue: {},
  });
  private readonly token = computed(
    () => (this.params() as { token?: string }).token ?? null,
  );

  private readonly done = signal<"mailed" | "changed" | null>(null);
  protected readonly stage = computed<Stage>(
    () => this.done() ?? (this.token() === null ? "asking" : "choosing"),
  );

  protected readonly email = signal("");
  protected readonly password = signal("");
  protected readonly busy = signal(false);
  protected readonly failure = signal<string | null>(null);

  protected async ask(): Promise<void> {
    await this.attempt(async () => {
      await this.resets.request(this.email().trim());
      this.done.set("mailed");
    });
  }

  protected async choose(): Promise<void> {
    const token = this.token();
    if (token === null) {
      return;
    }
    await this.attempt(async () => {
      await this.resets.redeem(token, this.password());
      this.done.set("changed");
    });
  }

  private async attempt(action: () => Promise<void>): Promise<void> {
    if (this.busy()) {
      return;
    }
    this.failure.set(null);
    this.busy.set(true);
    try {
      await action();
    } catch (cause: unknown) {
      this.failure.set(
        cause instanceof ResetRefused
          ? cause.message
          : "The server did not answer, try again later.",
      );
    } finally {
      this.busy.set(false);
    }
  }
}
