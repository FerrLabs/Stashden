import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  inject,
  input,
  output,
  resource,
  signal,
  viewChild,
} from '@angular/core';
import type { Node } from '../node';
import { PLATFORM } from '../platform';
import { Confirm } from '../shared/confirm';
import { DownloadCancelled } from '../transfer';
import type { Version } from '../version';
import { formatMoment, formatSize, t } from '../i18n/language';

@Component({
  selector: 'rx-versions-dialog',
  imports: [Confirm],
  templateUrl: './versions-dialog.html',
  styleUrl: './versions-dialog.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VersionsDialog {
  protected readonly t = t;
  private readonly platform = inject(PLATFORM);

  readonly node = input.required<Node>();
  readonly path = input.required<string>();
  readonly canRestore = input.required<boolean>();
  readonly dismissed = output<void>();
  readonly restored = output<void>();

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  protected readonly busy = signal(false);
  protected readonly failure = signal<string | null>(null);
  protected readonly announcement = signal<string | null>(null);
  protected readonly restoring = signal<Version | null>(null);

  protected readonly versions = resource({
    params: () => ({ path: this.path() }),
    loader: ({ params }) => this.platform.listVersions?.(params.path) ?? Promise.resolve([]),
  });

  protected readonly size = formatSize;
  protected readonly moment = formatMoment;

  constructor() {
    afterNextRender(() => this.dialog().nativeElement.showModal());
  }

  protected async download(version: Version): Promise<void> {
    const get = this.platform.downloadVersion;
    if (get === undefined) {
      return;
    }
    await this.attempt(async () => {
      const when = formatMoment(version.created_at);
      let saved: string | null;
      try {
        saved = await get(this.path(), version.id, this.node().name);
      } catch (cause: unknown) {
        if (cause instanceof DownloadCancelled) {
          return;
        }
        throw cause;
      }
      this.announcement.set(
        saved === null
          ? t('versions.announce.downloaded', { moment: when })
          : t('versions.announce.saved', { moment: when, destination: saved }),
      );
    });
  }

  protected async restore(version: Version): Promise<void> {
    this.restoring.set(null);
    const put = this.platform.restoreVersion;
    if (put === undefined) {
      return;
    }
    await this.attempt(async () => {
      await put(this.path(), version.id);
      this.announcement.set(
        t('versions.announce.restored', { moment: formatMoment(version.created_at) }),
      );
      this.versions.reload();
      this.restored.emit();
    });
  }

  protected dismiss(): void {
    this.dialog().nativeElement.close();
    this.dismissed.emit();
  }

  private async attempt(run: () => Promise<void>): Promise<void> {
    if (this.busy()) {
      return;
    }
    this.failure.set(null);
    this.announcement.set(null);
    this.busy.set(true);
    try {
      await run();
    } catch (cause: unknown) {
      this.failure.set(cause instanceof Error ? cause.message : String(cause));
    } finally {
      this.busy.set(false);
    }
  }
}
