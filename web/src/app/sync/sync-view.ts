import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
} from '@angular/core';
import { Router } from '@angular/router';
import { linkTo } from '../folder';
import { PLATFORM } from '../platform';
import { describe, needsAttention, summarise, type SyncCommand, type Syncing } from './syncing';

@Component({
  selector: 'rx-sync-view',
  templateUrl: './sync-view.html',
  styleUrl: './sync-view.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SyncView {
  private readonly platform = inject(PLATFORM);

  protected readonly folders = signal<Syncing[]>([]);
  protected readonly failure = signal<string | null>(null);
  protected readonly busy = signal(false);

  protected readonly describe = describe;
  protected readonly summarise = summarise;
  protected readonly needsAttention = needsAttention;

  constructor() {
    if (this.platform.startSync === undefined) {
      void inject(Router).navigate(linkTo(''));
      return;
    }

    void this.follow(inject(DestroyRef));
  }

  private async follow(destroyRef: DestroyRef): Promise<void> {
    const unlisten = await this.platform.watchSync?.((syncing) => this.upsert(syncing));
    if (unlisten !== undefined) {
      if (destroyRef.destroyed) {
        unlisten();
      } else {
        destroyRef.onDestroy(unlisten);
      }
    }

    await this.refresh();
  }

  private upsert(syncing: Syncing): void {
    this.folders.update((all) =>
      all.some((one) => one.folder === syncing.folder)
        ? all.map((one) => (one.folder === syncing.folder ? syncing : one))
        : [...all, syncing],
    );
  }

  protected async choose(): Promise<void> {
    await this.run(async () => {
      const folder = await this.platform.pickFolder?.();
      if (folder !== null && folder !== undefined) {
        await this.platform.startSync?.(folder);
        await this.refresh();
      }
    });
  }

  protected async control(folder: string, command: SyncCommand): Promise<void> {
    await this.run(async () => {
      await this.platform.controlSync?.(folder, command);
      await this.refresh();
    });
  }

  private async refresh(): Promise<void> {
    this.folders.set((await this.platform.syncStatus?.()) ?? []);
  }

  private async run(action: () => Promise<void>): Promise<void> {
    this.failure.set(null);
    this.busy.set(true);
    try {
      await action();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
    } finally {
      this.busy.set(false);
    }
  }
}
