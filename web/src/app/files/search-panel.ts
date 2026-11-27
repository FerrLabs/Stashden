import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { linkTo, parentOf } from '../folder';
import type { Hit } from '../node';
import { PLATFORM } from '../platform';
import { formatSize, t } from '../i18n/language';

@Component({
  selector: 'rx-search-panel',
  templateUrl: './search-panel.html',
  styleUrl: './search-panel.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SearchPanel {
  protected readonly t = t;
  private readonly platform = inject(PLATFORM);
  private readonly field = viewChild.required<ElementRef<HTMLInputElement>>('field');

  readonly closed = output<void>();
  readonly chosen = output<string[]>();

  protected readonly term = signal('');
  protected readonly searched = signal<string | null>(null);
  protected readonly hits = signal<Hit[]>([]);
  protected readonly busy = signal(false);
  protected readonly failure = signal<string | null>(null);

  constructor() {
    afterNextRender(() => this.field().nativeElement.focus());
  }

  protected async search(): Promise<void> {
    const term = this.term().trim();
    const search = this.platform.search;
    if (term.length === 0 || search === undefined) {
      return;
    }
    this.busy.set(true);
    this.failure.set(null);
    try {
      this.hits.set(await search(term));
      this.searched.set(term);
    } catch (cause: unknown) {
      this.failure.set(
        t('search.failed', { reason: cause instanceof Error ? cause.message : String(cause) }),
      );
    } finally {
      this.busy.set(false);
    }
  }

  protected where(hit: Hit): string {
    const folder = parentOf(hit.path);
    return folder ? t('search.inFolder', { path: folder }) : t('search.inRoot');
  }

  protected describe(hit: Hit): string {
    return hit.kind === 'directory' ? t('common.folder') : formatSize(hit.size);
  }

  protected choose(hit: Hit): void {
    const folder = hit.kind === 'directory' ? hit.path : (parentOf(hit.path) ?? '');
    this.chosen.emit(linkTo(folder));
  }

  protected onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      this.closed.emit();
    }
  }
}
